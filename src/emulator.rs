// Emulator for the MOS 6502
use std::fmt;
use std::rc::Rc;

use crate::databus::Bus;

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
    return (hi as u16) << 8 + lo as u16;
}

#[derive(Debug)]
enum AddressingMode {
    /// Zero-Page
    ZP,
    /// Zero-Page Indexed, X register
    ZPX,
    /// Zero-Page Indexed, Y register
    ZPY,
    /// Absolute Indexed, plus X register
    AbsX,
    /// Absolute Indexed, plus Y register
    AbsY,
    /// Indexed Indirect (d, x)
    IndX,
    /// Indirect Indexed (d), y
    ///
    /// gee thanks MOS what a helpful name
    /// not like there's a significant difference between how (d, x) and (d),y
    /// work
    ///
    /// ...oh wait
    IndY,
    /// Implicit indexing (do nothing, resolve nothing, deny everything)
    Impl,
    /// Use the Accumulator
    Accum,
    /// Don't fetch anything and use the operand as data
    Imm,
    /// Jump to a relative label
    Rel,
    /// Addressing mode specific to JMP
    AbsInd,
    /// The 16 address is included in the operand
    Abs,
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
    /// operand having 0, 1, or 2 bytes.
    ///
    /// The last 8 bits of this register are unused.
    opcode: u32,

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

    /// The instruction of the opcode being executed
    // insr: InstructionMnemonic,
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
    fn get_addr(&mut self, opcode: u32) -> u16 {
        let ops = opcode.to_le_bytes();
        // +2 cycles for instr + byte1 of op readout, minimum
        self.cycles += 2;

        return match self.addr_mode {
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
                return bytes_to_addr(hi, lo);
            }
            AddressingMode::AbsX => {
                let addr = bytes_to_addr(ops[2], ops[1]) + self.x as u16;
                if (self.x as u16 + ops[1] as u16) & 0x0100 == 0x0100 {
                    self.cycles += 1; // oops cycle
                }
                self.cycles += 3;
                return addr;
            }
            AddressingMode::AbsY => {
                let addr = bytes_to_addr(ops[2], ops[1]) + self.y as u16;
                if (self.y as u16 + ops[1] as u16) & 0x0100 == 0x0100 {
                    self.cycles += 1; // oops cycle
                }
                self.cycles += 3;
                return addr;
            }
            AddressingMode::Accum => {
                // TODO: Make addressing Optional?
                return 0x0000;
            }
            AddressingMode::Imm => {
                return 0x0000;
            }
            AddressingMode::Impl => {
                return 0x0000;
            }
            AddressingMode::IndX => {
                let lo = self.read_bus((ops[1] + self.x) as u16);
                let hi = self.read_bus((ops[1] + self.x + 1) as u16);
                self.cycles += 2;
                return bytes_to_addr(lo, hi);
            }
            AddressingMode::IndY => {
                let lo = self.read_bus(ops[1] as u16);
                // wrap cast to make sure Rust doesn't expand either op prematurely
                let hi = self.read_bus(((ops[1] + 1) as u8) as u16);
                self.cycles += 1;
                if (self.y as u16 + ops[1] as u16) & 0x0100 == 0x0100 {
                    self.cycles += 1; // oops cycle
                }
                return bytes_to_addr(lo, hi) + self.y as u16;
            }
            AddressingMode::Rel => {
                return self.pc + (ops[1] as u16);
            }
            AddressingMode::ZP => {
                return bytes_to_addr(ops[1], 0);
            }
            AddressingMode::ZPX => bytes_to_addr(ops[1] + self.x, 0),
            AddressingMode::ZPY => bytes_to_addr(ops[1] + self.y, 0),
        };
    }

    /// Read a byte from the bus, adding one to the cycle time
    fn read_bus(&mut self, addr: u16) -> u8 {
        self.cycles += 1;
        return self.bus.read(addr);
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
        return Cpu6502 {
            acc: 0,
            x: 0,
            y: 0,
            stack: 0xFD,
            pc: 0xC000,
            // IRQ disabled
            // Unwrapping b/c this is a constant and should be OK
            status: Status::from_bits(0x24).unwrap(),

            // internal state
            bus: bus,
            cycles: 0,
            tot_cycles: 0,
            opcode: 0,
            addr: 0,
            addr_mode: AddressingMode::Impl,
        };
    }
}

impl fmt::Display for Cpu6502 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let opcodes = self.opcode.to_le_bytes();
        let ops = match self.addr_mode {
            AddressingMode::Abs
            | AddressingMode::AbsX
            | AddressingMode::AbsY
            | AddressingMode::AbsInd => {
                format!("{:2X} {:2X} {:2X}", opcodes[0], opcodes[1], opcodes[2])
            }
            AddressingMode::Accum | AddressingMode::Impl => format!("{:8<2X}", opcodes[0]),
            _ => format!("{:2X} {:2X}   ", opcodes[0], opcodes[1]),
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
