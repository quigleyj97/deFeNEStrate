// Emulator for the MOS 6502
use std::fmt;
use std::cell::{RefCell};
use std::rc::{Rc};

use crate::databus::Bus;
use crate::structs::{AddressingMode, Instruction};

bitflags! {
    pub struct Status: u8 {
        const CARRY = 0x01;
        const ZERO = 0x02;
        const IRQ_DISABLE = 0x04;
        const DECIMAL = 0x08;
        const BREAK = 0x10;
        const UNUSED = 0x20;
        const OVERFLOW = 0x40;
        const NEGATIVE = 0x80;
    }
}

fn bytes_to_addr(lo: u8, hi: u8) -> u16 {
    (u16::from(lo) << 8) + u16::from(hi)
}

pub struct Cpu6502 {
    //region CPU Registers
    /// The Accumulator register
    acc: u8,

    /// X index register
    x: u8,

    /// Y index register
    y: u8,

    /// The stack pointer
    ///
    /// # Note
    ///
    /// This register is a pointer to a location in memory on the first page
    /// ($01XX) of memory. The 6502 uses a bottom-up stack, so the 'first'
    /// location on the stack is `$01FF` and the 'last' is `$0100`.
    ///
    /// Stack _overflow_ occurs when the stack pointer decreases all the way to
    /// $00 and wraps around to $FF (the beginning). _Underflow_ occurs the
    /// other way around, from $FF to $00.
    stack: u8,

    /// The program counter
    ///
    /// # Note
    ///
    /// This is incremented by the emulator after executing each instruction,
    /// and refers to the address in memory of the next instruction
    pc: u16,

    /// The instruction being executed.
    ///
    /// # Note
    ///
    /// Instructions consist of an opcode, having 1 byte, and an optional
    /// operand having 1 or 2 bytes (depending on the instruction and addressing
    /// mode).
    ///
    /// The last 8 bits of this register are unused.
    instruction: u32,

    /// The program status register.
    status: Status,
    //endregion

    //region internal state
    // The variables below are used as internal state by the emulator, and
    // are unrelated to the 6502.
    /// The number of cycles to wait before executing the next instruction.
    ///
    /// # Note
    ///
    /// On the 6502, most instructions took longer than 1 clock cycle. Some
    /// took quite a few more, as the instruction had to read off operands
    /// from memory. This is a counter to simulate that- if not zero,
    /// `clock` will simply decrement this and continue.
    cycles: u8,

    /// The total number of cycles that this CPU has ran
    ///
    /// # Note
    ///
    /// This is allowed to overflow, as it's only used for debugging and test
    /// comparison. It is not a part of core emulation.
    tot_cycles: u32,

    /// The last value of the program counter, used for debugging.
    ///
    /// During execution of an instruction, the pc will be changed (and some
    /// instructions might change it further, like JMP). To know where it came
    /// from when looking through logs, you need to print _this_ instead.
    last_pc: u16,

    /// The resolved address of the instruction
    addr: u16,

    /// The addressing mode of the opcode being executed
    addr_mode: AddressingMode,

    /// The opcode being executed
    instr: Instruction,
    //endregion

    //region stuff
    bus: Rc<RefCell<Bus>>,
}

impl Cpu6502 {
    pub fn tick(&mut self) -> bool {
        self.tot_cycles += 1;
        if self.cycles > 0 {
            self.cycles -= 1;
            return false;
        }
        true
    }

    pub fn exec(&mut self) {
        self.load_opcode();
        self.decode_opcode(self.instruction);
        self.last_pc = self.pc;
        self.addr = self.get_addr(self.instruction);
        self.exec_instr();
    }

    pub fn reset(&mut self) {
        self.stack -= 3;
        self.status |= Status::IRQ_DISABLE;
    }

    pub fn set_flag(&mut self, flag: Status) {
        self.status |= flag;
    }

    pub fn clear_flag(&mut self, flag: Status) {
        self.status &= !flag;
    }

