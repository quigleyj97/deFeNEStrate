pub struct PpuState {
    //#region Loopy registers
    // These registers represent internal registers that handle numerous
    // operations on the NES, such as PPUADDR addressing. The exact names
    // of these variables from Loopy's "The Skinny on NES Scrolling"
    /** The 15-bit VRAM address register */
    pub v: u16,
    /** The 15-bit temporary VRAM address register */
    pub t: u16,
    /** The 3-bit fine X scroll register */
    pub x: u8,
    /** The PPUADDR write latch */
    pub w: bool,
    //#endregion

    //#region Rendering shift registers
    // The PPU has a pair of shift registers for tile data, one for the high bit
    // and one for the low bit. It has another pair for the palette.
    // Sprites get their own shift registers and counters
    pub bg_tile_hi_shift_reg: u16,
    pub bg_tile_lo_shift_reg: u16,
    pub bg_attr_hi_shift_reg: u8,
    pub bg_attr_lo_shift_reg: u8,
    /** The 2-bit attribute for the next tile to render, which feeds the shift registers */
    pub bg_attr_latch: u8,
    // The 8 tile shift registers for the 8 sprites
    pub sprite_tile_hi_shift_regs: [u8; 8],
    pub sprite_tile_lo_shift_regs: [u8; 8],
    //#endregion

    //#region Byte buffers
    // The PPU reads various parts of the rendering data at different points in
    // a rendering lifecycle, and those are loaded into the registers at the end
    // of an 8-cycle period. Until then, they're held in temporary registers,
    // which the below variables model
    pub temp_nt_byte: u8,
    pub temp_at_byte: u8,
    pub temp_bg_lo_byte: u8,
    pub temp_bg_hi_byte: u8,
    pub temp_oam_byte: u8,
    //#endregion

    //#region PPU Control Registers
    // These are registers that are exposed to the CPU bus, like $PPUSTATUS and
    // $PPUMASK
    /** The $PPUCTRL register */
    pub control: u8,
    /** The $PPUMASK register */
    pub mask: u8,
    /** The $PPUSTATUS register */
    pub status: u8,
    //#endregion

    //#region Emulation helpers
    /** The OAM address byte */
    pub oam_addr: u8,
    /** The secondary OAM address, used for sprite evaluation */
    pub secondary_oam_addr: u8,
    /** The  */
    /** The internal OAM memory */
    pub oam: Vec<u8>,
    /** The secondary OAM used for sprite evaluation */
    pub secondary_oam: Vec<u8>,
    /** The pixel currently being output by the PPU. */
    pub pixel_cycle: u16,
    /** The scanline currently being rendered. */
    pub scanline: i16,
    /** Whether the PPU has completed a frame */
    pub frame_ready: bool,
    /** The internal framebuffer containing the rendered image, in u8 RGB */
    pub frame_data: Vec<u8>,
    /** Whether a VBlank interrupt has occured */
    pub vblank_nmi_ready: bool,
    /**
     * Buffer containing the value of the address given in PPUADDR.
     *
     * # Note
     *
     * Reads from regions of PPU memory (excluding the palette memory) are
     * delayed by one clock cycle, as the PPU first _recieves_ the address,
     * then puts that address on it's internal bus. On the _next_ cycle, it
     * then _writes_ that value to a buffer on the CPU bus. The effect of this
     * is that reads from the PPU take _two_ cycles instead of one.
     *
     * For palette memory, however, there happens to be entirely combinatorial
     * logic to plumb this read; meaning that no clock ticking has to occur.
     * _however_, reads will still populate the buffer! Except with name
     */
    pub ppudata_buffer: u8,
    /** The last value put on a PPU control port */
    pub last_control_port_value: u8,
    //#endregion
}

