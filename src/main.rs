#[macro_use]
extern crate bitflags;

pub mod databus;
pub mod devices;

use devices::nes::NesEmulator;

fn main() {
    let mut nes = NesEmulator::default();
    println!("deFeNEStrate initialized");

    println!("{}", nes.step_debug());
}
