//! Helpers for parsing iNES ROM files

/// Interface for an iNES header
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct INesHeader {
    /// The size of the PRG chunk, in 16k chunks. Will not be 0.
    pub prg_size: usize,
    /// The size of the CHR chunk, in 8k chunks. Will not be 0.
    pub chr_size: usize,
    // TODO: Flag support
    /// Mapper, mirroring, battery, trainer
    pub flags_6: INesFlags6,
    /// Mapper, VS/PlayChoice, NES 2.0 indicator
    pub flags_7: INesFlags7,
    /// PRG-RAM size, rarely used.
    pub flags_8: u8,
    /// NTSC/PAL, rarely used
    pub flags_9: u8,
    /// NTSC/PAL (again?!?), PRG-RAM (again!?!), also rarely used
    pub flags_10: u8,
}

/** Given the first 16 bytes, parse out an iNES header */
pub fn parse_ines_header(bytes: &[u8]) -> INesHeader {
    // the first 4 bytes of the header are the null-terminated string "NES"
    // the last 5 bytes are unused in iNES 1.0
    INesHeader {
        prg_size: if bytes[4] == 0 { 1 } else { bytes[4] as usize },
        chr_size: if bytes[5] == 0 { 1 } else { bytes[5] as usize },
        flags_6: INesFlags6::from_bits_truncate(bytes[6]),
        flags_7: INesFlags7::from_bits_truncate(bytes[7]),
        flags_8: bytes[8],
        flags_9: bytes[9],
        flags_10: bytes[10],
    }
}

bitflags! {
    pub struct INesFlags6: u8 {
        /** The mirroring mode.
         *
         * If 0, use horizontal (vertical arrangement) mirroring
         * If 1, use vertical (horizontal arrangement) mirroring.
         *
         * Note that some mappers (like MMC3) ignore this setting, and it only
         * applies to cartridges where the mirroring is set in hardware (such as
         * NROM).
         */
        const MIRRORING = 0x01;
        /** Whether this rom contains a battery-backed RAM */
        const HAS_PERSISTENT_MEMORY = 0x02;
        /** Whether this ROM contains a 512-bit trainer program.
         *
         * Note: This emulator does not support trainers
         */
        const HAS_TRAINER = 0x04;
        /** Whether to use 4-screen VRAM instead of mirroring */
        const USE_FOUR_SCREEN_VRAM = 0x08;
        /** The lower nibble of the iNES mapper number */
        const LOWER_MAPPER_NIBBLE = 0xF0;
    }
}

bitflags! {
    pub struct INesFlags7: u8 {
        /** Whether this ROM was developed for the VS arcade */
        const VS_UNISYSTEM_ROM = 0x01;
        /** Whether this ROM was developed for the PlayChoice arcade.
         *
         * Note that this is rarely seen in the wild, but the presense of this bit
         * indicates that 8kb of hint screen data is included at the end of the
         * CHR section
         */
        const PLAYCHOICE_10 = 0x02;
        /** If equal to 10, the rest of this ROM's headers are in iNES 2.0 format. */
        const IS_INES_2_0 = 0x0C;
        /** The upper nibble of the iNES mapper number */
        const UPPER_MAPPER_NIBBLE = 0xF0;
    }
}

// todo: implement other flags as needed

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_parse_header() {
        const INES_HEADER_DATA: [u8; 16] = [
            0x0, 0x0, 0x0, 0x0, 0x0, 0x1, 0x2, 0x3, 0x4, 0x5, 0x6, 0x0, 0x0, 0x0, 0x0, 0x0,
        ];
        let header = parse_ines_header(&INES_HEADER_DATA);
        assert_eq!(header.prg_size, 1, "PRG size mismatch");
        assert_eq!(header.prg_size, 1, "PRG size mismatch");
        assert_eq!(header.chr_size, 1, "CHR size mismatch");
        assert_eq!(header.flags_6.bits(), 2, "Flags6 mismatch");
        assert_eq!(header.flags_7.bits(), 3, "Flags7 mismatch");
        assert_eq!(header.flags_8, 4, "Flags8 mismatch");
        assert_eq!(header.flags_9, 5, "Flags9 mismatch");
        assert_eq!(header.flags_10, 6, "Flags10 mismatch");
    }
}
