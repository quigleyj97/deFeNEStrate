//! Emulator for the MOS 6502
//!
//! This does not include support for Binary Coded Decimal, which was omitted
//! on the 2A03 variant used on the NES and Famicom. Support for BCD may be
//! added later.

use std::fmt;
use std::num::Wrapping;

use crate::devices::bus::Bus;
use crate::utils::decode;
use crate::utils::structs::cpu::*;

fn bytes_to_addr(lo: u8, hi: u8) -> u16 {
    (u16::from(lo) << 8) + u16::from(hi)
}

pub struct Cpu6502 {
    /// A struct holding the CPU emulator's state
    pub state: CpuState,

    /// The CPU memory bus.
    ///
    /// # Note
    ///
    /// The bus may require additional setup, such as mapping devices onto it.
    /// Make sure that you do this before starting emulation, as the first thing
    /// the CPU does on powerup is read the reset vector.
    pub bus: Bus,

    /// The number of cycles to wait before executing the next instruction.
    ///
    /// # Note
    ///
    /// On the 6502, most instructions took longer than 1 clock cycle. Some
    /// took quite a few more, as the instruction had to read off operands
    /// from memory. This is a counter to simulate that- if not zero,
    /// `clock` will simply decrement this and continue.
    cycles: u8,

    /// Whether an interrupt is pending
    interrupt_pending: bool,
    /// Whether that interrupt was generated by an NMI (false) or IRQ (true)
    maskable_interrupt: bool,
}

impl Cpu6502 {
    pub fn tick(&mut self) -> bool {
        if self.cycles > 0 {
            self.state.tot_cycles += 1;
            self.cycles -= 1;
            return false;
        }
        true
    }

    pub fn exec(&mut self) {
        self.run_interrupt();
        self.load_opcode();
        self.decode_opcode(self.state.instruction);
        self.state.addr = self.get_addr(self.state.instruction);
        self.exec_instr();
    }

    pub fn debug(&mut self) -> String {
        let old_pc = self.state.pc;
        self.run_interrupt();
        self.load_opcode();
        self.decode_opcode(self.state.instruction);
        self.state.addr = self.get_addr(self.state.instruction);
        let new_pc = self.state.pc;
        self.state.pc = old_pc;
        let debug_str = format!("{}", self);
        self.state.pc = new_pc;
        self.exec_instr();
        debug_str
    }

    pub fn reset(&mut self) {
        self.state.stack -= 3;
        self.state.status |= Status::IRQ_DISABLE;
        let lo = self.read_bus(0xFFFC);
        let hi = self.read_bus(0xFFFD);
        self.state.pc = bytes_to_addr(hi, lo);
    }

    pub fn set_flag(&mut self, flag: Status) {
        self.state.status |= flag;
    }

    pub fn clear_flag(&mut self, flag: Status) {
        self.state.status &= !flag;
    }

    pub fn jmp(&mut self, addr: u16) {
        self.state.pc = addr;
    }

    pub fn trigger_nmi(&mut self) {
        self.interrupt_pending = true;
        self.maskable_interrupt = false;
    }

    pub fn trigger_irq(&mut self) {
        if self.state.status.contains(Status::IRQ_DISABLE) {
            return; // interrupt ignored
        }
        self.interrupt_pending = true;
        self.maskable_interrupt = true;
    }

    fn load_opcode(&mut self) {
        let mut bus = self.bus;
        let opcode = bus.read(self.state.pc);
        let operand1 = bus.read((Wrapping(self.state.pc) + Wrapping(1)).0);
        let operand2 = bus.read((Wrapping(self.state.pc) + Wrapping(2)).0);
        self.state.instruction =
            u32::from(opcode) + (u32::from(operand1) << 8) + (u32::from(operand2) << 16)
    }

    fn adv_pc(&mut self, inc: u16) {
        self.state.pc = (Wrapping(self.state.pc) + Wrapping(inc)).0;
    }

