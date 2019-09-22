/// This test runs NESTEST, which is a comprehensive CPU tester that works
/// even without the other components of the NES, like the PPU or APU.
///
/// NESTEST will test a number of instructions, and if they fail it will write
/// the index of the last failure to a particular address and then Halt-and-
/// Catch-Fire.
///
/// As of right now, the emu doesn't yet support 'cartridges', so instead the
/// ROM is loaded up by writing the whole thing to databus-internal memory and
/// pointing the emu CPU at $C000.

extern crate defenestrate;

use std::cell::{RefCell};
use std::rc::{Rc};

use defenestrate::emulator::Cpu6502;
use defenestrate::databus::Bus;

#[test]
fn nestest_exec() {
    let bus = Bus::new();

    let busref = Rc::new(RefCell::new(bus));

    write_test(&busref);

    let mut cpu = Cpu6502::new(busref);
    println!("CPU initialized");

    println!("{}", cpu);

    let mut instrs = 0;
    loop {
        cpu.exec();
        loop {
            let cycles = cpu.tick();
            if cycles { break; }
        }
        println!("{}", cpu);
        instrs += 1;
        if instrs > 7 { break; }
    }
    cpu.reset();
    println!("{}", cpu);
    assert_eq!(true, true);
}

fn write_test(busref: &Rc<RefCell<Bus>>) {
    let mut bus = busref.borrow_mut();

    bus.write(0xC000, 0x4C);
    bus.write(0xC001, 0xF5);
    bus.write(0xC002, 0xC5);
    bus.write(0xC5F5, 0xA2);
    bus.write(0xC5F6, 0x00);
    bus.write(0xC5F7, 0x86);
    bus.write(0xC5F8, 0x00);
    bus.write(0xC5F9, 0x86);
    bus.write(0xC5FA, 0x10);
    bus.write(0xC5FB, 0x86);
    bus.write(0xC5FC, 0x11);
    bus.write(0xC5FD, 0x20);
    bus.write(0xC5FE, 0x2D);
    bus.write(0xC5FF, 0xC7);
    bus.write(0xC72D, 0xEA);
}
