use crate::devices::bus::BusPeekResult;

/// Trait for a cartridge device
///
/// Cartridges are attached to _both_ the PPU and CPU address busses, and thus
/// can't really use the IBusDevice interface
pub trait ICartridge {
    fn read_chr(&mut self, addr: u16, last_bus_value: u8) -> u8;

    fn peek_chr(&self, addr: u16) -> BusPeekResult;

    fn write_chr(&mut self, addr: u16, value: u8);

    fn read_prg(&mut self, addr: u16, last_bus_value: u8) -> u8;

    fn peek_prg(&self, addr: u16) -> BusPeekResult;

    fn write_prg(&mut self, addr: u16, value: u8);

    fn dump_chr(&self) -> &[u8];

    fn dump_nametables(&self) -> &[u8];
}

/// A trait for devices that own a Cartridge
pub trait WithCartridge {
    /// Get a reference to a cartridge
    fn cart(&self) -> &Box<dyn ICartridge>;

    /// Get a mutable reference to a cartridge
    fn cart_mut(&mut self) -> &mut Box<dyn ICartridge>;
}
