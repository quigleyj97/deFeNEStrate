/// A struct holding state information about a 6502 CPU.
///
/// This struct is held internally, but can be copied to power to things
/// like debug formatters and, if taken at the end of a simulation cycle,
/// serialization.
#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub struct CpuState {
    /// The Accumulator register
    pub acc: u8,

    /// X index register
    pub x: u8,

    /// Y index register
    pub y: u8,

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
    pub stack: u8,

    /// The program counter
    ///
    /// # Note
    ///
    /// This is incremented by the emulator after executing each instruction,
    /// and refers to the address in memory of the next instruction
    pub pc: u16,

    /// The instruction being executed.
    ///
    /// # Note
    ///
    /// Instructions consist of an opcode, having 1 byte, and an optional
    /// operand having 1 or 2 bytes (depending on the instruction and addressing
    /// mode).
    ///
    /// The last 8 bits of this register are unused.
    pub instruction: u32,

    /// The program status register.
    pub status: Status,

    /// The total number of cycles that this CPU has ran
    ///
    /// # Note
    ///
    /// This is allowed to overflow, as it's only used for debugging and test
    /// comparison. It is not a part of core emulation.
    pub tot_cycles: u32,

    /// The resolved address of the instruction
    pub addr: u16,

    /// The addressing mode of the opcode being executed
    pub addr_mode: AddressingMode,

    /// The opcode being executed
    pub instr: Instruction,
}

// The addressing mode for the CPU
#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub enum AddressingMode {
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

/// The CPU opcode mnemonic
///
/// *depends on BCD flag, not currently supported
#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub enum Instruction {
    /// ADd with Carry*
    ADC,
    /// bitwise AND w/ acc
    AND,
    /// Arithmetic Shift Left
    ASL,
    /// test BITs
    BIT,

    //region Branch instructions
    /// Branch on PLus
    BPL,
    /// Branch on MInus
    BMI,
    /// Branch on oVerflow Clear
    BVC,
    /// Branch on oVerflow Set
    BVS,
    /// Branch on Carry Clear
    BCC,
    /// Branch on Carry Set
    BCS,
    /// Branch on Not Equal
    BNE,
    /// Branch on EQual
    BEQ,
    //endregion
    /// BReaK
    BRK,
    /// CoMPare acc
    CMP,
    /// ComPare X
    CPX,
    /// ComPare Y
    CPY,
    /// DECrement
    DEC,
    /// bitwise Exclusive OR
    EOR,

    //region Flag instructions
    /// CLear Carry
    CLC,
    /// SEt Carry
    SEC,
    /// CLear Interrupt mask
    CLI,
    /// SEt Interrupt mask
    SEI,
    /// CLear oVerflow
    CLV,
    /// CLear Decimal
    CLD,
    /// SEt Decimal
    SED,
    //endregion
    /// INCrement memory
    INC,
    /// JuMP
    ///
    /// # Note on a major CPU bug
    ///
    /// The 6502 had a serious bug with indirect absolute indexing and the
    /// JMP instruction. If the operand crosses a page boundary, the 6502 will
    /// 'forget' the carry and instead use the 00 byte on that page.
    ///
    /// TODO: Implement that bug
    JMP,
    /// Jump to SubRoutine
    JSR,
    /// LoaD Acc
    LDA,
    /// LoaD X
    LDX,
    /// LoaD Y
    LDY,
    /// Logical Shift Right
    LSR,
    /// No OPeration
    NOP,
    /// bitwise OR with Acc
    ORA,

    //region Register Instructions
    /// Transfer A to X
    TAX,
    /// Transfer X to A
    TXA,
    /// DEcrement X
    DEX,
    /// INcrement X
    INX,
    /// Transfer A to Y
    TAY,
    /// Transfer Y to A
    TYA,
    /// DEcrement Y
    DEY,
    /// INcrement Y
    INY,
    //endregion

    //region Rotation instructions
    // Note: Rotation actually includes the Carry bit in rotation operations. So
    // if you rotate 0b1100_0000 left, and C is not asserted, you will get
    // 0b1000_0000 instead of 0b1000_0001, and Carry will be asserted.
    // Early versions of the 6502 had a bad bug with these instructions, where
    // they would actually work as arithmetic shifts (ignoring Carry). This
    // was fixed long before the NES, and so this emulation doesn't implement
    // that bug.
    /// ROtate Left
    ROL,
    /// ROtate Right
    ROR,
    //endregion

    //region Returns
    /// ReTurn from Interrupt
    RTI,
    /// ReTurn from Subroutine
    RTS,
    //endregion
    /// SuBtract with Carry*
    SBC,

    //region Store instructions
    /// STore Acc
    STA,
    /// STore X
    STX,
    /// STore Y
    STY,
    //endregion

    //region Stack instructions
    /// Transfer X to Stack
    TXS,
    /// Transfer Stack to X
    TSX,
    /// PusH Acc
    PHA,
    /// PuLl Acc
    PLA,
    /// PusH Processor status
    PHP, // or, the dreaded spawn of Rasmus Lerdorf
    /// PuLl Processor status
    PLP,
    //endregion
}

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

pub const POWERON_CPU_STATE: CpuState = CpuState {
    acc: 0,
    x: 0,
    y: 0,
    stack: 0xFD,
    pc: 0,
    status: Status::from_bits_truncate(0x24),
    tot_cycles: 7,
    instruction: 0xEA,
    addr: 0,
    addr_mode: AddressingMode::Impl,
    instr: Instruction::NOP,
};
