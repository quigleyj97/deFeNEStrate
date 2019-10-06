#[macro_use]
extern crate bitflags;
extern crate quicksilver;
#[cfg(target_arch = "wasm32")]
#[macro_use]
extern crate stdweb;

pub mod databus;
pub mod devices;
mod ui;

use ui::MainWindow;

#[cfg(target_arch = "wasm32")]
fn main() {
    use quicksilver::{
        geom::Vector,
        lifecycle::{run, Settings},
    };
    use stdweb;
    stdweb::initialize();

    stdweb::js! {
        console.log("Hi WASM!");
    }

    run::<MainWindow>("deFeNEStrate", Vector::new(800, 600), Settings::default());
}

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    use devices::nes::NesEmulator;
    use quicksilver::{
        geom::Vector,
        lifecycle::{run_with, Settings},
    };
    use std::env::args;
    use std::process;
    eprintln!("Initializing...");

    let mut emu = MainWindow {
        nes: NesEmulator::default(),
    };

    let args: Vec<String> = args().collect();

    let cart_path = &args[1];

    let cart = devices::cartridge::NesMapper0Cart::from_file(cart_path);

    match cart {
        Result::Err(e) => {
            eprintln!("Failed to read cart at {}: {}", cart_path, e);
            process::exit(1);
        }
        Result::Ok(cart) => {
            emu.nes.load_cart(Box::new(cart));
        }
    }

    eprintln!("deFeNEStrate initialized");

    run_with(
        "Draw things",
        Vector::new(800, 600),
        Settings::default(),
        || Ok(emu),
    );
}