const PPU_POWERON_STATE: PpuState = PpuState {
    v: 0,
    t: 0,
    x: 0,
    w: false,
    oam_addr: 0,
    secondary_oam_addr: 0,
    bg_tile_hi_shift_reg: 0,
    bg_tile_lo_shift_reg: 0,
    bg_attr_hi_shift_reg: 0,
    bg_attr_lo_shift_reg: 0,
    bg_attr_latch: 0,
    sprite_tile_hi_shift_regs: [0u8; 8],
    sprite_tile_lo_shift_regs: [0u8; 8],
    ppudata_buffer: 0,
    temp_nt_byte: 0,
    temp_bg_hi_byte: 0,
    temp_bg_lo_byte: 0,
    temp_at_byte: 0,
    temp_oam_byte: 0,
    control: 0,
    mask: 0,
    // magic constant given from NESDEV for PPU poweron state
    status: 0xA0,
    oam: vec![0u8, 256],
    secondary_oam: vec![0u8, 64],
    pixel_cycle: 0,
    scanline: 0,
    frame_ready: false,
    frame_data: vec![0u8, 240 * 256 * 3],
    vblank_nmi_ready: false,
    last_control_port_value: 0,
};

bitflags! {
    /** Bitmasks for various components of a PPU register address */
    pub struct PpuAddressPart: u16 {
        const COARSE_X = 0x001F;
        const COARSE_Y = 0x03E0;
        const NAMETABLE_X = 0x0400;
        const NAMETABLE_Y = 0x0800;
        const FINE_Y = 0x700;
    }
}

bitflags! {
    /** Bitmasks for fields of the PPU control register ($PPUCTRL) */
    pub struct PpuControlFlags: u8 {
        /// Select which nametable to use. 0 = $2000, 1 = $2400, 2 = $2800, 3 = $2C00
        const NAMETABLE_BASE_SELECT = 0x03;
        /// Select the increment mode for writes to $PPUDATA. 0 = add 1, 1 = add 32
        const VRAM_INCREMENT_SELECT = 0x04;
        /// Select the base address for sprite tiles. 0 = $0000, 1 = $1000
        const SPRITE_TILE_SELECT = 0x08;
        /// Select the base address for background tiles. 0 = $0000, 1 = $1000
        const BG_TILE_SELECT = 0x10;
        /// If 1, use 8x16 sprites instead of the usual 8x8
        const SPRITE_MODE_SELECT = 0x20;
        /// If 1, use the PPU's EXT pins to source the background color
        /// Note: This is not used in the NES since the EXT pins of the 2C02 are
        /// grounded (and thus enabling this bit will cause a ground fault on real
        /// hardware). Nesdev referrs to this flag as the "PPU master/slave select",
        /// Presumably this comes from the PPU's internal documentation.
        const PPU_BG_COLOR_SELECT = 0x40;
        /// If 1, enable NMI generation on VBlank
        const VBLANK_NMI_ENABLE = 0x80;
    }
}

bitflags! {
    /// Bitmasks for the PPU mask register ($PPUMASK)
    pub struct PpuMaskFlags: u8 {
        /// If true, use the leftmost pallete colors only
        const USE_GRAYSCALE = 0x01;
        /// If false, don't render the background in the leftmost 8 columns
        const BG_LEFT_ENABLE = 0x02;
        /// If false, don't render sprites in the leftmost 8 columns
        const SPRITE_LEFT_ENABLE = 0x04;
        /// If false, don't render the background
        const BG_ENABLE = 0x08;
        /// If false, don't render sprites
        const SPRITE_ENABLE = 0x10;
        const COLOR_EMPHASIS_RED = 0x20;
        const COLOR_EMPHASIS_GREEN = 0x40;
        const COLOR_EMPHASIS_BLUE = 0x80;
    }
}

bitflags! {
    /// Bitmasks for the PPU status register ($PPUSTATUS)
    pub struct PpuStatusFlags: u8 {
        const STATUS_IGNORED = 0x1F;
        const SPRITE_OVERFLOW = 0x20;
        const SPRITE_0_HIT = 0x40;
        const VBLANK = 0x80;
    }
}

