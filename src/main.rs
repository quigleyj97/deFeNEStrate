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

    cpu.print_debug();
    cpu.tick();
    cpu.reset();
}
