/// The interface for a NES Cart.
///
/// Cartridges are a complex topic, since they can implement everything from
/// bank switching to coprocessing and audio augmentation. This class offers
/// a simple interface for writing cartridge implementations, which will
/// generally be based on the mapper behind them.
///
/// Addresses are given in CPU format (for PRG) or PPU format (for CHR). This
/// means they're the same address as what the program is trying to load.
pub trait Cartridge {
    fn read_chr(&self, addr: u16) -> u8;

    fn write_chr(&mut self, addr: u16, data: u8);

    fn read_prg(&self, addr: u16) -> u8;

    fn write_prg(&mut self, addr: u16, data: u8);
}

/// The simplest possible sort of cartridge
pub struct NesMapper0Cart {
    prg_rom: Vec<u8>,
    chr_rom: Vec<u8>,
    is_16k: bool,
}

impl Cartridge for NesMapper0Cart {
    fn read_prg(&self, addr: u16) -> u8 {
        if addr < 0x8000 {
            return 0; // open bus
        }
        // 16k prg roms are mirrored
        let addr = if self.is_16k { addr & 0xBFFF } else { addr };
        self.prg_rom[(addr - 0x8000) as usize]
    }

    fn write_prg(&mut self, _addr: u16, _data: u8) {
        // do nothing
    }

    fn read_chr(&self, addr: u16) -> u8 {
        if addr > 0x2000 {
            // open bus
            return 0;
        }
        self.chr_rom[addr as usize]
    }

    fn write_chr(&mut self, _addr: u16, _data: u8) {
        // do nothing
    }
}

// Ideally we'd have a Cartridge that does this...
impl NesMapper0Cart {
    #[cfg(not(target = "wasm32"))]
    pub fn from_file(path: &str) -> std::io::Result<NesMapper0Cart> {
        use std::fs::File;
        use std::io::prelude::*;
        use std::path::Path;
        let path = Path::new(&path);
        let mut file = File::open(path).expect("Could not read rom");

        let mut header_bytes = [0u8; 16];
        file.read_exact(&mut header_bytes)?;

        let header = INesHeader::from_bytes(header_bytes);

        let mut prg_rom = vec![0; 0x4000 * header.prg_size as usize];
        let mut chr_rom = vec![0; 0x2000 * header.chr_size as usize];

        file.read_exact(&mut prg_rom[..])?;
        file.read_exact(&mut chr_rom[..])?;

        Result::Ok(NesMapper0Cart {
            prg_rom,
            chr_rom,
            is_16k: header.prg_size == 1,
        })
    }
}

/// Interface for an iNES header
/// ...which should go somewhere else...
#[allow(dead_code)] // make clippy less viscerally unhappy
struct INesHeader {
    /// The size of the PRG chunk, in 16k chunks. Will not be 0.
    prg_size: u8,
    /// The size of the CHR chunk, in 8k chunks. Will not be 0.
    chr_size: u8,
    // TODO: Flag support
    /// Mapper, mirroring, battery, trainer
    flags_6: u8,
    /// Mapper, VS/PlayChoice, NES 2.0 indicator
    flags_7: u8,
    /// PRG-RAM size, rarely used.
    flags_8: u8,
    /// NTSC/PAL, rarely used
    flags_9: u8,
    /// NTSC/PAL (again?!?), PRG-RAM (again!?!), also rarely used
    flags_10: u8,
}

impl INesHeader {
    fn from_bytes(bytes: [u8; 16]) -> INesHeader {
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
