pub trait Bus {
    /// Read from the Bus at the given address.
    fn read(&mut self, addr: u16) -> u8;

    /// A side-effect free bus read for debugging and formatting.
    ///
    /// Note that some addresses and behaviors may be inaccessible from this
    /// function
    fn read_debug(&self, addr: u16) -> u8;

    /// Write to the Bus at the given address.
    fn write(&mut self, addr: u16, value: u8);
}
