use super::structs::{
    PpuAddressPart, PpuControlFlags, PpuControlPorts, PpuMaskFlags, PpuOamAttributes,
    PpuOamByteOffsets, PpuState, PpuStatusFlags, PALLETE_TABLE, PPU_POWERON_STATE,
};
use super::utils;
use crate::devices::bus::{ppu_memory_map, BusDevice, BusPeekResult};
use crate::devices::cartridge::{self, WithCartridge};
use crate::state;

const PPU_NAMETABLE_START_ADDR: u16 = 0x2000;
const PPU_NAMETABLE_END_ADDR: u16 = 0x3EFF;
const PPU_NAMETABLE_MASK: u16 = 0x0FFF;
const PPU_PALETTE_START_ADDR: u16 = 0x3F00;
const PPU_PALETTE_END_ADDR: u16 = 0x3FFF;
const PPU_PALETTE_MASK: u16 = 0x001F;
//  _____________________________________
// / I am 0x3-CO, you probably didn't    \
// \ recognize me because of the red arm /
//  -------------------------------------
//    \
//     \
//        /!\
//       |oo )
//       _\=/_
//      /     \
//     //|/.\|\\
//    ||  \_/  ||
//    || |\ /| ||
//     # \_ _/  #
//       | | |
//       | | |
//       []|[]
//       | | |
//      /_]_[_\
const ATTR_TABLE_OFFSET: u16 = 0x3C0;

/// A trait for a device that owns a PPU, such as the NES Motherboard
pub trait WithPpu {
    /// Get an immutable reference to the PPU
    fn ppu(&self) -> &Ppu2C02;
    /// Get a mutable reference to the PPU
    fn ppu_mut(&mut self) -> &mut Ppu2C02;
}

pub struct Ppu2C02 {
    /** The internal palette memory */
    palette: PpuPaletteRam,
    state: PpuState,
}

impl Ppu2C02 {
    pub fn new() -> Ppu2C02 {
        let palette = PpuPaletteRam::new();
        let state = PPU_POWERON_STATE;
        Ppu2C02 { palette, state }
    }

    /** Whether a VBlank NMI has occured. This should be plumbed to the CPU. */
    pub fn is_vblank(&self) -> bool {
        self.state.vblank_nmi_ready
    }

    /** Acknowledge the vblank NMI, so that the PPU stops asserting it */
    pub fn ack_vblank(&mut self) {
        self.state.vblank_nmi_ready = false;
    }

    /** Whether the PPU has completely rendered a frame. */
    pub fn is_frame_ready(&self) -> bool {
        self.state.frame_ready
    }

    /** Retrieve a slice of the current frame */
    pub fn get_buffer(&self) -> &[u8] {
        &self.state.frame_data
    }

    /** Write a byte to the OAM
     *
     * This is intended for OAM-DMA
     */
    pub fn write_oam(&mut self, addr: u8, data: u8) {
        self.state.oam[addr as usize] = data;
    }

    pub fn dump_palettes(&self) -> &[u8] {
        &self.palette.palette_buffer
    }

    /** Returns true if rendering is enabled and the PPU is in the visible region */
    fn is_rendering(&self) -> bool {
        return (self.state.mask & (PpuMaskFlags::BG_ENABLE | PpuMaskFlags::SPRITE_ENABLE).bits())
            > 0
            && self.state.scanline > -1
            && self.state.scanline < 240;
    }
}

/** Read data from a control port on the PPU.
 *
 * Addresses should be given in CPU Bus addresses (eg, $PPUCTRL)
 */
