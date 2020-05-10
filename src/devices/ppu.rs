use crate::devices::bus::Bus;
use crate::devices::ram::Ram;
use crate::utils::structs::ppu::*;

const PPU_NAMETABLE_START_ADDR: u16 = 0x2000;
const PPU_NAMETABLE_END_ADDR: u16 = 0x4000;
const PPU_NAMETABLE_MASK: u16 = 0x0FFF;
const PPU_PALETTE_START_ADDR: u16 = 0x4000;
const PPU_PALETTE_END_ADDR: u16 = 0xFFFF;
const PPU_PALETTE_MASK: u16 = 0x001F;

pub struct Ppu2C02 {
    /// The PPU bus
    pub bus: Bus,
    /// The PPU nametables
    ///
    /// TODO: This should live inside the Cartridge, as the mapper implementation
    /// has a high degree of control over this region of memory.
    nametable: Ram,
    /// The internal palette memory
    palette: Ram,
    /// The write-only control register
    control: u8,
    /// The mask register used for controlling various aspects of rendering
    mask: u8,
    /// The read-only status register
    status: u8,

    //#region Emulation helpers
    /// The last value on the PPU bus.
    ///
    /// The PPU's bus to the CPU has such long traces that electrically, they
    /// act as a latch, retaining the value of last value placed on the bus for
    /// up to a full frame.
    ///
    /// It should be said that this behavior is unreliable, and no reasonable
    /// game would ever depend on this functionality.
    last_bus_value: u8,
    /// Whether the PPUADDR is filling the hi (false) or the lo byte (true).
    ///
    /// # Note
    ///
    /// Oddly, PPUADDR seems to be _big_ endian even though the rest of the NES
    /// is little endian. I'm not sure why this is.
    is_ppuaddr_lo: bool,
    /// The address loaded into PPUADDR
    ppuaddr: u16,
    /// Buffer containing the value of the address given in PPUADDR.
    ///
    /// # Note
    ///
    /// Reads from regions of PPU memory (excluding the palette memory) are
    /// delayed by one clock cycle, as the PPU first _recieves_ the address,
    /// then puts that address on it's internal bus. On the _next_ cycle, it
    /// then _writes_ that value to a buffer on the CPU bus. The effect of this
    /// is that reads from the PPU take _two_ cycles instead of one.
    ///
    /// For palette memory, however, there happens to be entirely combinatorial
    /// logic to plumb this read; meaning that no clock ticking has to occur.
    ppudata_buffer: u8,
    /// The pixel currently being output by the PPU.
    pixel_cycle: i16,
    /// The scanline currently being rendered.
    scanline: i16,
    /// Whether the PPU has completed a frame
    frame_ready: bool,
    /// Whether a VBlank interrupt has occured
    vblank_nmi_ready: bool,
    /// The internal framebuffer containing the rendered image, in u8 RGB
    frame_data: Box<[u8; 240 * 256 * 3]>,
    //#endregion
}

impl Ppu2C02 {
    //#region Statics
    pub fn new() -> Ppu2C02 {
        let bus = Bus::new();
        let nametable = Ram::new(0xF00);
        let palette = Ram::new(0x20);
        // TODO: Validate this
        bus.map_device(
            &nametable,
            PPU_NAMETABLE_START_ADDR,
            PPU_NAMETABLE_END_ADDR,
            PPU_NAMETABLE_MASK,
        );
        bus.map_device(
            &palette,
            PPU_PALETTE_START_ADDR,
            PPU_PALETTE_END_ADDR,
            PPU_PALETTE_MASK,
        );
        Ppu2C02 {
            bus,
            nametable,
            palette,
            control: 0,
            mask: 0,
            // cf NesDev PPU powerup state
            status: 0xA0,
            last_bus_value: 0,
            is_ppuaddr_lo: false,
            ppuaddr: 0,
            ppudata_buffer: 0,
            pixel_cycle: 0,
            scanline: 0,
            frame_ready: true,
            vblank_nmi_ready: false,
            frame_data: Box::new([0u8; 240 * 256 * 3]),
        }
    }
    //#endregion