bitflags! {
    /// Constants for the CPU addresses of PPU control ports
    pub struct PpuControlPorts: u16 {
        /// Write-only PPU control register
        const PPUCTRL = 0x2000;
        /// PPU mask register
        const PPUMASK = 0x2001;
        /// Read-only PPU status register
        const PPUSTATUS = 0x2002;
        /// Latch to set the address for OAMDATA into the PPU's OAM memory
        const OAMADDR = 0x2003;
        /// The value to be written into OAM
        const OAMDATA = 0x2004;
        /// Write-twice latch for setting the scroll position
        const PPUSCROLL = 0x2005;
        /// Write-twice latch for setting the address for the PPUDATA latch
        const PPUADDR = 0x2006;
        /// Read-write port for interfacing with the PPU bus
        const PPUDATA = 0x2007;
        /// Address for setting up OAM
        const OAMDMA = 0x4014;
    }
}

bitflags! {
    pub struct PpuOamAttributes: u8 {
        const PALLETE = 0x03;
        const UNUSED = 0x1C;
        const BACKGROUND_PRIORITY = 0x20;
        const FLIP_HORI = 0x40;
        const FLIP_VERT = 0x80;
    }
}

bitflags! {
    pub struct PpuOamByteOffsets: u8 {
        const Y_POS = 0;
        const TILE = 1;
        const ATTR = 2;
        const X_POS = 3;
    }
}

/// Palette table taken from NesDev
///
/// To index, multiply the color index by 3 and take the next 3 values in memory
/// as an (R,G,B) 8-byte triplet
#[rustfmt::skip]
pub const PALLETE_TABLE: Vec<u8> = vec![
    //          0*
    /* *0 */    101, 101, 101, 
    /* *1 */    0, 45, 105, 
    /* *2 */    19, 31, 127, 
    /* *3 */    60, 19, 124, 
    /* *4 */    96, 11, 98, 
    /* *5 */    115, 10, 55,
    /* *6 */    113, 15, 7, 
    /* *7 */    90, 26, 0, 
    /* *8 */    52, 40, 0, 
    /* *9 */    11, 52, 0,
    /* *A */    0, 60, 0, 
    /* *B */    0, 61, 16, 
    /* *C */    0, 56, 64,
    /* *D */    0, 0, 0, 
    /* *E */    0, 0, 0, 
    /* *F */    0, 0, 0,

    //          1*    
    /* *0 */    174, 174, 174, 
    /* *1 */    15, 99, 179, 
    /* *2 */    64,81, 208, 
    /* *3 */    120, 65, 204, 
    /* *4 */    167, 54, 169, 
    /* *5 */    192, 52, 112,
    /* *6 */    189, 60, 48, 
    /* *7 */    159, 74, 0, 
    /* *8 */    109, 92, 0,
    /* *9 */    54, 109, 0, 
    /* *A */    7, 119, 4, 
    /* *B */    0, 121, 61, 
    /* *C */    0, 114, 125, 
    /* *D */    0, 0, 0, 
    /* *E */    0, 0, 0, 
    /* *F */    0, 0, 0,

    //          2*
    /* *0 */    254, 254, 255, 
    /* *1 */    93, 179, 255, 
    /* *2 */    143, 161, 255, 
    /* *3 */    200, 144, 255, 
    /* *4 */    247, 133, 250, 
    /* *5 */    255, 131, 192, 
    /* *6 */    255, 139, 127, 
    /* *7 */    239, 154, 73, 
    /* *8 */    189, 172, 44,
    /* *9 */    133, 188, 47, 
    /* *A */    85, 199, 83, 
    /* *B */    60, 201, 140,
    /* *C */    62, 194, 205, 
    /* *D */    78, 78, 78, 
    /* *E */    0, 0, 0, 
    /* *F */    0, 0, 0, 
    
    //          3*
    /* *0 */    254, 254, 255, 
    /* *1 */    188, 223, 255, 
    /* *2 */    209, 216, 255,
    /* *3 */    232, 209, 255, 
    /* *4 */    251, 205, 253, 
    /* *5 */    255, 204, 229,
    /* *6 */    255, 207, 202, 
    /* *7 */    248, 213, 180, 
    /* *8 */    228, 220, 168,
    /* *9 */    204, 227, 169, 
    /* *A */    185, 232, 184, 
    /* *B */    174, 232, 208,
    /* *C */    175, 229, 234, 
    /* *D */    182, 182, 182, 
    /* *E */    0, 0, 0,
    /* *F */    0, 0, 0,
];