pub fn control_port_read<T: WithPpu + WithCartridge>(mb: &mut T, port_addr: u16) -> u8 {
    match port_addr + 0x2000 {
        PpuControlPorts::PPUSTATUS => {
            let status = state!(get status, mb)
                | (PpuStatusFlags::STATUS_IGNORED.bits() & state!(get last_control_port_value, mb));
            state!(set status, mb, state!(get status, mb) &
                0xFF & !(PpuStatusFlags::VBLANK | PpuStatusFlags::STATUS_IGNORED).bits());
            state!(set w, mb, false);
            state!(set vblank_nmi_ready, mb, false);
            state!(set last_control_port_value, mb, status);
            return status;
        }
        PpuControlPorts::OAMDATA => {
            // TODO: OAMDATA reads, like OAMADDR writes, also corrupt OAM
            return state!(get oam, mb)[state!(get oam_addr, mb) as usize];
        }
        PpuControlPorts::PPUDATA => {
            // For most addresses, we need to buffer the response in internal
            // memory, since the logic for PPUDATA reads isn't actually
            // combinatorial and requires some plumbing (except for palette
            // memory, which is spe
            let addr = mb.ppu().state.v;

            if !mb.ppu().is_rendering() {
                if (0xFF
                    & (state!(get control, mb) & PpuControlFlags::VRAM_INCREMENT_SELECT.bits()))
                    != 0
                {
                    state!(set v, mb, 0x7FFF & (state!(get v, mb) + 32));
                } else {
                    state!(set v, mb, 0x7FFF & (state!(get v, mb) + 1));
                }
            } else {
                eprintln!(" [INFO] Read from PPUDATA during render");
                // Since we're writing during rendering, the PPU will
                // increment both the coarse X and fine Y due to how the
                // PPU is wired
                inc_coarse_x(mb);
                inc_fine_y(mb);
            }
            if port_addr >= 0x3F00 {
                // This is palette memory, don't buffer...
                //
                // ......ish...
                //
                // According to Nesdev, the PPU actually _will_ populate the
                // buffer with whatever's in the nametable, mirrored though
                // 0x3F00. So let's do that after setting data, just in case
                // anything needs that...
                let data = read(mb, addr);
                let buffer = read(mb, addr & 0x0FFF);
                state!(set ppudata_buffer, mb, buffer);
                state!(set last_control_port_value, mb, data);
                return data;
            }
            let buffer = read(mb, addr);
            let data = state!(get ppudata_buffer, mb);
            state!(set ppudata_buffer, mb, buffer);
            state!(set last_control_port_value, mb, data);
            return data;
        }
        _ => mb.ppu().state.last_control_port_value,
    }
}

/** Write data to a control port on the PPU.
 *
 * Addresses should be given in CPU Bus addresses (eg, $PPUCTRL)
 */
pub fn control_port_write<T: WithPpu + WithCartridge>(mb: &mut T, port_addr: u16, data: u8) {
    mb.ppu_mut().state.last_control_port_value = data;
    match port_addr + 0x2000 {
        // TODO: pre-boot cycle check
        // TODO: simulate immediate NMI hardware bug
        // TODO: Bit 0 race condition
        // TODO: Complain loudly when BG_COLOR_SELECT is set
        // The exact writes to T and V come from NESDEV documentation on
        // how the internal PPU registers work:
        // https://wiki.nesdev.com/w/index.php/PPU_scrolling
        PpuControlPorts::PPUCTRL => {
            let ppu = mb.ppu_mut();
            state!(set control, mb, data);
            state!(and t, mb,                 0x7FFF & !(PpuAddressPart::NAMETABLE_X | PpuAddressPart::NAMETABLE_Y).bits());
            state!(or t, mb, ((data & PpuControlFlags::NAMETABLE_BASE_SELECT.bits()) as u16) << 10);
            return;
        }
        PpuControlPorts::PPUMASK => {
            let ppu = mb.ppu_mut();
            state!(set mask, mb, data);
            return;
        }
        PpuControlPorts::OAMADDR => {
            // TODO: OAMADDR writes corrupt the OAM in particular ways, which
            // I might need to implement
            let ppu = mb.ppu_mut();
            state!(set oam_addr, mb, data);
            return;
        }
        PpuControlPorts::OAMDATA => {
            // TODO: OAMDATA writes, like OAMADDR writes, also corrupt OAM
            let ppu = mb.ppu_mut();
            let oam_addr = state!(get oam_addr, mb) as usize;
            state!(set_arr oam, oam_addr, mb, data);
            return;
        }
        PpuControlPorts::PPUSCROLL => {
            let ppu = mb.ppu_mut();
            if !state!(get w, mb) {
                state!(set x, mb, data & 0x07);
                state!(and t, mb, 0xFFFF & !PpuAddressPart::COARSE_X.bits());
                state!(or t, mb, ((data as u16) >> 3) & PpuAddressPart::COARSE_X.bits());
                state!(set w, mb, true);
            } else {
                state!(and t, mb,                     0xFFFF & (!(PpuAddressPart::FINE_Y | PpuAddressPart::COARSE_Y).bits()));
                state!(or t, mb, ((0x07 & (data as u16)) << 12) | (((data as u16) & 0xF8) << 2));
                state!(set w, mb, false);
            }
            return;
        }
        PpuControlPorts::PPUADDR => {
            let ppu = mb.ppu_mut();
            if !state!(get w, mb) {
                state!(and t, mb, 0x00FF);
                state!(or t, mb, ((data as u16) & 0x3F) << 8);
                state!(set w, mb, true);
            } else {
                state!(and t, mb, 0xFF00);
                state!(or t, mb, data as u16);
                state!(set v, mb, state!(get t, mb));
                state!(set w, mb, false);
            }
            return;
        }
        PpuControlPorts::PPUDATA => {
            write(mb, mb.ppu().state.v, data);
            let ppu = mb.ppu_mut();
            if !ppu.is_rendering() {
                if (state!(get control, mb) & PpuControlFlags::VRAM_INCREMENT_SELECT.bits()) > 0 {
                    state!(set v, mb, 0x7FFF & (state!(get v, mb) + 32));
                } else {
                    state!(set v, mb, 0x7FFF & (state!(get v, mb) + 1));
                }
            } else {
                eprintln!(" [INFO] Write to PPUDATA during render");
                // Since we're writing during rendering, the PPU will
                // increment both the coarse X and fine Y due to how the
                // PPU is wired
                inc_coarse_x(mb);
                inc_fine_y(mb);
            }
            return;
        }
        _ => unreachable!("Invalid PPU control port: ${:04X}", port_addr),
    };
}

