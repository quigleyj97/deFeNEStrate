/// This test runs NESTEST, which is a comprehensive CPU tester that works
/// even without the other components of the NES, like the PPU or APU.
///
/// NESTEST will test a number of instructions, and if they fail it will write
/// the index of the last failure to a particular address and then Halt-and-
/// Catch-Fire.
///
/// As of right now, the emu doesn't yet support 'cartridges', so instead the
/// ROM is loaded up by writing the whole thing to databus-internal memory and
/// pointing the emu CPU at $C000.

extern crate defenestrate;

use defenestrate::devices::{nes::NesEmulator, cartridge::NesMapper0Cart};

#[test]
fn nestest_exec() {
    let mut nes = NesEmulator::default();

    let cart = NesMapper0Cart::from_file("./tests/data/nestest.nes");

    match cart {
        Result::Err(e) => panic!("Failed to read nestest: {}", e),
        Result::Ok(cart) => {
            nes.load_cart(Box::new(cart));
        }
    }

    nes.set_pc(0xC000);

    for _ in 0..9000 {
        println!("{}", nes.step_debug());
    }

    assert_eq!(nes.read_bus(0x0000), 0x00);
    assert_eq!(nes.read_bus(0x0001), 0x00);
    assert_eq!(nes.read_bus(0x0002), 0x00);
    assert_eq!(nes.read_bus(0x0003), 0x00);
}
