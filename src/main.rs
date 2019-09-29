#[macro_use]
extern crate bitflags;

pub mod databus;
pub mod devices;

use devices::nes::NesEmulator;

fn main() {
    let mut nes = NesEmulator::default();

    let cart = devices::cartridge::NesMapper0Cart::from_file("./tests/data/nestest.nes");

    match cart {
        Result::Err(e) => panic!("Failed to read nestest: {}", e),
        Result::Ok(cart) => {
            nes.load_cart(Box::new(cart));
        }
    }

    nes.set_pc(0xC000);

    println!("deFeNEStrate initialized");

    for _ in 0..5000 {
        println!("{}", nes.step_debug());
    }
}