    /// Decodes an instruction into it's opcode and operand.
    ///
    /// # Notes
    ///
    /// Uses an algorithm described here: http://nparker.llx.com/a2/opcodes.html
    ///
    /// This may have errors or omissions for the NES 2A03, as that CPU's
    /// undocumented opcodes may be different in important ways.
    fn decode_opcode(&mut self, instruction: u32) {
        let ops = instruction.to_le_bytes();

        let (addr_mode, instr) = decode::decode_instruction(ops[0]);
        self.state.addr_mode = addr_mode;
        self.state.instr = instr;
    }

    /// Gets the address of the operand to read from.
    ///
    /// # Notes
    ///
    /// This sets the `cycles` to the average whole number of cycles any
    /// instruction with this addressing mode will have. Other instructions may
    /// need to add or subtract to compensate, refer to the 6502 datasheet for
    /// details:
    ///
    /// http://archive.6502.org/datasheets/mos_6501-6505_mpu_preliminary_aug_1975.pdf
    ///
    /// A note on the so-called "oops" cycle: The "oops" cycle occurs when an
    /// index instruction crosses a page boundary, as the CPU reads off the high
    /// byte first without checking for a carry-out. Some instructions (like all
    /// the store instructions) have some special-cased behavior that the 6502
    /// datasheet details. These depend on the instruction being executed, but
    /// this function is the best place to
    fn get_addr(&mut self, instruction: u32) -> u16 {
        let CpuState {
            x,
            y,
            addr_mode,
            pc,
            ..
        } = self.state;
        let ops = instruction.to_le_bytes();
        // +2 cycles for instr + byte1 of op readout, minimum
        self.cycles += 2;
        // Advance the PC at _least_ 1 byte
        self.adv_pc(1);

        match addr_mode {
            AddressingMode::Abs => {
                self.cycles += 1;
                self.adv_pc(2);
                bytes_to_addr(ops[2], ops[1])
            }
            AddressingMode::AbsInd => {
                let addr_lo = bytes_to_addr(ops[2], ops[1]);
                let addr_hi = bytes_to_addr(ops[2], ops[1].wrapping_add(1));
                self.adv_pc(2);
                let hi = self.read_bus(addr_hi);
                let lo = self.read_bus(addr_lo);
                bytes_to_addr(hi, lo)
            }
            AddressingMode::AbsX => {
                let addr = bytes_to_addr(ops[2], ops[1]) + u16::from(x);
                self.adv_pc(2);
                if (u16::from(x) + u16::from(ops[1])) & 0x0100 == 0x0100 {
                    self.cycles += 1; // oops cycle
                }
                self.cycles += 3;
                addr
            }
            AddressingMode::AbsY => {
                let addr = bytes_to_addr(ops[2], ops[1]).wrapping_add(u16::from(y));
                self.adv_pc(2);
                if (u16::from(y) + u16::from(ops[1])) & 0x0100 == 0x0100 {
                    self.cycles += 1; // oops cycle
                }
                self.cycles += 3;
                addr
            }
            AddressingMode::Accum => {
                // TODO: Make addressing Optional?
                0x0000
            }
            AddressingMode::Imm => {
                self.adv_pc(1);
                0x0000
            }
            AddressingMode::Impl => 0x0000,
            AddressingMode::IndX => {
                self.adv_pc(1);
                let val = Wrapping(ops[1]) + Wrapping(x);
                let lo = self.read_bus(u16::from(val.0));
                let hi = self.read_bus(0xFF & (u16::from(val.0) + 1));
                self.cycles += 1;
                bytes_to_addr(hi, lo)
            }
            AddressingMode::IndY => {
                self.adv_pc(1);
                let lo = self.read_bus(u16::from(ops[1]));
                let hi = self.read_bus(0xFF & (u16::from(ops[1]) + 1));
                if (u16::from(y) + u16::from(lo)) & 0x0100 == 0x0100 {
                    self.cycles += 1; // oops cycle
                }
                (Wrapping(bytes_to_addr(hi, lo)) + Wrapping(u16::from(y))).0
            }
            AddressingMode::Rel => {
                self.adv_pc(1);
                let bytes = pc.to_le_bytes();
                // The 'offset' is _signed_, so we need to add it as a signed
                // integer. Rust doesn't seem to like direct casts since they
                // can hide undefined behavior on some platforms, so we have
                // to be explicit.
                let lo = bytes[0];
                let hi = bytes[1];
                let addr = bytes_to_addr(hi, lo);
                if ops[1] > 127 {
                    // Twos compliment
                    addr.wrapping_sub(u16::from(!(ops[1]) + 1))
                } else {
                    addr.wrapping_add(u16::from(ops[1]))
                }
            }
            AddressingMode::ZP => {
                self.adv_pc(1);
                bytes_to_addr(0, ops[1])
            }
            AddressingMode::ZPX => {
                self.adv_pc(1);
                bytes_to_addr(0, (Wrapping(ops[1]) + Wrapping(x)).0)
            }
            AddressingMode::ZPY => {
                self.adv_pc(1);
                bytes_to_addr(0, (Wrapping(ops[1]) + Wrapping(y)).0)
            }
        }
    }

