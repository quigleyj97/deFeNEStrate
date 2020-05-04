//! Helper structs for emulation

#[derive(Debug, PartialEq)]
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

/// Enum for the instructions implemented by this emulator
///
/// *depends on BCD flag, not currently supported
#[derive(Debug, PartialEq)]
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