    fn load_opcode(&mut self) {
        let bus = self.bus.borrow();
        let opcode = bus.read(self.pc);
        let operand1 = bus.read(self.pc + 1);
        let operand2 = bus.read(self.pc + 2);
        self.instruction = u32::from(opcode) + (u32::from(operand1) << 8) + (u32::from(operand2) << 16)
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

        // Instructions are structured as:
        //   0......7 8..........15 16.........23
        //   aaabbbcc <lo operand?> <hi operand?>
        //
        // The `cc` bits differentiate between a few subtables. The `aaa` bits
        // determine the opcode, and the `bbb` bits determine the addrssing
        // mode. `cc` never takes the form `11`.

        // Before we go any further, there's a few instructions that are better
        // to special case.
        match ops[0] {
            0x00 => {
                self.instr = Instruction::BRK;
                self.addr_mode = AddressingMode::Impl;
                return;
            }
            0x20 => {
                self.instr = Instruction::JSR;
                self.addr_mode = AddressingMode::Abs;
                return;
            }
            0x40 => {
                self.instr = Instruction::RTI;
                self.addr_mode = AddressingMode::Impl;
                return;
            }
            0x6C => {
                self.instr = Instruction::RTS;
                self.addr_mode = AddressingMode::AbsInd;
                return;
            }
            0x8A => {
                self.addr_mode = AddressingMode::Impl;
                self.instr = Instruction::TXA;
                return;
            }
            0x9A => {
                self.addr_mode = AddressingMode::Impl;
                self.instr = Instruction::TXS;
                return;
            }
            0xAA => {
                self.addr_mode = AddressingMode::Impl;
                self.instr = Instruction::TAX;
                return;
            }
            0xBA => {
                self.addr_mode = AddressingMode::Impl;
                self.instr = Instruction::TSX;
                return;
            }
            0xCA => {
                self.addr_mode = AddressingMode::Impl;
                self.instr = Instruction::DEX;
                return;
            }
            0xEA => {
                self.addr_mode = AddressingMode::Impl;
                self.instr = Instruction::NOP;
                return;
            }
            _ => {}
        };

        let subtable = ops[0] & 0x3;
        let addr_mode = (ops[0] & 0x1c) >> 2;
        let opcode = (ops[0] & 0xe0) >> 5;

        match subtable {
            0b01 => {
                self.instr = match opcode {
                    0b000 => Instruction::ORA,
                    0b001 => Instruction::AND,
                    0b010 => Instruction::EOR,
                    0b011 => Instruction::ADC,
                    0b100 => Instruction::STA,
                    0b101 => Instruction::LDA,
                    0b110 => Instruction::CMP,
                    0b111 => Instruction::SBC,
                    _ => panic!("Invalid opcode"),
                };
                self.addr_mode = match addr_mode {
                    0b000 => AddressingMode::IndX,
                    0b001 => AddressingMode::ZP,
                    0b010 => AddressingMode::Imm,
                    0b011 => AddressingMode::Abs,
                    0b100 => AddressingMode::IndY,
                    0b101 => AddressingMode::ZPX,
                    0b110 => AddressingMode::AbsY,
                    0b111 => AddressingMode::AbsX,
                    _ => panic!("Invalid addressing mode"),
                };
            }
            0b10 => {
                self.instr = match opcode {
                    0b000 => Instruction::ASL,
                    0b001 => Instruction::ROL,
                    0b010 => Instruction::LSR,
                    0b011 => Instruction::ROR,
                    0b100 => Instruction::STX,
                    0b101 => Instruction::LDX,
                    0b110 => Instruction::DEC,
                    0b111 => Instruction::INC,
                    _ => panic!("Invalid opcode"),
                };
                // the STX and LDX instructions should target the Y index register instead
                let use_y = self.instr == Instruction::STX || self.instr == Instruction::LDX;
                self.addr_mode = match addr_mode {
                    0b000 => AddressingMode::Imm,
                    0b001 => AddressingMode::ZP,
                    0b010 => AddressingMode::Accum,
                    0b011 => AddressingMode::Abs,
                    // skip 0b100 (branch instr)
                    0b101 => {
                        if use_y {
                            AddressingMode::ZPY
                        } else {
                            AddressingMode::ZPX
                        }
                    }
                    // skip 0b110 (single byte instr)
                    0b111 => {
                        if use_y {
                            AddressingMode::AbsY
                        } else {
                            AddressingMode::AbsX
                        }
                    }
                    _ => panic!("Invalid addressing mode"),
                }
            }
            0b00 => {
                if ops[0] & 0x0F == 0x08 {
                    // this is a single byte instruction, and doesn't map to the AAABBBCC pattern
                    // handle it specially
                    self.addr_mode = AddressingMode::Impl;
                    self.instr = match ops[0] >> 4 {
                        0x0 => Instruction::PHP,
                        0x1 => Instruction::CLC,
                        0x2 => Instruction::PLP,
                        0x3 => Instruction::SEC,
                        0x4 => Instruction::PHA,
                        0x5 => Instruction::CLI,
                        0x6 => Instruction::PLA,
                        0x7 => Instruction::SEI,
                        0x8 => Instruction::DEY,
                        0x9 => Instruction::TYA,
                        0xA => Instruction::TAY,
                        0xB => Instruction::CLV,
                        0xC => Instruction::INY,
                        0xD => Instruction::CLD,
                        0xE => Instruction::INX,
                        0xF => Instruction::SED,
                        _ => panic!("Invalid instruction"),
                    };
                    return;
                }
                if opcode == 0b100 {
                    // these are branch instructions
                    self.addr_mode = AddressingMode::Rel;
                    // in this case the AAAs are actually the instruction type
                    self.instr = match addr_mode {
                        0b000 => Instruction::BPL,
                        0b001 => Instruction::BMI,
                        0b010 => Instruction::BVC,
                        0b011 => Instruction::BVS,
                        0b100 => Instruction::BCC,
                        0b101 => Instruction::BCS,
                        0b110 => Instruction::BNE,
                        0b111 => Instruction::BEQ,
                        _ => panic!("Invalid opcode"),
                    };
                }
                self.instr = match opcode {
                    // skip 0b000 (branch instr)
                    0b001 => Instruction::BIT,
                    0b010 => Instruction::JMP,
                    0b011 => Instruction::JMP, // abs addressing
                    0b100 => Instruction::STY,
                    0b101 => Instruction::LDY,
                    0b110 => Instruction::CPY,
                    0b111 => Instruction::CPX,
                    _ => panic!("Invalid opcode"),
                };
                self.addr_mode = match addr_mode {
                    0b000 => AddressingMode::Imm,
                    0b001 => AddressingMode::ZP,
                    // skip 0b010
                    0b011 => AddressingMode::Abs,
                    // skip 0b100 (branch instr)
                    0b101 => AddressingMode::ZPX,
                    // skip 0b110
                    0b111 => AddressingMode::AbsX,
                    _ => panic!("Invalid addressing mode"),
                }
            }
            0b11 => {}
            _ => panic!("Invalid instruction"),
        }
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
        let ops = instruction.to_le_bytes();
        // +2 cycles for instr + byte1 of op readout, minimum
        self.cycles += 2;
        // Advance the PC at _least_ 1 byte
        self.pc += 1;

        match self.addr_mode {
            AddressingMode::Abs => {
                self.cycles += 2;
                self.pc += 2;
                bytes_to_addr(ops[2], ops[1])
            }
            AddressingMode::AbsInd => {
                let addr = bytes_to_addr(ops[2], ops[1]);
                self.pc += 2;
                let lo = self.read_bus(addr);
                let hi = self.read_bus(addr + 1);
                // TODO: JMP,AbsInd should get the right # of cycles
                self.cycles += 1;
                bytes_to_addr(hi, lo)
            }
            AddressingMode::AbsX => {
                let addr = bytes_to_addr(ops[2], ops[1]) + u16::from(self.x);
                self.pc += 2;
                if (u16::from(self.x) + u16::from(ops[1])) & 0x0100 == 0x0100 {
                    self.cycles += 1; // oops cycle
                }
                self.cycles += 3;
                addr
            }
            AddressingMode::AbsY => {
                let addr = bytes_to_addr(ops[2], ops[1]) + u16::from(self.y);
                self.pc += 2;
                if (u16::from(self.y) + u16::from(ops[1])) & 0x0100 == 0x0100 {
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
                self.pc += 1;
                0x0000
            }
            AddressingMode::Impl => 0x0000,
            AddressingMode::IndX => {
                self.pc += 1;
                let lo = self.read_bus(u16::from(ops[1] + self.x));
                let hi = self.read_bus(u16::from(ops[1] + self.x + 1));
                self.cycles += 2;
                bytes_to_addr(lo, hi)
            }
            AddressingMode::IndY => {
                self.pc += 1;
                let lo = self.read_bus(u16::from(ops[1]));
                // wrap cast to make sure Rust doesn't expand either op prematurely
                let hi = self.read_bus(u16::from((ops[1] + 1) as u8));
                self.cycles += 1;
                if (u16::from(self.y) + u16::from(ops[1])) & 0x0100 == 0x0100 {
                    self.cycles += 1; // oops cycle
                }
                bytes_to_addr(lo, hi) + u16::from(self.y)
            }
            AddressingMode::Rel => {
                self.pc += 1;
                (self.pc - 2) + u16::from(ops[1])
            },
            AddressingMode::ZP => {
                self.pc += 1;
                bytes_to_addr(ops[1], 0)
            }
            AddressingMode::ZPX => {
                self.pc += 1;
                bytes_to_addr(ops[1] + self.x, 0)
            }
            AddressingMode::ZPY => {
                self.pc += 1;
                bytes_to_addr(ops[1] + self.y, 0)
            }
        }
    }

    /// Read a byte from the bus, adding one to the cycle time
    fn read_bus(&mut self, addr: u16) -> u8 {
        self.cycles += 1;
        let bus = self.bus.borrow();
        bus.read(addr)
    }

    /// Read the data at the resolved address
    fn read(&mut self) -> u8 {
        let ops = self.instruction.to_le_bytes();
        match self.addr_mode {
            AddressingMode::Imm => ops[1],
            AddressingMode::Accum => self.acc,
            _ => self.read_bus(self.addr)
        }
    }

    fn write(&mut self, data: u8) {
        self.cycles += 1;
        let mut bus = self.bus.borrow_mut();
        bus.write(self.addr, data);
    }

    fn push_stack(&mut self, data: u8) {
        let mut bus = self.bus.borrow_mut();
        let addr = bytes_to_addr(0x01, self.stack);
        bus.write(addr, data);
        self.cycles += 1;
        self.stack -= 1;
    }

    fn pop_stack(&mut self) -> u8 {
        let addr = bytes_to_addr(0x01, self.stack);
        self.stack += 1;
        self.read_bus(addr)
    }

    /// Execute the loaded instruction.
    ///
    /// Internally this uses a massive match pattern- TBD on whether this should
    /// be changed, but given that most of the instructions are self-contained
    /// and very short, I think it's not indefensible (plus it's easy).
    fn exec_instr(&mut self) {
        match self.instr {
            Instruction::ADC => {
                if self.status.contains(Status::DECIMAL) {
                    println!(" [WARN] This emulator doesn't support BCD, but the BCD flag is set");
                }
                let val: u16 = u16::from(self.acc) + u16::from(self.read());
                if val & 0x10 == 0x10 {
                    // an overflow occured
                    self.set_flag(Status::OVERFLOW);
                }
                self.acc = (0x0F & val) as u8;
            }
            Instruction::JMP => {
                self.cycles += 1;
                self.pc = self.addr;
            }
            Instruction::JSR => {
                let addr_bytes = (self.addr - 1).to_le_bytes();
                self.push_stack(addr_bytes[0]);
                self.push_stack(addr_bytes[1]);
                self.pc = self.addr;
            }
            Instruction::LDX => {
                self.x = self.read();
            }
            Instruction::STX => {
                self.write(self.x);
            }
            _ => {} // treat as no op
        }
    }
}

// Statics
impl Cpu6502 {
    /// Create a new CPU, connected to the given databus.
    ///
    /// # Note
    ///
    /// Default values are the NES power-up vals
    /// cf. http://wiki.nesdev.com/w/index.php/CPU_power_up_state
    pub fn new(bus: Rc<RefCell<Bus>>) -> Cpu6502 {
        Cpu6502 {
            acc: 0,
            x: 0,
            y: 0,
            stack: 0xFD,
            pc: 0xC000,
            // IRQ disabled
            // Unwrapping b/c this is a constant and should be OK
            status: Status::from_bits(0x24).unwrap(),

            // internal state
            bus,
            cycles: 0,
            tot_cycles: 1,
            last_pc: 0xC000,
            instruction: 0xEA,
            addr: 0,
            addr_mode: AddressingMode::Impl,
            instr: Instruction::NOP,
        }
    }
}

impl fmt::Display for Cpu6502 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let bytes = self.instruction.to_le_bytes();
        let ops = match self.addr_mode {
            AddressingMode::Abs
            | AddressingMode::AbsX
            | AddressingMode::AbsY
            | AddressingMode::AbsInd => format!("{:02X} {:02X} {:02X}", bytes[0], bytes[1], bytes[2]),
            AddressingMode::Accum | AddressingMode::Impl => format!("{:8<2X}", bytes[0]),
            _ => format!("{:02X} {:02X}   ", bytes[0], bytes[1]),
        };

