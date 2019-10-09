use super::cartridge::Cartridge;
use std::cell::RefCell;
use std::rc::Rc;

pub struct Ppu2C02 {
    cart: Rc<RefCell<Option<Box<dyn Cartridge>>>>,
    nametable: [u8; 1024],
    palette: [u8; 32],
}

impl Ppu2C02 {
    pub fn dump_chr(&self) -> Box<[u8; 8192]> {
        let mut buf = Box::new([0u8; 8192]);
        let cart_ref = self.cart.borrow();
        let cart = (*cart_ref).as_ref().unwrap();

        for addr in 0..8192 {
            buf[addr] = cart.read_chr(addr as u16)
        }

        buf
    }

    pub fn dump_pattern_table(&self) -> Box<[u8; 256 * 128 * 3]> {
        let mut buf = Box::new([0u8; 256 * 128 * 3]);

        let cart_ref = self.cart.borrow();
        let cart = (*cart_ref).as_ref().unwrap();

        for r in 0..256 {
            for c in 0..128 {
                // How the address is calculated:
                // RR = (r / 8) represents the first 2 nibbles of our address,
                // C = (c / 8) represents the third.
                // c = The fourth comes from the actual pixel row, ie, r % 8.
                // eg, 0xRRCr
                let addr = (r / 8 * 0x100) + (r % 8) + (c / 8) * 0x10; //((r / 8) << 8) | ((c / 8) << 4) | (r % 8);
                let lo = cart.read_chr(addr);
                let hi = cart.read_chr(addr + 8);
                // Now to pull the column, we shift right by c mod 8.
                let offset = 7 - (c % 8);
                let color = ((1 & (hi >> offset)) << 1) | (1 & (lo >> offset));
                // This algorithm isn't true color, but it's
                // not really possible to be accurate anyway since CHR tiles
                // have no explicit color (that is defined by the pallete
                // pairing in the nametable, which is a separate step and allows
                // CHR tiles to be reused)
                let color = match color {
                    0b00 => 0x00,                       // black
                    0b01 => 0x7C,                       // dark gray
                    0b10 => 0xBC,                       // light gray
                    0b11 => 0xF8,                       // aaalllllmooosst white,
                    _ => panic!("Invalid color index"), // I screwed up
                };
                buf[(u32::from(r * 128) * 3 + u32::from(c) * 3) as usize] = color;
                buf[(u32::from(r * 128) * 3 + u32::from(c) * 3 + 1) as usize] = color;
                buf[(u32::from(r * 128) * 3 + u32::from(c) * 3 + 2) as usize] = color;
            }
        }

        buf
    }

    pub fn poke_chr(&mut self, addr: u16, data: u8) {
        let mut cart_ref = self.cart.borrow_mut();
        match &mut *cart_ref {
            Option::None => {
                eprintln!("Cannot POKE null cart");
            }
            Option::Some(cart) => cart.write_chr(addr, data),
        }
    }

    // Statics

    pub fn new(cart: Rc<RefCell<Option<Box<dyn Cartridge>>>>) -> Ppu2C02 {
        Ppu2C02 {
            cart,
            nametable: [0u8; 1024],
            palette: [0u8; 32],
        }
    }
}
