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
    dev: *mut dyn BusDevice,
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
        unsafe { Option::Some((&mut *self.dev).read((addr - self.addr_start) & self.addr_mask)) }
    }

    fn mapped_write(&mut self, addr: u16, data: u8) -> Option<()> {
        if addr < self.addr_start || addr > self.addr_end {
            return Option::None;
        }
        unsafe {
            (&mut *self.dev).write((addr - self.addr_start) & self.addr_mask, data);
        }
        Option::Some(())
    }

    fn new(
        dev: *mut dyn BusDevice,
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
        dev: *const dyn BusDevice,
        addr_start: u16,
        addr_end: u16,
        addr_mask: u16,
    ) {
        self.devices.push(MemoryMappedDevice::new(
            dev as *mut dyn BusDevice,
            addr_start,
            addr_end,
            addr_mask,
        ));
    }

    pub fn unmap_device(&mut self, dev: *const dyn BusDevice) {
        let mut i = 0;
        while i != self.devices.len() {
            if self.devices[i].dev == (dev as *mut dyn BusDevice) {
                self.devices.remove(i);
                return;
            }
            i += 1;
        }
        panic!("Attempted to remove unmapped device: {:#?}", dev);
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

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_ADDR: u16 = 0x34;
    const TEST_DATA: u8 = 0xEA;
    const TEST_MIRROR: u16 = 0x00FF;
    const MIRRORED_ADDR: u16 = 0x1234;

    struct TestObj {}
    impl BusDevice for TestObj {
        fn read(&mut self, addr: u16) -> u8 {
            assert_eq!(addr, TEST_ADDR, "Read address mismatch");
            TEST_DATA
        }

        fn write(&mut self, addr: u16, data: u8) {
            assert_eq!(addr, TEST_ADDR, "Write address mismatch");
            assert_eq!(data, TEST_DATA, "Test data mismatch");
        }
    }

    #[test]
    fn constructs() {
        let bus = Bus::new();
        assert_eq!(0, bus.devices.len());
        assert_eq!(0, bus.last_bus_val);
    }

    #[test]
    fn adds_mapped_device() {
        let dev = TestObj {};
        let mut bus = Bus::new();
        bus.map_device(&dev, 0, 0xFFFF, 0xFFFF);
        assert!(bus.devices.len() == 1);
        assert_eq!(TEST_DATA, bus.read(TEST_ADDR));
        bus.write(TEST_ADDR, TEST_DATA);
    }

    #[test]
    fn handles_offsets_correctly() {
        const ADDR_OFFSET: u16 = 0xFF;
        let dev = TestObj {};
        let mut bus = Bus::new();
        bus.map_device(&dev, ADDR_OFFSET, 0xFFFF, 0xFFFF);
        assert_eq!(TEST_DATA, bus.read(TEST_ADDR + ADDR_OFFSET));
    }

    #[test]
    fn mirrors_correctly() {
        let dev = TestObj {};
        let mut bus = Bus::new();
        bus.map_device(&dev, 0, 0xFFFF, TEST_MIRROR);
        assert_eq!(TEST_DATA, bus.read(MIRRORED_ADDR));
        bus.write(MIRRORED_ADDR, TEST_DATA);
    }

    #[test]
    fn mirrors_with_offset_correctly() {
        const ADDR_OFFSET: u16 = 0xABCD;
        let dev = TestObj {};
        let mut bus = Bus::new();
        bus.map_device(&dev, ADDR_OFFSET, 0xFFFF, TEST_MIRROR);
        assert_eq!(TEST_DATA, bus.read(MIRRORED_ADDR + ADDR_OFFSET));
    }

    #[test]
    fn unmaps_correctly() {
        let dev = TestObj {};
        let mut bus = Bus::new();
        bus.map_device(&dev, 0, 0xFFFF, 0xFFFF);
        bus.read(TEST_ADDR);
        bus.unmap_device(&dev);
        assert_eq!(bus.devices.len(), 0, "Bus device not dropped!");
    }

    #[test]
    #[should_panic]
    fn panics_when_attempting_to_unmap_unmapped_dev() {
        let dev = TestObj {};
        let mut bus = Bus::new();
        bus.unmap_device(&dev);
    }
}