        let operand_bytes = bytes_to_addr(bytes[1], bytes[2]);
        let bus = self.bus.borrow();
        let data = bus.read(self.addr);
        let addr = self.addr;
        let instr = match self.addr_mode {
            AddressingMode::Abs => format!("{:3?} ${:04X}", self.instr, addr),
            AddressingMode::AbsX => format!("{:3?} ${:04X},X @ {:04X} = {:02X}", self.instr, operand_bytes, addr, data),
            AddressingMode::AbsY => format!("{:3?} ${:04X},Y @ {:04X} = {:02X}", self.instr, operand_bytes, addr, data),
            AddressingMode::AbsInd => format!("{:3?} (${:04X}) = {:04X}", self.instr, operand_bytes, addr),
            AddressingMode::Imm => format!("{:3?} #${:02X}", self.instr, data),
            AddressingMode::ZP => format!("{:3?} ${:02X} = {:02X}", self.instr, bytes[1], data),
            AddressingMode::ZPX => format!("{:3?} ${:02X},X @ {:02X} = {:02X}", self.instr, bytes[1], self.x, addr),
            AddressingMode::ZPY => format!("{:3?} ${:02X},Y @ {:02X} = {:02X}", self.instr, bytes[1], self.y, addr),
            AddressingMode::Impl => format!("{:3?}", self.instr),
            _ => format!("{:3?} {:02X} {:02X} <TODO>", self.instr, bytes[1], bytes[2])
        };
        write!(
            f,
            //PC     Ops   Inst Accum    X reg    Y reg    Status   Stack     PPU.row...line  tot_cycles
            "{:04X}  {:8}  {:32}A:{:02X} X:{:02X} Y:{:02X} P:{:02X} SP:{:02X} PPU:{:>3},{:>3} CYC:{}",
            self.last_pc,
            ops,
            instr,
            self.acc,
            self.x,
            self.y,
            self.status,
            self.stack,
            0,
            0,
            self.tot_cycles
        )
    }
}
