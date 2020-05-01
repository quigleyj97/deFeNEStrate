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
    use quicksilver::{
        geom::Vector,
        lifecycle::{run_with, Settings, State},
    };
    use std::env::args;
    eprintln!("Initializing...");
    let args: Vec<String> = args().collect();

    if args.len() < 2 {
        eprintln!("No cart file provided, exiting...");
        return;
    }

    let cart_path = &args[1];

    run_with(
        &format!("deFeNEStrate [ {} ]", cart_path),
        Vector::new(800, 600),
        Settings::default(),
        || {
            eprintln!("deFeNEStrate initialized");
            let mut app = MainWindow::new()?;

            let cart = devices::cartridge::NesMapper0Cart::from_file(cart_path)?;

            app.nes.load_cart(Box::from(cart));
            eprintln!("Loaded cart");
            Ok(app)
        },
    );
}
