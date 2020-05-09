//! Helper structs for emulation

pub mod cpu {
    /// A struct holding state information about a 6502 CPU.
    ///
    /// This struct is held internally, but can be copied to power to things
    /// like debug formatters and, if taken at the end of a simulation cycle,
    /// serialization.
    #[derive(Debug, Copy, PartialEq)]
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

    impl CpuState {
        /// Create a new CpuState
        ///
        /// # Note
        ///
        /// Default values are the NES power-up vals
        /// cf. http://wiki.nesdev.com/w/index.php/CPU_power_up_state
        pub fn new() -> CpuState {
            CpuState {
                acc: 0,
                x: 0,
                y: 0,
                stack: 0xFD,
                pc: 0xC000,
                // IRQ disabled
                // Unwrapping b/c this is a constant and should be OK
                status: Status::from_bits(0x24).unwrap(),

                // internal state
                tot_cycles: 7,
                instruction: 0xEA,
                addr: 0,
                addr_mode: AddressingMode::Impl,
                instr: Instruction::NOP,
            }
        }
    }

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
}

pub mod ppu {
    /// Bitmasks for fields of the PPU control register ($PPUCTRL)
    pub mod ppu_ctrl_flags {
        /// Select which nametable to use. 0 = $2000, 1 = $2400, 2 = $2800, 3 = $2C00
        pub const NAMETABLE_BASE_SELECT: u8 = 0x03;
        /// Select the increment mode for writes to $PPUDATA. 0 = add 1, 1 = add 32
        pub const VRAM_INCREMENT_SELECT: u8 = 0x04;
        /// Select the base address for sprite tiles. 0 = $0000, 1 = $1000
        pub const SPRITE_TILE_SELECT: u8 = 0x08;
        /// Select the base address for background tiles. 0 = $0000, 1 = $1000
        pub const BG_TILE_SELECT: u8 = 0x10;
        /// If 1, use 8x16 sprites instead of the usual 8x8
        pub const SPRITE_MODE_SELECT: u8 = 0x20;
        /// If 1, use the PPU's EXT pins to source the background color
        /// Note: This is not used in the NES since the EXT pins of the 2C02 are
        /// grounded (and thus enabling this bit will cause a ground fault on real
        /// hardware). Nesdev referrs to this flag as the "PPU master/slave select",
        /// Presumably this comes from the PPU's internal documentation.
        pub const PPU_BG_COLOR_SELECT: u8 = 0x40;
        /// If 1, enable NMI generation on VBlank
        pub const VBLANK_NMI_ENABLE: u8 = 0x80;
    }

    /// Bitmasks for the PPU mask register ($PPUMASK)
    pub mod ppu_mask_flags {
        /// If true, use the leftmost pallete colors only
        pub const USE_GRAYSCALE: u8 = 0x01;
        /// If false, don't render the background in the leftmost 8 columns
        pub const BG_LEFT_ENABLE: u8 = 0x02;
        /// If false, don't render sprites in the leftmost 8 columns
        pub const SPRITE_LEFT_ENABLE: u8 = 0x04;
        /// If false, don't render the background
        pub const BG_ENABLE: u8 = 0x08;
        /// If false, don't render sprites
        pub const SPRITE_ENABLE: u8 = 0x10;
        pub const COLOR_EMPHASIS_RED: u8 = 0x20;
        pub const COLOR_EMPHASIS_GREEN: u8 = 0x40;
        pub const COLOR_EMPHASIS_BLUE: u8 = 0x80;
    }

    /// Bitmasks for the PPU status register ($PPUSTATUS)
    pub mod ppu_status_flags {
        pub const STATUS_IGNORED: u8 = 0x1F;
        pub const SPRITE_OVERFLOW: u8 = 0x20;
        pub const SPRITE_0_HIT: u8 = 0x40;
        pub const VBLANK: u8 = 0x80;
    }

    /// Constants for the CPU addresses of PPU control ports
    pub mod ppu_port {
        /// Write-only PPU control register
        pub const PPUCTRL: u16 = 0x2000;
        /// PPU mask register
        pub const PPUMASK: u16 = 0x2001;
        /// Read-only PPU status register
        pub const PPUSTATUS: u16 = 0x2002;
        /// Latch to set the address for OAMDATA into the PPU's OAM memory
        pub const OAMADDR: u16 = 0x2003;
        /// The value to be written into OAM
        pub const OAMDATA: u16 = 0x2004;
        /// Write-twice latch for setting the scroll position
        pub const PPUSCROLL: u16 = 0x2005;
        /// Write-twice latch for setting the address for the PPUDATA latch
        pub const PPUADDR: u16 = 0x2006;
        /// Read-write port for interfacing with the PPU bus
        pub const PPUDATA: u16 = 0x2007;
        /// Address for setting up OAM
        pub const OAMDMA: u16 = 0x4014;
    }

    /// Palette table taken from NesDev
    ///
    /// To index, multiply the color index by 3 and take the next 3 values in memory
    /// as an (R,G,B) 8-byte triplet
    #[rustfmt::skip]
    pub const PALLETE_TABLE: [u8; 0x40 * 3] = [
        //          0*              1*              2*              3*
        /* *0 */    101, 101, 101,  174, 174, 174,  254, 254, 255,  254, 254, 255, // White
        /* *1 */    0, 45, 105,     15,  99,  179,  93,  179, 255,  188, 223, 255, // Blue
        /* *2 */    19, 31, 127,    64, 81, 208,    143, 161, 255,  209, 216, 255,
        /* *3 */    60, 19, 124,    120, 65, 204,   200, 144, 255,  232, 209, 255,
        /* *4 */    96, 11, 98,     167, 54, 169,   247, 133, 250,  251, 205, 253,
        /* *5 */    115, 10, 55,    192, 52, 112,   255, 131, 192,  255, 204, 229,
        /* *6 */    113, 15, 7,     189, 60, 48,    255, 139, 127,  255, 207, 202, // Red
        /* *7 */    90, 26, 0,      159, 74, 0,     239, 154, 73,   248, 213, 180,
        /* *8 */    52, 40, 0,      109, 92, 0,     189, 172, 44,   228, 220, 168,
        /* *9 */    11, 52, 0,      54, 109, 0,     133, 188, 47,   204, 227, 169,
        /* *A */    0, 60, 0,       7, 119, 4,      85, 199, 83,    185, 232, 184, // Green
        /* *B */    0, 61, 16,      0, 121, 61,     60, 201, 140,   174, 232, 208,
        /* *C */    0, 56, 64,      0, 114, 125,    62, 194, 205,   175, 229, 234,
        /* *D */    0, 0, 0,        0, 0, 0,        78, 78, 78,     182, 182, 182, // White
        /* *E */    0, 0, 0,        0, 0, 0,        0, 0, 0,        0, 0, 0,       // Black
        /* *F */    0, 0, 0,        0, 0, 0,        0, 0, 0,        0, 0, 0
    ];
}
