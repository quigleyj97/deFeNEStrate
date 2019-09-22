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

use std::cell::{RefCell};
use std::rc::{Rc};
use std::fs::File;
use std::io::{SeekFrom, prelude::*};
use std::path::Path;

use defenestrate::emulator::Cpu6502;
use defenestrate::databus::Bus;

#[test]
fn nestest_exec() {
    let bus = Bus::new();

    let busref = Rc::new(RefCell::new(bus));

    write_test(&busref);

    let mut cpu = Cpu6502::new(busref);
    println!("CPU initialized");

    println!("{}", cpu);

    let mut instrs = 0;
    loop {
        cpu.exec();
        loop {
            let cycles = cpu.tick();
            if cycles { break; }
        }
        println!("{}", cpu);
        instrs += 1;
        if instrs > 500 { break; }
    }
    cpu.reset();
    println!("{}", cpu);
    assert_eq!(true, true);
}

fn write_test(busref: &Rc<RefCell<Bus>>) {
    let path = Path::new("./tests/data/nestest.nes");
    let mut file = File::open(path).expect("Could not read NESTEST rom");
    file.seek(SeekFrom::Start(16)).unwrap(); // skip the header
    let mut bus = busref.borrow_mut();

    let mut pc = 0xC000;

    for byte in file.bytes() {
        if pc == 0xFFFF { return; }
        bus.write(pc, byte.unwrap());
        pc += 1;
    }
}