    /// Read a byte from the bus, adding one to the cycle time
    fn read_bus(&mut self, addr: u16) -> u8 {
        self.cycles += 1;
        self.bus.read(addr)
    }

    /// Read the data at the resolved address
    fn read(&mut self) -> u8 {
        let ops = self.state.instruction.to_le_bytes();
        match self.state.addr_mode {
            AddressingMode::Imm => ops[1],
            AddressingMode::Accum => self.state.acc,
            _ => self.read_bus(self.state.addr),
        }
    }

    fn write(&mut self, data: u8) {
        self.cycles += 1;
        self.bus.write(self.state.addr, data);
    }

    fn push_stack(&mut self, data: u8) {
        let addr = bytes_to_addr(0x01, self.state.stack);
        self.bus.write(addr, data);
        self.cycles += 1;
        self.state.stack = (Wrapping(self.state.stack) - Wrapping(1)).0;
    }

    fn pop_stack(&mut self) -> u8 {
        self.state.stack = (Wrapping(self.state.stack) + Wrapping(1)).0;
        let addr = bytes_to_addr(0x01, self.state.stack);
        self.read_bus(addr)
    }

    fn check_carry(&mut self, val: u16) {
        if val & 0x100 == 0x100 {
            // an overflow occured
            self.set_flag(Status::CARRY);
        } else {
            self.clear_flag(Status::CARRY);
        }
    }

    fn check_zero(&mut self, val: u8) {
        if val == 0 {
            self.set_flag(Status::ZERO);
        } else {
            self.clear_flag(Status::ZERO);
        }
    }

    fn check_overflow(&mut self, left: u8, right: u8) {
        let left = u16::from(left);
        let right = u16::from(right);
        let res = left + right;
        if ((left ^ res) & (right ^ res)) & 0x80 != 0 {
            self.set_flag(Status::OVERFLOW);
        } else {
            self.clear_flag(Status::OVERFLOW);
        }
    }

    fn check_negative(&mut self, op: u8) {
        if op & 0x80 != 0 {
            self.set_flag(Status::NEGATIVE);
        } else {
            self.clear_flag(Status::NEGATIVE);
        }
    }