/// Read from the PPU bus
fn read<T: WithPpu + WithCartridge>(mb: &mut T, addr: u16) -> u8 {
    let (device, addr) = ppu_memory_map::match_addr(addr);
    let last_bus_value = mb.ppu().state.last_bus_value;
    let response = match device {
        ppu_memory_map::Device::CartridgeOrNametable => {
            mb.cart_mut().read_chr(addr, last_bus_value)
        }
        ppu_memory_map::Device::PaletteRAM => mb.ppu_mut().palette.read(addr, last_bus_value),
        _ => last_bus_value,
    };
    mb.ppu_mut().state.last_bus_value = response;
    return response;
}

fn write<T: WithPpu + WithCartridge>(mb: &mut T, addr: u16, data: u8) {
    let (device, addr) = ppu_memory_map::match_addr(addr);
    mb.ppu_mut().state.last_bus_value = data;
    match device {
        ppu_memory_map::Device::CartridgeOrNametable => mb.cart_mut().write_chr(addr, data),
        ppu_memory_map::Device::PaletteRAM => mb.ppu_mut().palette.write(addr, data),
        _ => {}
    }
}

/** Clock the PPU, rendering to the internal framebuffer and modifying state as appropriate */
pub fn clock<T: WithPpu + WithCartridge>(mb: &mut T) {
    if mb.ppu().state.scanline < 240 || mb.ppu().state.scanline == 261 {
        //#region Background evaluation
        if (mb.ppu().state.pixel_cycle >= 1 && mb.ppu().state.pixel_cycle < 258)
            || (mb.ppu().state.pixel_cycle > 320 && mb.ppu().state.pixel_cycle < 337)
        {
            update_shift_regs(mb);
            let CHR_BANK =
                ((mb.ppu().state.control & PpuControlFlags::BG_TILE_SELECT.bits()) as u16) << 8;
            match (mb.ppu().state.pixel_cycle - 1) % 8 {
                0 => {
                    transfer_registers(mb);
                    mb.ppu_mut().state.temp_nt_byte =
                        read(mb, PPU_NAMETABLE_START_ADDR | (mb.ppu().state.v & 0x0FFF));
                }
                2 => {
                    // self.state addressing comes from NESDEV:
                    // https://wiki.nesdev.com/w/index.php/PPU_scrolling#Tile_and_attribute_fetching
                    mb.ppu_mut().state.temp_at_byte = read(
                        mb,
                        PPU_NAMETABLE_START_ADDR
                            | ATTR_TABLE_OFFSET
                            | (mb.ppu().state.v & 0x0C00)
                            | ((mb.ppu().state.v >> 4) & 0x38)
                            | ((mb.ppu().state.v >> 2) & 0x07),
                    );
                    if (((mb.ppu().state.v & PpuAddressPart::COARSE_Y.bits()) >> 5) & 0x02) > 0 {
                        mb.ppu_mut().state.temp_at_byte >>= 4;
                    }
                    if ((mb.ppu().state.v & PpuAddressPart::COARSE_X.bits()) & 0x02) > 0 {
                        mb.ppu_mut().state.temp_at_byte >>= 2;
                    }
                    mb.ppu_mut().state.temp_at_byte &= 3;
                }
                4 => {
                    mb.ppu_mut().state.temp_bg_lo_byte = read(
                        mb,
                        CHR_BANK
                            | ((mb.ppu().state.temp_nt_byte as u16) << 4)
                            | ((mb.ppu().state.v & PpuAddressPart::FINE_Y.bits()) >> 12),
                    );
                }
                6 => {
                    mb.ppu_mut().state.temp_bg_hi_byte = read(
                        mb,
                        CHR_BANK
                            | ((mb.ppu().state.temp_nt_byte as u16) << 4)
                            | ((mb.ppu().state.v & PpuAddressPart::FINE_Y.bits()) >> 12)
                            | 8,
                    );
                }
                7 => {
                    inc_coarse_x(mb);
                }
                _ => {
                    // no-op- we're waiting on a read or doing something else
                }
            }
        }
        if state!(get pixel_cycle, mb) == 337 || state!(get pixel_cycle, mb) == 339 {
            // make a dummy read of the nametable bit
            // self.state is important, since some mappers like MMC3 use self.state to
            // clock a scanline counter
            read(mb, PPU_NAMETABLE_START_ADDR | (state!(get v, mb) & 0x0FFF));
        }
        //#endregion

        //#region Sprite evaluation
        // I'm cheating here, technically the sprite evaluation is pipelined
        // just like the background, but I'm gonna implement that later
        if state!(get pixel_cycle, mb) == 258 {
            // clear the secondary OAM
            state!(set secondary_oam, mb, [0xFFu8; 64]);
            let mut n_sprites = 0;
            let mut byte_addr = 0;
            for sprite in (state!(get oam_addr, mb) / 4)..64 {
                let diff =
                    state!(get scanline, mb) - (state!(get oam, mb)[(sprite * 4) as usize] as i16);
                let diff_cmp =
                    if state!(get control, mb) & PpuControlFlags::SPRITE_MODE_SELECT.bits() > 0 {
                        16
                    } else {
                        8
                    };
                if diff >= 0 && diff < (diff_cmp) {
                    // self.state sprite is visible
                    n_sprites += 1;
                    if n_sprites == 8 {
                        // TODO: Sprite Overflow bug
                        // for now self.state is an incorrectly correct setup
                        state!(or status, mb, PpuStatusFlags::SPRITE_OVERFLOW.bits());
                        break;
                    }
                    for i in 0u8..4u8 {
                        mb.ppu_mut().state.secondary_oam[((n_sprites - 1) * 4 + i) as usize] =
                            state!(get oam, mb)[(sprite * 4 + i) as usize];
                    }
                }
            }
            // prepare the shifters for rendering
            for i in 0..n_sprites {
                let tile_addr = (((state!(get control, mb) & PpuControlFlags::SPRITE_TILE_SELECT.bits()) as u16) << 9)
                            // +1 = tile id
                        | ((state!(get secondary_oam, mb)[(i * 4 + 1) as usize] as u16) << 4)
                        | ((state!(get scanline, mb) as u16) - (state!(get secondary_oam, mb)[(i * 4) as usize] as u16));
                state!(set_arr sprite_tile_lo_shift_regs, i, mb, read(mb, tile_addr));
                state!(set_arr sprite_tile_hi_shift_regs, i, mb, read(mb, tile_addr + 8));
            }
        }
        //#endregion

        //#region Address increments
        if state!(get pixel_cycle, mb) == 256 {
            inc_fine_y(mb);
        }
        if state!(get pixel_cycle, mb) == 257 {
            transfer_x_addr(mb);
        }
        // self.state is the pre-render scanline, it has some special handling
        if state!(get scanline, mb) == 261 {
            if state!(get pixel_cycle, mb) == 1 {
                state!(and status, mb, 0xFF
                    & !(PpuStatusFlags::SPRITE_0_HIT
                        | PpuStatusFlags::SPRITE_OVERFLOW
                        | PpuStatusFlags::VBLANK)
                        .bits());
            }
            if state!(get pixel_cycle, mb) >= 280 || state!(get pixel_cycle, mb) < 305 {
                transfer_y_addr(mb);
            }
        }
        //#endregion
    }
    // check if we need to set the vblank flag
    let nmi_enabled = (state!(get control, mb) & PpuControlFlags::VBLANK_NMI_ENABLE.bits()) > 0;
    if state!(get scanline, mb) == 241 && state!(get pixel_cycle, mb) == 0 {
        state!(set vblank_nmi_ready, mb, nmi_enabled);
        if (nmi_enabled) {
            panic!("panik")
        } else {
        } // kalm
        state!(or status, mb, PpuStatusFlags::VBLANK.bits());
    }
    // self.state is a true render scanline
    if state!(get scanline, mb) < 240
        && state!(get pixel_cycle, mb) > 3
        && state!(get scanline, mb) < 257
    {
        // interestingly enough, pixel output doesn't begin until cycle _4_.
        // self.state comes from NESDEV:
        // https://wiki.nesdev.com/w/index.php/NTSC_video
        //#region Background rendering
        let mut bg_pixel = 0x00;
        let mut bg_palette = 0x00;

        if (state!(get mask, mb) & PpuMaskFlags::BG_ENABLE.bits()) > 0 {
            let bit_mux = 0x8000 >> state!(get x, mb);
            let pattern_hi = if (state!(get bg_tile_hi_shift_reg, mb) & bit_mux) > 0 {
                1
            } else {
                0
            };
            let pattern_lo = if (state!(get bg_tile_lo_shift_reg, mb) & bit_mux) > 0 {
                1
            } else {
                0
            };
            bg_pixel = (pattern_hi << 1) | pattern_lo;
            let palette_hi = if ((state!(get bg_attr_hi_shift_reg, mb) as u16) & bit_mux) > 0 {
                1
            } else {
                0
            };
            let palette_lo = if ((state!(get bg_attr_lo_shift_reg, mb) as u16) & bit_mux) > 0 {
                1
            } else {
                0
            };
            bg_palette = (palette_hi << 1) | palette_lo;
        }
        //#endregion

        //#region Sprite rendering
        let mut sprite_pixel = 0x00;
        let mut sprite_palette = 0x00;
        let mut sprite_priority = false;
        let mut is_sprite0_rendered = false;

        if (state!(get mask, mb) & PpuMaskFlags::SPRITE_ENABLE.bits()) > 0 {
            for i in 0..8 {
                // self.state sprite is active, use the shifters
                if state!(get secondary_oam, mb)[(i * 4 + PpuOamByteOffsets::X_POS.bits()) as usize]
                    == 0
                {
                    if i == 0 {
                        is_sprite0_rendered = true;
                    }
                    let pattern_hi = state!(get sprite_tile_hi_shift_regs, mb)[i as usize] & 0x80;
                    let pattern_lo = state!(get sprite_tile_lo_shift_regs, mb)[i as usize] & 0x80;
                    sprite_pixel = (pattern_hi << 1) | pattern_lo;
                    let attr = state!(get secondary_oam, mb)
                        [(i * 4 + PpuOamByteOffsets::ATTR.bits()) as usize];
                    // add 0x04 since the sprites use the last 4 palettes
                    sprite_palette = (attr & PpuOamAttributes::PALLETE.bits()) + 0x04;
                    sprite_priority = attr & PpuOamAttributes::BACKGROUND_PRIORITY.bits() > 0;
                    if sprite_pixel != 0 {
                        // we're done, a non-transparent sprite pixel has been selected
                        break;
                    }
                }
            }
        }
        //#endregion

        //#region Compositing
        let mut pixel = bg_pixel;
        let mut palette = bg_palette;
        if sprite_pixel != 0 {
            if bg_pixel == 0 {
                // use the sprite
                pixel = sprite_pixel;
                palette = sprite_palette;
            } else {
                // we need to sort out priority
                if !sprite_priority {
                    pixel = sprite_pixel;
                    palette = sprite_palette;
                }
                // then test for sprite0 hits
                if is_sprite0_rendered {
                    if (state!(get mask, mb) & PpuMaskFlags::BG_ENABLE.bits() > 0)
                        && (state!(get mask, mb) & PpuMaskFlags::SPRITE_ENABLE.bits() > 0)
                    {
                        state!(or status, mb, PpuStatusFlags::SPRITE_0_HIT.bits());
                    }
                }
            }
        }
        let color = read(
            mb,
            PPU_PALETTE_START_ADDR
                | (if pixel == 0x00 {
                    0u16
                } else {
                    ((palette as u16) << 2) | (pixel as u16)
                }),
        ) as u16;
        let idx = (state!(get scanline, mb) as u16) * 256 + state!(get pixel_cycle, mb);
        for i in 0..3 {
            state!(set_arr frame_data, idx * 3 + i, mb, PALLETE_TABLE[(color * 3 + i) as usize]);
        }
    //#endregion
    } else if state!(get pixel_cycle, mb) < 4 {
        let idx = (state!(get scanline, mb) as u16) * 256 + state!(get pixel_cycle, mb);
        let color = read(mb, PPU_PALETTE_START_ADDR) as u16;
        for i in 0..3 {
            // fill with black for now
            // technically self.state should actually be the background color
            state!(set_arr frame_data, (idx * 3 + i) as usize, mb, PALLETE_TABLE[(color * 3 + i) as usize]);
        }
    }
    state!(add pixel_cycle, mb, 1);

    if state!(get pixel_cycle, mb) > 340 {
        state!(set pixel_cycle, mb, 0);
        state!(add scanline, mb, 1);
    }

    state!(set frame_ready, mb, false);

    if state!(get scanline, mb) > 261 {
        // The "0" scanline is special, and rendering should handle it differently
        state!(set scanline, mb, 0);
        state!(set frame_ready, mb, true);
    }
}

