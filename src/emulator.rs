// Emulator for the MOS 6502
use std::fmt;
use std::rc::Rc;

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
    u16::from(hi) << (8 + u16::from(lo))
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

    /// The resolved address of the instruction
    addr: u16,

    /// The addressing mode of the opcode being executed
    addr_mode: AddressingMode,

    /// The opcode being executed
    instr: Instruction,
    //endregion

    //region stuff
    bus: Rc<Bus>,
}

impl Cpu6502 {
    pub fn tick(&mut self) {
        self.tot_cycles += 1;
        if self.cycles > 0 {
            self.cycles -= 1;
            return;
        }

        // execute instruction
        self.decode_opcode(self.instruction);
        self.addr = self.get_addr(self.instruction);
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
            },
            0x20 => {
                self.instr = Instruction::JSR;
                self.addr_mode = AddressingMode::Abs;
                return;
            },
            0x40 => {
                self.instr = Instruction::RTI;
                self.addr_mode = AddressingMode::Impl;
                return;
            },
            0x6C => {
                self.instr = Instruction::RTS;
                self.addr_mode = AddressingMode::AbsInd;
                return;
            },
            0x8A => { self.addr_mode = AddressingMode::Impl; self.instr = Instruction::TXA; return; },
            0x9A => { self.addr_mode = AddressingMode::Impl; self.instr = Instruction::TXS; return; },
            0xAA => { self.addr_mode = AddressingMode::Impl; self.instr = Instruction::TAX; return; },
            0xBA => { self.addr_mode = AddressingMode::Impl; self.instr = Instruction::TSX; return; },
            0xCA => { self.addr_mode = AddressingMode::Impl; self.instr = Instruction::DEX; return; },
            0xEA => { self.addr_mode = AddressingMode::Impl; self.instr = Instruction::NOP; return; },
            _ => {}
        };

        let subtable = ops[0] & 0x3;
        let addr_mode = ops[0] & 0x1c;
        let opcode = ops[0] & 0xe0;

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
                    _ => panic!("Invalid opcode")
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
                    _ => panic!("Invalid addressing mode")
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
                    _ => panic!("Invalid opcode")
                };
                // the STX and LDX instructions should target the Y index register instead
                let use_y = self.instr == Instruction::STX || self.instr == Instruction::LDX;
                self.addr_mode = match addr_mode {
                    0b000 => AddressingMode::Imm,
                    0b001 => AddressingMode::ZP,
                    0b010 => AddressingMode::Accum,
                    0b011 => AddressingMode::Abs,
                    // skip 0b100 (branch instr)
                    0b101 => if use_y { AddressingMode::ZPY } else { AddressingMode::ZPX },
                    // skip 0b110 (single byte instr)
                    0b111 => if use_y { AddressingMode::AbsY } else { AddressingMode::AbsX },
                    _ => panic!("Invalid addressing mode")
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
                        _ => panic!("Invalid instruction")
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
                        _ => panic!("Invalid opcode")
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
                    _ => panic!("Invalid opcode")
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
                    _ => panic!("Invalid addressing mode")
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

        match self.addr_mode {
            AddressingMode::Abs => {
                self.cycles += 2;
                bytes_to_addr(ops[2], ops[1])
            }
            AddressingMode::AbsInd => {
                let addr = bytes_to_addr(ops[2], ops[1]);
                let lo = self.bus.read(addr);
                let hi = self.bus.read(addr + 1);
                // TODO: JMP,AbsInd should get the right # of cycles
                self.cycles += 3;
                bytes_to_addr(hi, lo)
            }
            AddressingMode::AbsX => {
                let addr = bytes_to_addr(ops[2], ops[1]) + u16::from(self.x);
                if (u16::from(self.x) + u16::from(ops[1])) & 0x0100 == 0x0100 {
                    self.cycles += 1; // oops cycle
                }
                self.cycles += 3;
                addr
            }
            AddressingMode::AbsY => {
                let addr = bytes_to_addr(ops[2], ops[1]) + u16::from(self.y);
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
                0x0000
            }
            AddressingMode::Impl => {
                0x0000
            }
            AddressingMode::IndX => {
                let lo = self.read_bus(u16::from(ops[1] + self.x));
                let hi = self.read_bus(u16::from(ops[1] + self.x + 1));
                self.cycles += 2;
                bytes_to_addr(lo, hi)
            }
            AddressingMode::IndY => {
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
                self.pc + u16::from(ops[1])
            }
            AddressingMode::ZP => {
                bytes_to_addr(ops[1], 0)
            }
            AddressingMode::ZPX => bytes_to_addr(ops[1] + self.x, 0),
            AddressingMode::ZPY => bytes_to_addr(ops[1] + self.y, 0),
        }
    }

    /// Read a byte from the bus, adding one to the cycle time
    fn read_bus(&mut self, addr: u16) -> u8 {
        self.cycles += 1;
        self.bus.read(addr)
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
    pub fn new(bus: Rc<Bus>) -> Cpu6502 {
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
            tot_cycles: 0,
            instruction: 0,
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
            | AddressingMode::AbsInd => {
                format!("{:2X} {:2X} {:2X}", bytes[0], bytes[1], bytes[2])
            }
            AddressingMode::Accum | AddressingMode::Impl => format!("{:8<2X}", bytes[0]),
            _ => format!("{:2X} {:2X}   ", bytes[0], bytes[1]),
        };
        write!(
            f,
            //PC     Ops   Inst Accum    X reg    Y reg    Status   Stack     PPU.row...line  tot_cycles
            "{:04X}  {:8}  {:32}A:{:02X} X:{:02X} Y:{:02X} P:{:02X} SP:{:02X} PPU:{:>3},{:>3} CYC:{}",
            self.pc,
            ops,
            "FAKE INSTR", //TODO: we need a way of formatting decoded instructions
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