    /// Execute the loaded instruction.
    ///
    /// Internally this uses a massive match pattern- TBD on whether this should
    /// be changed, but given that most of the instructions are self-contained
    /// and very short, I think it's not indefensible (plus it's easy).
    fn exec_instr(&mut self) {
        let CpuState {
            instr,
            ref status,
            ref acc,
            ref x,
            ref y,
            ref pc,
            ref stack,
            addr_mode,
            addr,
            ..
        } = self.state;
        match instr {
            //region Arithmetic ops
            // ADC SBC
            Instruction::ADC => {
                if status.contains(Status::DECIMAL) {
                    eprintln!(" [WARN] This emulator doesn't support BCD, but the BCD flag is set");
                }
                let op = self.read();
                let val = Wrapping(u16::from(*acc))
                    + Wrapping(u16::from(op))
                    + Wrapping(if status.contains(Status::CARRY) { 1 } else { 0 });
                self.check_carry(val.0);
                self.check_overflow(*acc, op);
                *acc = (0xFF & val.0) as u8;
                self.check_zero(*acc);
                self.check_negative(*acc);
            }
            Instruction::SBC => {
                if status.contains(Status::DECIMAL) {
                    eprintln!(" [WARN] This emulator doesn't support BCD, but the BCD flag is set");
                }
                let op = self.read();
                let val = Wrapping(u16::from(*acc))
                    - Wrapping(u16::from(op))
                    - Wrapping(if !status.contains(Status::CARRY) {
                        1
                    } else {
                        0
                    });
                self.check_carry(!val.0);
                self.check_overflow(*acc, !op);
                *acc = (0xFF & val.0) as u8;
                self.check_zero(*acc);
                self.check_negative(*acc);
            }
            //endregion

            //region Bitwise ops
            // AND BIT EOR ORA
            Instruction::AND => {
                *acc &= self.read();
                self.check_zero(*acc);
                self.check_negative(*acc);
            }
            Instruction::BIT => {
                let op = self.read();
                let res = *acc & op;
                self.check_zero(res);
                *status = Status::from_bits_truncate((status.bits() & 0x3F) | (0xC0 & op));
            }
            Instruction::EOR => {
                *acc ^= self.read();
                self.check_zero(*acc);
                self.check_negative(*acc);
            }
            Instruction::ORA => {
                *acc |= self.read();
                self.check_zero(*acc);
                self.check_negative(*acc);
            }
            //endregion
            Instruction::ASL => {
                let op = self.read();
                let res = u16::from(op) << 1;
                self.check_carry(res);
                let res = (0xFF & res) as u8;
                self.check_zero(res);
                self.check_negative(res);
                // Cycle corrections
                if addr_mode == AddressingMode::ZP || addr_mode == AddressingMode::Abs {
                    self.cycles += 1;
                };
                match addr_mode {
                    AddressingMode::Accum => *acc = res,
                    _ => self.write(res),
                }
            }

            //region Branch instructions
            // BPL BMI BVC BVS BCC BCS BEQ BNE
            Instruction::BPL => {
                if status.contains(Status::NEGATIVE) {
                    return;
                }
                self.cycles += 1;
                *pc = addr;
            }
            Instruction::BMI => {
                if !status.contains(Status::NEGATIVE) {
                    return;
                }
                self.cycles += 1;
                *pc = addr;
            }
            Instruction::BVC => {
                if status.contains(Status::OVERFLOW) {
                    return;
                }
                self.cycles += 1;
                *pc = addr;
            }
            Instruction::BVS => {
                if !status.contains(Status::OVERFLOW) {
                    return;
                }
                self.cycles += 1;
                *pc = addr;
            }
            Instruction::BCC => {
                if status.contains(Status::CARRY) {
                    return;
                }
                self.cycles += 1;
                *pc = addr;
            }
            Instruction::BCS => {
                if !status.contains(Status::CARRY) {
                    return;
                }
                self.cycles += 1;
                *pc = addr;
            }
            Instruction::BEQ => {
                if !status.contains(Status::ZERO) {
                    return;
                }
                self.cycles += 1;
                *pc = addr;
            }
            Instruction::BNE => {
                if status.contains(Status::ZERO) {
                    return;
                }
                self.cycles += 1;
                *pc = addr;
            }
            //endregion
            Instruction::BRK => {
                let addr_bytes = pc.to_le_bytes();
                self.push_stack(addr_bytes[1]);
                self.push_stack(addr_bytes[0]);
                self.set_flag(Status::BREAK);
                self.set_flag(Status::UNUSED);
                let status = status.bits();
                self.push_stack(status);
                let addr_hi = self.read_bus(0xFFFE);
                let addr_lo = self.read_bus(0xFFFF);
                *pc = bytes_to_addr(addr_lo, addr_hi);
            }

            //region Compare functions
            // CMP CPX CPY
            Instruction::CMP => {
                let data = self.read();
                let res = Wrapping(*acc) - Wrapping(data);
                status.set(Status::CARRY, *acc >= data);
                self.check_zero(res.0);
                self.check_negative(res.0);
            }
            Instruction::CPX => {
                let data = self.read();
                let res = Wrapping(*x) - Wrapping(data);
                status.set(Status::CARRY, *x >= data);
                self.check_zero(res.0);
                self.check_negative(res.0);
            }
            Instruction::CPY => {
                let data = self.read();
                let res = Wrapping(*y) - Wrapping(data);
                status.set(Status::CARRY, *y >= data);
                self.check_zero(res.0);
                self.check_negative(res.0);
            }
            // endregion

            //region Memory functions
            // DEC INC LSR ROL ROR
            Instruction::DEC => {
                let op = (Wrapping(self.read()) - Wrapping(1)).0;
                self.cycles += 1;
                self.write(op);
                self.check_zero(op);
                self.check_negative(op);
            }
            Instruction::INC => {
                let op = (Wrapping(self.read()) + Wrapping(1)).0;
                self.cycles += 1;
                self.write(op);
                self.check_zero(op);
                self.check_negative(op);
            }
            Instruction::LSR => {
                // I'm doing a bit of a trick here
                // If we look at the *high* byte, then functionally there's no
                // difference between (u16 << 7) and (u8 >> 1). But by casting
                // to u16 and doing it 'backwards', we preserve the lopped off
                // bit so that we can use it to set the carry bit
                let data = u16::from(self.read()) << 7;
                // we want the last bit for the carry -----v
                status.set(Status::CARRY, data & 0x00_80 == 0x00_80);
                // throw out the extra byte now that we're done with it
                let data = data.to_be_bytes()[0];
                self.check_zero(data);
                self.check_negative(data);
                // Finally, since this _could_ go to the accumulator, we need to
                // check for that addressing mode
                match addr_mode {
                    AddressingMode::ZP => {
                        self.cycles += 1;
                        self.write(data);
                    }
                    AddressingMode::Accum => *acc = data,
                    _ => self.write(data),
                };
                // cycle count correction
                if addr_mode == AddressingMode::Abs {
                    self.cycles += 1
                };
            }
            Instruction::ROR => {
                // See my notes on the LSR instruction, I do a similar trick
                // here (for similar reasons)
                let data = u16::from(self.read()) << 7
                    | if status.contains(Status::CARRY) {
                        0x80_00
                    } else {
                        0x0
                    };
                status.set(Status::CARRY, data & 0x00_80 == 0x00_80);
                let data = data.to_be_bytes()[0];
                self.check_zero(data);
                self.check_negative(data);
                // Even the caveat on addressing is the same
                match addr_mode {
                    AddressingMode::Accum => *acc = data,
                    _ => self.write(data),
                };
                // cycle count correction
                if addr_mode == AddressingMode::Abs || addr_mode == AddressingMode::ZP {
                    self.cycles += 1
                };
            }
            Instruction::ROL => {
                let data = (u16::from(self.read()) << 1)
                    | if status.contains(Status::CARRY) {
                        0x01
                    } else {
                        0x00
                    };
                status.set(Status::CARRY, data & 0x01_00 == 0x01_00);
                let data: u8 = (data & 0xFF) as u8;
                self.check_zero(data);
                self.check_negative(data);
                match addr_mode {
                    AddressingMode::Accum => *acc = data,
                    _ => self.write(data),
                };
                // cycle count correction
                if addr_mode == AddressingMode::Abs || addr_mode == AddressingMode::ZP {
                    self.cycles += 1
                };
            }
            //endregion

            //region Flag operations
            // CLC SEC CLI SEI CLV CLD SED
            Instruction::CLC => self.clear_flag(Status::CARRY),
            Instruction::SEC => self.set_flag(Status::CARRY),
            Instruction::CLI => self.clear_flag(Status::IRQ_DISABLE),
            Instruction::SEI => self.set_flag(Status::IRQ_DISABLE),
            Instruction::CLV => self.clear_flag(Status::OVERFLOW),
            Instruction::CLD => self.clear_flag(Status::DECIMAL),
            Instruction::SED => self.set_flag(Status::DECIMAL),
            //endregion

            //region Jumps
            // JMP JSR RTI RTS
            Instruction::JMP => {
                if addr_mode != AddressingMode::Abs {
                    self.cycles += 1;
                }
                *pc = addr;
            }
            Instruction::JSR => {
                if addr_mode != AddressingMode::Abs {
                    self.cycles += 1;
                }
                let addr_bytes = (*pc - 1).to_le_bytes();
                self.push_stack(addr_bytes[1]);
                self.push_stack(addr_bytes[0]);
                *pc = addr;
                self.cycles += 1;
            }
            Instruction::RTI => {
                let flags = self.pop_stack();
                *status = Status::from_bits_truncate(flags) | Status::UNUSED;
                let lo = self.pop_stack();
                let hi = self.pop_stack();
                *pc = bytes_to_addr(hi, lo);
                self.cycles += 1;
            }
            Instruction::RTS => {
                let lo = self.pop_stack();
                let hi = self.pop_stack();
                *pc = bytes_to_addr(hi, lo) + 1;
                self.cycles += 2;
            }
            //endregion

            //region Loads
            Instruction::LDA => {
                *acc = self.read();
                self.check_zero(*acc);
                self.check_negative(*acc);
            }
            Instruction::LDX => {
                *x = self.read();
                self.check_zero(*x);
                self.check_negative(*x);
            }
            Instruction::LDY => {
                *y = self.read();
                self.check_zero(*y);
                self.check_negative(*y);
            }
            //endregion
            Instruction::NOP => {
                // no operation
            }

            //region Register instructions
            Instruction::TAX => {
                *x = *acc;
                self.check_zero(*x);
                self.check_negative(*x);
            }
            Instruction::TXA => {
                *acc = *x;
                self.check_zero(*acc);
                self.check_negative(*acc);
            }
            Instruction::TAY => {
                *y = *acc;
                self.check_zero(*y);
                self.check_negative(*y);
            }
            Instruction::TYA => {
                *acc = *y;
                self.check_zero(*acc);
                self.check_negative(*acc);
            }
            Instruction::INX => {
                *x = (Wrapping(*x) + Wrapping(1)).0;
                self.check_zero(*x);
                self.check_negative(*x);
            }
            Instruction::DEX => {
                *x = (Wrapping(*x) - Wrapping(1)).0;
                self.check_zero(*x);
                self.check_negative(*x);
            }
            Instruction::INY => {
                *y = (Wrapping(*y) + Wrapping(1)).0;
                self.check_zero(*y);
                self.check_negative(*y);
            }
            Instruction::DEY => {
                *y = (Wrapping(*y) - Wrapping(1)).0;
                self.check_zero(*y);
                self.check_negative(*y);
            }
            //endregion

            //region Storage instruction
            Instruction::STA => {
                self.write(*acc);
                // Cycle count corrections
                if addr_mode == AddressingMode::IndY {
                    self.cycles += 1;
                }
            }
            Instruction::STX => {
                self.write(*x);
            }
            Instruction::STY => {
                self.write(*y);
            }
            //endregion

            //region Stack instructions
            Instruction::TXS => {
                *stack = *x;
            }
            Instruction::TSX => {
                *x = *stack;
                self.check_zero(*x);
                self.check_negative(*x);
            }
            Instruction::PHA => {
                self.push_stack(*acc);
            }
            Instruction::PLA => {
                *acc = self.pop_stack();
                self.check_zero(*acc);
                self.check_negative(*acc);
                self.cycles += 1;
            }
            Instruction::PHP => self.push_stack(status.bits() | 0x30),
            Instruction::PLP => {
                *status = Status::from_bits_truncate((self.pop_stack() & 0xEF) | 0x20);
                self.cycles += 1;
            } //endregion
        }
    }