/** Increment the coarse X register */
fn inc_coarse_x<T: WithPpu>(mb: &mut T) {
    if (state!(get mask, mb) & (PpuMaskFlags::BG_ENABLE | PpuMaskFlags::SPRITE_ENABLE).bits()) == 0
    {
        return;
    }
    if (state!(get v, mb) & PpuAddressPart::COARSE_X.bits()) == 31 {
        // clear the coarse X and invert the X nametable
        state!(and v, mb, 0xFFFF & !PpuAddressPart::COARSE_X.bits());
        state!(xor v, mb, PpuAddressPart::NAMETABLE_X.bits());
    } else {
        // increment coarse X directly
        state!(add v, mb, 1);
    }
}

/** Increment the fine Y register */
fn inc_fine_y<T: WithPpu>(mb: &mut T) {
    if (state!(get mask, mb) & (PpuMaskFlags::BG_ENABLE | PpuMaskFlags::SPRITE_ENABLE).bits()) == 0
    {
        return;
    }
    if (state!(get v, mb) & PpuAddressPart::FINE_Y.bits()) != 0x7000 {
        // if the fine Y is less than 7, we can increment it directly
        state!(add v, mb, 0x1000);
    } else {
        // clear fine Y and attempt to increment coarse Y
        state!(and v, mb, 0xFFFF & !PpuAddressPart::FINE_Y.bits());
        let mut new_y = (state!(get v, mb) & PpuAddressPart::COARSE_Y.bits()) >> 5;
        if new_y == 29 {
            // flip nametables
            new_y = 0;
            state!(xor v, mb, PpuAddressPart::NAMETABLE_Y.bits());
        } else if new_y == 31 {
            // a weird quirk of the PPU is that it allows setting coarse Y
            // out-of-bounds. When the coarse Y increments to 31 (where it
            // would overflow), the PPU doesn't switch the nametable. This
            // is, in effect, a "negative" scroll value of sorts.
            new_y = 0;
        } else {
            new_y += 1;
        }
        state!(and v, mb, 0xFFFF & !PpuAddressPart::COARSE_Y.bits());
        state!(or v, mb, new_y << 5);
    }
}

