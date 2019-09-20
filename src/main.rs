#[macro_use]
extern crate bitflags;

mod databus;
mod emulator;

use std::rc::Rc;

fn main() {
    let bus = databus::Bus::new();
    let busref = Rc::from(bus);

    let mut cpu = emulator::Cpu6502::new(busref);
    println!("CPU initialized");

    println!("{}", cpu);
    cpu.tick();
    println!("{}", cpu);
    cpu.reset();
    println!("{}", cpu);
}
