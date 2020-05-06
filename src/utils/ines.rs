//! Helpers for parsing iNES ROM files

/// Interface for an iNES header
pub struct INesHeader {
    /// The size of the PRG chunk, in 16k chunks. Will not be 0.
    pub prg_size: u8,
    /// The size of the CHR chunk, in 8k chunks. Will not be 0.
    pub chr_size: u8,
    // TODO: Flag support
    /// Mapper, mirroring, battery, trainer
    pub flags_6: u8,
    /// Mapper, VS/PlayChoice, NES 2.0 indicator
    pub flags_7: u8,
    /// PRG-RAM size, rarely used.
    pub flags_8: u8,
    /// NTSC/PAL, rarely used
    pub flags_9: u8,
    /// NTSC/PAL (again?!?), PRG-RAM (again!?!), also rarely used
    pub flags_10: u8,
}

impl INesHeader {
    pub fn from_bytes(bytes: [u8; 16]) -> INesHeader {
        // First 4 bytes are the "NES" magic string, last 5 are unused.
        INesHeader {
            prg_size: if bytes[4] == 0 { 1 } else { bytes[4] },
            chr_size: if bytes[5] == 0 { 1 } else { bytes[5] },
            flags_6: bytes[6],
            flags_7: bytes[7],
            flags_8: bytes[8],
            flags_9: bytes[9],
            flags_10: bytes[10],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inesheader_decodes_correctly() {
        let ines_header_data = [
            0x0, 0x0, 0x0, 0x0, 0x0, 0x1, 0x2, 0x3, 0x4, 0x5, 0x6, 0x0, 0x0, 0x0, 0x0, 0x0,
        ];

        let header = INesHeader::from_bytes(ines_header_data);
        assert_eq!(header.prg_size, 1, "PRG size mismatch");
        assert_eq!(header.chr_size, 1, "CHR size mismatch");
        assert_eq!(header.flags_6, 2, "Flags6 mismatch");
        assert_eq!(header.flags_7, 3, "Flags7 mismatch");
        assert_eq!(header.flags_8, 4, "Flags8 mismatch");
        assert_eq!(header.flags_9, 5, "Flags9 mismatch");
        assert_eq!(header.flags_10, 6, "Flags10 mismatch");
    }
}
