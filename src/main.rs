#[macro_use]
extern crate bitflags;

mod databus;
mod emulator;
mod structs;

use std::cell::{RefCell};
use std::rc::{Rc};

use databus::Bus;
use emulator::Cpu6502;

fn main() {
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
