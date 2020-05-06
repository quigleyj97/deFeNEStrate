//! A set of helpers for memory-mapped address buses. Multiple devices can
//! connect to a single device, and define what addresses they respond to.
//!
//! Devices recieve 0-indexed addresses; that is, it doesn't matter where on
//! the address space they're mapped, the addresses will be the same and start
//! from 0.

/// A generic interface for devices that can be driven by the bus.
pub trait BusDevice {
    /// Read a byte from the device.
    ///
    /// This is _mutable_ because some devices may actually change state in
    /// response to a _read_ (like the PPU control registers)
    fn read(&mut self, addr: u16) -> u8;

    /// Write to the device
    fn write(&mut self, addr: u16, data: u8);
}

struct MemoryMappedDevice {
    dev: Box<dyn BusDevice>,
    /// The start of this mapped range
    addr_start: u16,
    /// The end of this mapped range
    addr_end: u16,
    /// A mask to apply to all addresses to implement mirroring.
    ///
    /// ### Note
    ///
    /// This mask is applied after subtracting addr_start from the address,
    /// before passing that address on to the device. Therefore, if your device
    /// is mounted on $2000 - $20FF, and the first 8 addresses are to be
    /// mirrored, then the mask should be 0x0007.
    addr_mask: u16,
}

impl MemoryMappedDevice {
    fn mapped_read(&mut self, addr: u16) -> Option<u8> {
        if addr < self.addr_start || addr > self.addr_end {
            return Option::None;
        }
        Option::Some(self.dev.read((addr - self.addr_start) & self.addr_mask))
    }

    fn mapped_write(&mut self, addr: u16, data: u8) -> Option<()> {
        if addr < self.addr_start || addr > self.addr_end {
            return Option::None;
        }
        self.dev
            .write((addr - self.addr_start) & self.addr_mask, data);
        Option::Some(())
    }

    fn new(
        dev: Box<dyn BusDevice>,
        addr_start: u16,
        addr_end: u16,
        addr_mask: u16,
    ) -> MemoryMappedDevice {
        MemoryMappedDevice {
            dev,
            addr_start,
            addr_end,
            addr_mask,
        }
    }
}

/// A data bus for 16-bit address spaces
#[derive(Default)]
pub struct Bus {
    /// The list of devices currently mounted to the bus.
    ///
    /// Note that there is no ordering to this bus- most-frequently-used devices
    /// should be added first, for performance reasons.
    devices: Vec<MemoryMappedDevice>,
    /// The last value put on the bus.
    ///
    /// This is memorized to emulate electrical effects of open bus conditions.
    last_bus_val: u8,
}

impl Bus {
    pub fn map_device(
        &mut self,
        dev: Box<dyn BusDevice>,
        addr_start: u16,
        addr_end: u16,
        addr_mask: u16,
    ) {
        self.devices.push(MemoryMappedDevice::new(
            dev, addr_start, addr_end, addr_mask,
        ));
    }

    pub fn read(&mut self, addr: u16) -> u8 {
        for dev in &mut self.devices {
            match dev.mapped_read(addr) {
                Option::Some(data) => {
                    self.last_bus_val = data;
                    return data;
                }
                Option::None => {}
            };
        }
        self.last_bus_val
    }

    pub fn write(&mut self, addr: u16, data: u8) {
        self.last_bus_val = data;
        for dev in &mut self.devices {
            match dev.mapped_write(addr, data) {
                Option::Some(()) => {
                    return;
                }
                Option::None => {}
            };
        }
    }

    pub fn new() -> Bus {
        Bus {
            devices: vec![],
            last_bus_val: 0,
        }
    }
}

#[cfg(tests)]
mod tests {
    use super::*;

    #[test]
    fn constructs() {
        let bus = Bus::new();
        assert_eq!(0, bus.devices.len());
        assert_eq!(0, bus.last_bus_val);
    }
}