    /// Clock the PPU, rendering to the internal framebuffer and modifying state
    ///
    /// TODO: Cycle count timing for the PPU memory operations
    pub fn clock(&mut self) {
        // Render a checkerboard pattern for now
        if self.scanline > -1 && self.scanline < 240 && self.pixel_cycle < 256 {
            let idx = (i32::from(self.scanline) * 256 + i32::from(self.pixel_cycle)) as usize;
            let x = (self.pixel_cycle / 8) as u16;
            let y = (self.scanline / 8) as u16;
            let tile = self.bus.read(PPU_NAMETABLE_START_ADDR + x * 32 + y);
            let color = if tile == 0x20 { 0 } else { 255 };
            for offset in 0..3 {
                self.frame_data[idx * 3 + offset] = color;
            }
        }
        let nmi_enabled = self.control & ppu_ctrl_flags::VBLANK_NMI_ENABLE > 0;
        if self.scanline == 241 && self.pixel_cycle == 0 {
            self.vblank_nmi_ready = nmi_enabled;
            self.status |= ppu_status_flags::VBLANK;
        } else if self.scanline == 262 && self.pixel_cycle == 1 {
            self.vblank_nmi_ready = false;
            self.status &= !(ppu_status_flags::VBLANK | ppu_status_flags::STATUS_IGNORED);
        }

        self.pixel_cycle += 1;

        if self.pixel_cycle > 340 {
            self.pixel_cycle = 0;
            self.scanline += 1;
        }

        self.frame_ready = false;

        if self.scanline > 260 {
            // The "-1" scanline is special, and rendering should handle it differently
            self.scanline = -1;
            self.frame_ready = true;
        }
    }

    /// Whether a VBlank NMI has occured. This should be plumbed to the CPU.
    pub fn is_vblank(&self) -> bool {
        self.vblank_nmi_ready
    }

    /// Acknowledge the vblank NMI, so that the PPU stops asserting it
    pub fn ack_vblank(&mut self) {
        self.vblank_nmi_ready = false;
    }

    /// Whether the PPU has completely rendered a frame.
    pub fn is_frame_ready(&self) -> bool {
        self.frame_ready
    }

    /// Retrieve a copy of the current frame.
    pub fn get_buffer(&mut self) -> Box<[u8; 240 * 256 * 3]> {
        let mut new_frame = Box::new([0u8; 240 * 256 * 3]);
        new_frame.copy_from_slice(&self.frame_data[..self.frame_data.len()]);
        self.frame_data = Box::new([0u8; 240 * 256 * 3]);
        new_frame
    }

    /// Read data from a control port on the PPU.
    ///
    /// Addresses should be given in CPU Bus addresses (eg, $PPUCTRL)
    pub fn read_ppu(&mut self, addr: u16) -> u8 {
        let val = match addr {
            ppu_port::PPUSTATUS => {
                let status = self.status | (ppu_status_flags::STATUS_IGNORED & self.last_bus_value);
                self.status &= !(ppu_status_flags::VBLANK | ppu_status_flags::STATUS_IGNORED);
                self.is_ppuaddr_lo = false;
                self.vblank_nmi_ready = false;
                status
            }
            ppu_port::OAMDATA => {
                eprintln!(" [WARN] $OAMDATA not implemented");
                self.last_bus_value
            }
            ppu_port::PPUDATA => {
                if addr >= 0x3F00 {
                    // This is palette memory, don't buffer...
                    //
                    // ......ish...
                    //
                    // According to Nesdev, the PPU actually _will_ populate the
                    // buffer with whatever's in the nametable, mirrored though
                    // 0x3F00. So let's do that after setting data, just in case
                    // anything needs that...
                    let data = self.read(self.ppuaddr);
                    self.ppudata_buffer = self.read(self.ppuaddr & !0x1000);
                    return data;
                }
                let data = self.ppudata_buffer;
                self.ppudata_buffer = self.read(self.ppuaddr);
                return data;
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
                if self.is_ppuaddr_lo {
                    self.is_ppuaddr_lo = false;
                    self.ppuaddr |= u16::from(data);
                } else {
                    self.is_ppuaddr_lo = true;
                    self.ppuaddr = u16::from(data) << 8;
                }
            }
            ppu_port::PPUDATA => {
                self.write(self.ppuaddr, data);
                if (self.control & ppu_ctrl_flags::VRAM_INCREMENT_SELECT) > 0 {
                    self.ppuaddr += 32;
                } else {
                    self.ppuaddr += 1;
                }
            }
            _ => {}
        };
    }

    //region Debug aids
    pub fn dump_palettes(&self) -> [u8; 128 * 2 * 3] {
        let mut buf = [0u8; 128 * 2 * 3];
        for c in 0..32 {
            let color = self.read(0x3F00 | c);
            let red = PALLETE_TABLE[usize::from(color) * 3];
            let green = PALLETE_TABLE[usize::from(color) * 3 + 1];
            let blue = PALLETE_TABLE[usize::from(color) * 3 + 2];
            for r in 0..4 {
                let idx = (c * 4 + r) as usize;
                buf[idx * 3] = red;
                buf[idx * 3 + 1] = green;
                buf[idx * 3 + 2] = blue;
                buf[(idx + 128) * 3] = red;
                buf[(idx + 128) * 3 + 1] = green;
                buf[(idx + 128) * 3 + 2] = blue;
            }
        }
        buf
    }
    //endregion

    pub fn read(&self, addr: u16) -> u8 {
        self.bus.read(addr)
    }

    pub fn write(&mut self, addr: u16, data: u8) {
        self.bus.write(addr, data);
    }
}
