use super::cartridge::Cartridge;
use std::cell::RefCell;
use std::rc::Rc;

pub struct Ppu2C02 {
    cart: Rc<RefCell<Option<Box<dyn Cartridge>>>>,
    /// The PPU nametables
    ///
    /// TODO: This should live inside the Cartridge, as the mapper implementation
    /// has a high degree of control over this region of memory.
    nametable: [u8; 0x800],
    /// The internal palette memory
    palette: [u8; 0x20],
    /// The write-only control register
    control: u8,
    /// The mask register used for controlling various aspects of rendering
    mask: u8,
    /// The read-only status register
    status: u8,
    /// The last value on the PPU bus.
    ///
    /// The PPU's bus to the CPU has such long traces that electrically, they
    /// act as a latch, retaining the value of last value placed on the bus for
    /// up to a full frame.
    ///
    /// It should be said that this behavior is unreliable, and no reasonable
    /// game would ever depend on this functionality.
    last_bus_value: u8,
}

impl Ppu2C02 {
    /// Read data from a control port on the PPU.
    ///
    /// Addresses should be given in CPU Bus addresses (eg, $PPUCTRL)
    pub fn read_ppu(&mut self, addr: u16) -> u8 {
        let val = match addr {
            ppu_port::PPUSTATUS => {
                let status = self.status | (ppu_status_flags::STATUS_IGNORED & self.last_bus_value);
                self.status &= !(ppu_status_flags::VBLANK & ppu_status_flags::STATUS_IGNORED);
                // TODO: Clear PPUADDR/PPUSCROLL register
                status
            }
            ppu_port::OAMDATA => {
                eprintln!(" [WARN] $OAMDATA not implemented");
                self.last_bus_value
            }
            _ => self.last_bus_value,
        };
        self.last_bus_value = val;
        val
    }

    /// Write data to a control port on the PPU
    pub fn write_ppu(&mut self, addr: u16, data: u8) {
        match addr {
            // TODO: pre-boot cycle check
            // TODO: simulate immediate NMI hardware bug
            // TODO: Bit 0 race condition
            // TODO: Complain loudly when BG_COLOR_SELECT is set
            ppu_port::PPUCTRL => self.control = data,
            ppu_port::PPUMASK => self.mask = data,
            ppu_port::OAMADDR => {
                eprintln!(" [WARN] $OAMADDR not implemented");
            }
            ppu_port::OAMDATA => {
                eprintln!(" [WARN] $OAMDATA not implemented");
            }
            ppu_port::PPUSCROLL => {
                eprintln!(" [WARN] $PPUSCROLL not implemented");
            }
            ppu_port::PPUADDR => {
                eprintln!(" [WARN] $PPUADDR not implemented");
            }
            ppu_port::PPUDATA => {
                eprintln!(" [WARN] $PPUDATA not implemented");
            }
            _ => {}
        };
    }

    //region Debug aids
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
    //endregion

    pub fn read(&self, addr: u16) -> u8 {
        if addr < 0x2000 {
            // read from cart
            let cart_ref = self.cart.borrow();
            return match (*cart_ref).as_ref() {
                Option::None => 0, // TODO: Open bus
                Option::Some(cart) => cart.read_chr(addr),
            };
        } else if addr < 0x3F00 {
            // Mirroring occurs over 0x3000..0x3EFF -> 0x2000..0x2EFF
            // Functionally this means that $3EFF = $(0x3EFF & !0x1000) = $2EFF
            let addr = ((addr & !0x1000) - 0x2000) % 0x0800;
            // TODO: Mirroring
            return self.nametable[addr as usize];
        } else if addr < 0x4000 {
            let addr = addr & 0x1F;
            let addr = match addr {
                0x10 => 0x00,
                0x14 => 0x04,
                0x18 => 0x08,
                0x1C => 0x0C,
                _ => addr,
            };
            return self.palette[addr as usize];
        }

        // TODO: Open bus
        0
    }

    pub fn write(&mut self, addr: u16, data: u8) {
        if addr < 0x2000 {
            // read from cart
            let mut cart_ref = self.cart.borrow_mut();
            match &mut *cart_ref {
                Option::None => {} // TODO: Open bus
                Option::Some(cart) => cart.write_chr(addr, data),
            };
        } else if addr < 0x3F00 {
            // Mirroring occurs over 0x3000..0x3EFF -> 0x2000..0x2EFF
            // Functionally this means that $3EFF = $(0x3EFF & !0x1000) = $2EFF
            let addr = ((addr & !0x1000) - 0x2000) % 0x0800;
            // TODO: Mirroring
            self.nametable[addr as usize] = data;
        } else if addr < 0x4000 {
            let addr = addr & 0x1F;
            let addr = match addr {
                0x10 => 0x00,
                0x14 => 0x04,
                0x18 => 0x08,
                0x1C => 0x0C,
                _ => addr,
            };
            self.palette[addr as usize] = data;
        }
    }

    // Statics

    pub fn new(cart: Rc<RefCell<Option<Box<dyn Cartridge>>>>) -> Ppu2C02 {
        Ppu2C02 {
            cart,
            nametable: [0u8; 2048],
            palette: [0u8; 32],
            control: 0,
            mask: 0,
            status: 0,
            last_bus_value: 0,
        }
    }
}

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
