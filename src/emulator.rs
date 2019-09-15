// Emulator for the Motorola 6502
use std::rc::Rc;
use crate::databus::{Bus};

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
    return ((hi as u16) % 256) * 256 + lo as u16;
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
    /// gee thanks motorola what a helpful name
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

    pub fn print_debug(&self) {
        println!("Status: {:#?}", self.status);
        println!("Acc: {:x}, X: {:x}, Y: {:x}", self.acc, self.x, self.y);
        println!("PC: {:x}, instr: {:x}", self.pc, self.opcode);
        let addr = self.get_addr(self.opcode);
        println!("Addr mode: {:#?}, resolved addr: {:x}", self.addr_mode, addr);
    }

    /// Gets the address of the operand to read from.
    /// 
    /// # Notes
    /// 
    /// This does _not_ sets the `cycles` property as required for emulation!
    /// Some instructions (like all the store instructions) have some special-
    /// cased behavior that the 6502 datasheet details.
    fn get_addr(&self, opcode: u32) -> u16 {
        let ops = opcode.to_le_bytes();
        return match self.addr_mode {
            AddressingMode::Abs => bytes_to_addr(ops[2], ops[1]),
            AddressingMode::AbsInd => {
                let addr = bytes_to_addr(ops[2], ops[1]);
                let lo = self.bus.read(addr);
                let hi = self.bus.read(addr + 1);
                return bytes_to_addr(hi, lo);
            },
            _ => {
                panic!("Unimplemented addressing mode")
            }
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
    pub fn new(bus: Rc<Bus>) -> Cpu6502 {
        return Cpu6502 {
            acc: 0,
            x: 0,
            y: 0,
            stack: 0xFD,
            pc: 0,
            // IRQ disabled
            // Unwrapping b/c this is a constant and should be OK
            status: Status::from_bits(0x34).unwrap(), 

            // internal state
            bus: bus,
            cycles: 0,
            opcode: 0,
            addr: 0,
            addr_mode: AddressingMode::Impl,
        }
    }
}
