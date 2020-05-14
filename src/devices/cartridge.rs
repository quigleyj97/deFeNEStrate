use crate::devices::bus::BusDevice;
use crate::utils::ines::INesHeader;

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

pub const CART_START_ADDR: u16 = 0x4020;
pub const CART_PPU_START_ADDR: u16 = 0x2000;

pub struct CartCpuBridge<T: Cartridge + ?Sized> {
    cart: *mut T,
}

impl<T: Cartridge + ?Sized> CartCpuBridge<T> {
    pub fn new(cart: *const T) -> CartCpuBridge<T> {
        CartCpuBridge {
            cart: cart as *mut T,
        }
    }
}

impl<T: Cartridge + ?Sized> BusDevice for CartCpuBridge<T> {
    fn read(&mut self, addr: u16) -> u8 {
        unsafe { (*self.cart).read_prg(addr + CART_START_ADDR) }
    }

    fn write(&mut self, addr: u16, data: u8) {
        unsafe {
            (*self.cart).write_prg(addr + CART_START_ADDR, data);
        }
    }
}

pub struct CartPpuBridge<T: Cartridge + ?Sized> {
    cart: *mut T,
}

impl<T: Cartridge + ?Sized> CartPpuBridge<T> {
    pub fn new(cart: *const T) -> CartPpuBridge<T> {
        CartPpuBridge {
            cart: cart as *mut T,
        }
    }
}

impl<T: Cartridge + ?Sized> BusDevice for CartPpuBridge<T> {
    fn read(&mut self, addr: u16) -> u8 {
        unsafe { (*self.cart).read_chr(addr + CART_PPU_START_ADDR) }
    }

    fn write(&mut self, addr: u16, data: u8) {
        unsafe {
            (*self.cart).write_chr(addr + CART_PPU_START_ADDR, data);
        }
    }
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

    pub fn of_zeros() -> NesMapper0Cart {
        NesMapper0Cart {
            prg_rom: vec![0; 0x4000],
            chr_rom: vec![0; 0x2000],
            is_16k: true,
        }
    }
}
