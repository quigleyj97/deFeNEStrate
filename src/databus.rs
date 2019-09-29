pub trait Bus {
    /// Read from the Bus at the given address.
    fn read(&self, addr: u16) -> u8;

    /// Write to the Bus at the given address.
    fn write(&mut self, addr: u16, value: u8);
}
