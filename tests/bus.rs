//! Tests the ability of the Bus API to work with interior mutability, which is
//! required for the emulator to work.
//!
//! This will roughly mirror what the NES emulator will actually need

extern crate defenestrate;

use defenestrate::devices::bus::{Bus, BusDevice};

const TEST_DATA: u8 = 0xA5;

struct MySharedResource {}
impl BusDevice for MySharedResource {
    fn read(&mut self, _addr: u16) -> u8 {
        TEST_DATA
    }

    fn write(&mut self, _addr: u16, _data: u8) {}
}

#[derive(Debug)]
struct MyChildDevice {
    res: *mut MySharedResource,
}
impl BusDevice for MyChildDevice {
    fn read(&mut self, addr: u16) -> u8 {
        unsafe { (&mut *self.res).read(addr) }
    }

    fn write(&mut self, addr: u16, data: u8) {
        unsafe {
            (&mut *self.res).write(addr, data);
        }
    }
}

impl MyChildDevice {
    fn new(res: *const MySharedResource) -> MyChildDevice {
        MyChildDevice {
            res: res as *mut MySharedResource,
        }
    }
}

#[allow(dead_code)] // res is here to test a move
struct MyParentDevice {
    res: MySharedResource,
    child: *const MyChildDevice,
    bus: Bus,
}

#[test]
fn should_construct_bus() {
    const CHILD_ADDR: u16 = 0xEA;
    const SHARED_ADDR: u16 = 0xEAEA;
    let shr = MySharedResource {};
    let child = MyChildDevice::new(&shr);
    let mut bus = Bus::new();
    bus.map_device(&child, 0, 0x00FF, 0xFFFF);
    bus.map_device(&shr, 0x0100, 0xFFFF, 0xFFFF);
    let mut dev = MyParentDevice {
        res: shr,
        child: &child as *const MyChildDevice,
        bus,
    };
    assert_eq!(dev.bus.read(CHILD_ADDR), TEST_DATA);
    assert_eq!(dev.bus.read(SHARED_ADDR), TEST_DATA);
    assert_eq!(dev.child, &child);
}
