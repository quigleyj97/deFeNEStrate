pub trait BusDevice {
    /// Read from the bus at the given address, triggering any possible side-effects
    fn read(&mut self, addr: u16) -> u8;

    /// Attempt to determinisitcally read from the bus
    ///
    /// This should return None if such a read is not possible without
    /// side-effects or determinism (for instance, open bus reads or PPU control
    /// ports)
    fn peek(&self, addr: u16) -> Option<u8>;

    /// Write to the bus with the given data
    fn write(&mut self, addr: u16, data: u8);
}
