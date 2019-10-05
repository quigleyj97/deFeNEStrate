#[macro_use]
extern crate bitflags;

pub mod databus;
pub mod devices;

use devices::nes::NesEmulator;
use std::env::args;
use std::process;

fn main() {
    eprintln!("Initializing...");

    let mut nes = NesEmulator::default();

    let args: Vec<String> = args().collect();

    let cart_path = &args[1];

    let cart = devices::cartridge::NesMapper0Cart::from_file(cart_path);

    match cart {
        Result::Err(e) => {
            eprintln!("Failed to read cart at {}: {}", cart_path, e);
            process::exit(1);
        }
        Result::Ok(cart) => {
            nes.load_cart(Box::new(cart));
        }
    }

    nes.set_pc(0xC000);

    eprintln!("deFeNEStrate initialized");

    for _ in 0..9000 {
        println!("{}", nes.step_debug());
    }
}
