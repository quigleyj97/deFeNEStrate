#[macro_use]
extern crate bitflags;
#[cfg(target_arch = "wasm32")]
extern crate stdweb;

pub mod databus;
pub mod devices;

use devices::nes::NesEmulator;

#[cfg(target_arch = "wasm32")]
fn main() {
    use stdweb;
    stdweb::initialize();

    stdweb::web::alert("Hi WASM!");

    let mut nes = NesEmulator::default();

    for _ in 0..5 {
        println!("{}", nes.step_debug());
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    use std::env::args;
    use std::process;
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
