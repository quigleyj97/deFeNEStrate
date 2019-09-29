use std::fs::File;
use std::io::{SeekFrom, prelude::*};
use std::path::Path;

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
    prg_rom: Box<[u8]>,
    chr_rom: Box<[u8]>,
    is_16k: bool,
}

impl Cartridge for NesMapper0Cart {
    fn read_prg(&self, addr: u16) -> u8 {
        // 16k prg roms are mirrored
        let addr = if self.is_16k { addr & 0xBFFF } else { addr };
        self.prg_rom[(addr - 0x8000) as usize]
    }

    fn write_prg(&mut self, _addr: u16, _data: u8) {
        // do nothing
    }

    fn read_chr(&self, _addr: u16) -> u8 {
        0 // unimplemented
    }

    fn write_chr(&mut self, _addr: u16, _data: u8) {
        // do nothing
    }
}

// Ideally we'd have a Cartridge that does this...
impl NesMapper0Cart {
    pub fn from_file(path: &str) -> std::io::Result<NesMapper0Cart> {
        let path = Path::new(&path);
        let mut file = File::open(path).expect("Could not read NESTEST rom");
        file.seek(SeekFrom::Start(16)).unwrap(); // skip header

        let mut prg_rom = Box::new([0u8; 16_384]);
        let mut chr_rom = Box::new([0u8; 8192]);

        file.read_exact(&mut prg_rom[..])?;
        file.read_exact(&mut chr_rom[..])?;

        Result::Ok(NesMapper0Cart {
            prg_rom,
            chr_rom,
            is_16k: true
        })
    }
}