fn transfer_registers<T: WithPpu>(mb: &mut T) {
    let ppu = mb.ppu_mut();
    state!(set bg_tile_lo_shift_reg, mb,         (state!(get bg_tile_lo_shift_reg, mb) & 0xFF00) | (state!(get temp_bg_lo_byte, mb) as u16));
    state!(set bg_tile_hi_shift_reg, mb,         (state!(get bg_tile_hi_shift_reg, mb) & 0xFF00) | (state!(get temp_bg_hi_byte, mb) as u16));
    state!(set bg_attr_latch, mb, state!(get temp_at_byte, mb));
    state!(and bg_attr_lo_shift_reg, mb, 0x0);
    state!(or bg_attr_lo_shift_reg, mb, 0xFF * (state!(get bg_attr_latch, mb) & 0x01));
    state!(and bg_attr_hi_shift_reg, mb, 0x0);
    state!(or bg_attr_hi_shift_reg, mb, 0xFF * ((state!(get bg_attr_latch, mb) & 0x02) >> 1));
}

fn update_shift_regs<T: WithPpu>(mb: &mut T) {
    if state!(get mask, mb) & PpuMaskFlags::BG_ENABLE.bits() > 0 {
        state!(set bg_tile_hi_shift_reg, mb, 0xFFFF & state!(get bg_tile_hi_shift_reg, mb) << 1);
        state!(set bg_tile_lo_shift_reg, mb, 0xFFFF & state!(get bg_tile_lo_shift_reg, mb) << 1);
        state!(set bg_attr_lo_shift_reg, mb, 0xFF & state!(get bg_attr_lo_shift_reg, mb) << 1);
        state!(set bg_attr_hi_shift_reg, mb, 0xFF & state!(get bg_attr_hi_shift_reg, mb) << 1);
    }
    if (state!(get mask, mb) & PpuMaskFlags::SPRITE_ENABLE.bits() > 0)
        && state!(get pixel_cycle, mb) >= 1
        && state!(get pixel_cycle, mb) < 258
    {
        for i in 0..8 {
            let idx = i * 4 + PpuOamByteOffsets::X_POS.bits() as usize;
            if state!(get secondary_oam, mb)[idx] > 0 {
                state!(set_arr secondary_oam, idx, mb, state!(get secondary_oam, mb)[idx].wrapping_sub(1));
            } else {
                state!(shl_arr sprite_tile_hi_shift_regs, i, mb, 1);
                state!(shl_arr sprite_tile_lo_shift_regs, i, mb, 1);
            }
        }
    }
}

