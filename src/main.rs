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

    run_with(
        "deFeNEStrate [ nestest.nes ]",
        Vector::new(800, 600),
        Settings::default(),
        || {
            eprintln!("deFeNEStrate initialized");
            let mut app = MainWindow::new()?;
            let args: Vec<String> = args().collect();

            let cart_path = &args[1];

            let cart = devices::cartridge::NesMapper0Cart::from_file(cart_path)?;

            app.nes.load_cart(Box::from(cart));
            app.nes.set_pc(0xC000);
            eprintln!("Loaded cart");
            Ok(app)
        },
    );
}
