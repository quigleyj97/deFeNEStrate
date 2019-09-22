#[macro_use]
extern crate bitflags;

pub mod databus;
pub mod emulator;
mod structs;

use std::cell::{RefCell};
use std::rc::{Rc};

use databus::Bus;
use emulator::Cpu6502;

fn main() {
    let bus = Bus::new();

    let busref = Rc::new(RefCell::new(bus));

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