fn transfer_x_addr<T: WithPpu>(mb: &mut T) {
    let ppu = mb.ppu_mut();
    if (state!(get mask, mb) & (PpuMaskFlags::BG_ENABLE | PpuMaskFlags::SPRITE_ENABLE).bits()) == 0
    {
        return;
    }
    let X_ADDR_PART = PpuAddressPart::COARSE_X | PpuAddressPart::NAMETABLE_X;
    state!(and v, mb, 0xFFFF & !X_ADDR_PART.bits());
    state!(or v, mb, state!(get t, mb) & X_ADDR_PART.bits());
}

fn transfer_y_addr<T: WithPpu>(mb: &mut T) {
    let ppu = mb.ppu_mut();
    if (state!(get mask, mb) & (PpuMaskFlags::BG_ENABLE | PpuMaskFlags::SPRITE_ENABLE).bits()) == 0
    {
        return;
    }
    let Y_ADDR_PART =
        PpuAddressPart::FINE_Y | PpuAddressPart::NAMETABLE_Y | PpuAddressPart::COARSE_Y;
    state!(and v, mb, 0xFFFF & !Y_ADDR_PART.bits());
    state!(or v, mb, state!(get t, mb) & Y_ADDR_PART.bits());
}

/**
 * A helper for handling some of the odd PPU palette mirrors
 */
struct PpuPaletteRam {
    palette_buffer: [u8; 32],
}

impl PpuPaletteRam {
    fn new() -> PpuPaletteRam {
        PpuPaletteRam {
            palette_buffer: [0u8; 32],
        }
    }
}

impl BusDevice for PpuPaletteRam {
    fn read(&mut self, addr: u16, last_bus_value: u8) -> u8 {
        self.peek(addr).unwrap(last_bus_value)
    }
    fn peek(&self, addr: u16) -> BusPeekResult {
        let read_addr = match addr {
            0x10 => 0x00,
            0x14 => 0x04,
            0x18 => 0x08,
            0x1C => 0x0C,
            _ => addr,
        };
        return BusPeekResult::Result(self.palette_buffer[read_addr as usize]);
    }

    fn write(&mut self, addr: u16, data: u8) {
        // these sprite palette locations are actually mirrors into the bg colors
        let read_addr = match addr {
            0x10 => 0x00,
            0x14 => 0x04,
            0x18 => 0x08,
            0x1C => 0x0C,
            _ => addr,
        };
        self.palette_buffer[read_addr as usize] = data;
    }
}
