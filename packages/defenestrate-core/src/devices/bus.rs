/// Trait for an object that owns an address bus
pub trait Motherboard {
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

#[derive(Debug, Eq, PartialEq)]
pub enum BusPeekResult {
    Unmapped,
    MutableRead,
    Result(u8),
}

impl BusPeekResult {
    /// Unwrap a BusPeekResult to an u8
    pub fn unwrap(&self, last_bus_value: u8) -> u8 {
        match self {
            BusPeekResult::Result(val) => *val,
            _ => last_bus_value,
        }
    }

    /// Convert a BusPeekResult into an Option<u8>
    pub fn to_optional(&self) -> Option<u8> {
        match self {
            BusPeekResult::Result(val) => Some(*val),
            _ => None,
        }
    }
}

/// Trait for an object that may be mounted to and driven by an address bus
pub trait BusDevice {
    /// Given a local address and the last bus value, return a new bus value
    ///
    /// At it's most basic level, this is a read- but some devices may handle
    /// this differently. For instance, the Controller port only writes a few
    /// bits to the bus- the rest are left at the same value as the last bus
    /// operation.
    fn read(&mut self, addr: u16, last_bus_value: u8) -> u8;

    /// Attempt to immutably and deterministically read from the bus
    fn peek(&self, addr: u16) -> BusPeekResult;

    /// Write to the device at the local address
    fn write(&mut self, addr: u16, value: u8);
}

pub struct Range {
    start: u16,
    end: u16,
    mask: u16,
}

impl Range {
    pub const fn new(start: u16, end: u16, mask: u16) -> Range {
        Range { start, end, mask }
    }

    pub const fn new_unmasked(start: u16, end: u16) -> Range {
        Range {
            start,
            end,
            mask: 0xFFFF,
        }
    }

    /// Given an address, return the local address or none if the global addr is outside this Range.
    pub fn map(&self, test_addr: u16) -> Option<u16> {
        if test_addr < self.start || test_addr > self.end {
            None
        } else {
            Some((test_addr - self.start) & self.mask)
        }
    }
}

pub mod cpu_memory_map {
    use super::Range;

    pub enum Device {
        Cartridge,
        RAM,
        Unmapped,
    }

    /// The Cartridge
    pub const Cartridge: Range = Range::new_unmasked(0x4020, 0xFFFF);

    /// The primary RAM
    pub const RAM: Range = Range::new(0x0000, 0x1FFF, 0x07FF);

    /// Given a test address, return a device and a local address
    ///
    /// If the address is unmapped, the returned address will be a global addr.
    pub fn match_addr(addr: u16) -> (Device, u16) {
        if let Some(addr) = Cartridge.map(addr) {
            (Device::Cartridge, addr)
        } else if let Some(addr) = RAM.map(addr) {
            (Device::RAM, addr)
        } else {
            (Device::Unmapped, addr)
        }
    }
}