    fn run_interrupt(&mut self) -> bool {
        if !self.interrupt_pending {
            return false;
        }
        eprintln!(
            " [INFO] CPU Interrupt: {}",
            if self.maskable_interrupt {
                "IRQ"
            } else {
                "NMI"
            }
        );
        self.interrupt_pending = false;
        let addr_bytes = self.state.pc.to_le_bytes();
        self.push_stack(addr_bytes[1]);
        self.push_stack(addr_bytes[0]);
        self.clear_flag(Status::BREAK);
        self.set_flag(Status::UNUSED);
        let status = self.state.status.bits();
        self.push_stack(status);
        let addr = if self.maskable_interrupt {
            0xFFFE
        } else {
            0xFFFA
        };
        let addr_lo = self.read_bus(addr);
        let addr_hi = self.read_bus(addr + 1);
        self.state.pc = bytes_to_addr(addr_lo, addr_hi);
        true
    }

    pub fn new() -> Cpu6502 {
        Cpu6502 {
            state: CpuState::new(),
            // internal state
            bus: Bus::new(),
            cycles: 0,
            interrupt_pending: false,
            maskable_interrupt: false,
        }
    }
}

impl fmt::Display for Cpu6502 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let CpuState {
            instruction,
            addr_mode,
            addr,
            instr,
            x,
            y,
            acc,
            pc,
            status,
            stack,
            tot_cycles,
            ..
        } = self.state;
        let bytes = instruction.to_le_bytes();
        let ops = match addr_mode {
            AddressingMode::Abs
            | AddressingMode::AbsX
            | AddressingMode::AbsY
            | AddressingMode::AbsInd => {
                format!("{:02X} {:02X} {:02X}", bytes[0], bytes[1], bytes[2])
            }
            AddressingMode::Accum | AddressingMode::Impl => format!("{:8<02X}", bytes[0]),
            _ => format!("{:02X} {:02X}   ", bytes[0], bytes[1]),
        };

        let operand_bytes = bytes_to_addr(bytes[2], bytes[1]);
        let bus = self.bus;
        let data = bus.read(addr);
        let is_jmp = instr == Instruction::JMP || instr == Instruction::JSR;
        let instr = match addr_mode {
            AddressingMode::Abs => {
                if !is_jmp {
                    format!("{:3?} ${:04X} = {:02X}", instr, addr, data)
                } else {
                    format!("{:3?} ${:04X}", instr, addr)
                }
            }
            AddressingMode::AbsX => format!(
                "{:3?} ${:04X},X @ {:04X} = {:02X}",
                instr, operand_bytes, addr, data
            ),
            AddressingMode::AbsY => format!(
                "{:3?} ${:04X},Y @ {:04X} = {:02X}",
                instr, operand_bytes, addr, data
            ),
            AddressingMode::AbsInd => {
                format!("{:3?} (${:04X}) = {:04X}", instr, operand_bytes, addr)
            }
            AddressingMode::Imm => format!("{:3?} #${:02X}", instr, bytes[1]),
            AddressingMode::ZP => format!("{:3?} ${:02X} = {:02X}", instr, addr, data),
            AddressingMode::ZPX => format!(
                "{:3?} ${:02X},X @ {:02X} = {:02X}",
                instr, bytes[1], addr, data
            ),
            AddressingMode::ZPY => format!(
                "{:3?} ${:02X},Y @ {:02X} = {:02X}",
                instr, bytes[1], addr, data
            ),
            AddressingMode::Impl => format!("{:3?}", instr),
            AddressingMode::Rel => format!("{:3?} ${:04X}", instr, addr),
            AddressingMode::Accum => format!("{:3?} A", instr),
            AddressingMode::IndX => {
                let sum = Wrapping(x) + Wrapping(bytes[1]);
                format!(
                    "{:3?} (${:02X},X) @ {:02X} = {:04X} = {:02X}",
                    instr, bytes[1], sum, addr, data
                )
            }
            AddressingMode::IndY => {
                let ind = bytes_to_addr(
                    bus.read(0xFF & (u16::from(bytes[1]) + 1)),
                    bus.read(u16::from(bytes[1])),
                );
                format!(
                    "{:3?} (${:02X}),Y = {:04X} @ {:04X} = {:02X}",
                    instr, bytes[1], ind, addr, data
                )
            }
        };
        write!(
            f,
            //PC     Ops   Inst Accum    X reg    Y reg    Status   Stack     PPU.row...line  tot_cycles
            "{:04X}  {:8}  {:32}A:{:02X} X:{:02X} Y:{:02X} P:{:02X} SP:{:02X} PPU:{:>3},{:>3} CYC:{}",
            pc,
            ops,
            instr,
            acc,
            x,
            y,
            status,
            stack,
            0,
            0,
            tot_cycles
        )
    }
}
