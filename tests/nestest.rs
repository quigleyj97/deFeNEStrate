//! This test runs NESTEST, which is a comprehensive CPU tester that works
//! even without the other components of the NES, like the PPU or APU.
//!
//! NESTEST will test a number of instructions, and if they fail it will write
//! the index of the last failure to a particular address and then Halt-and-
//! Catch-Fire.
//!
//! NESTEST includes a reference output from a known-good emulator, so that
//! this integration test can compare the emulator's output to that good output
//! to catch differences in everything from execution to cycle counts.
//!
//! As of right now, PPU cycle counts are not accurate, so the test will parse
//! out each field from the log and compare all fields but the PPU counts.
//!
//! Further, the CPU emulator is not 100% accurate compared to the good output
//! (though it's close!) so the test will run to completion and _then_ report
//! the number of differences. If that number exceeds 100, the test will fail.

extern crate defenestrate;

mod util;

use util::{logparse, provider};

use defenestrate::devices::nes::NesEmulator;

// If true, test Nestest to completion
const TEST_ILLEGAL_OPCODES: bool = false;

#[test]
fn nestest_exec() {
    let mut nes = NesEmulator::default();

    let cart = provider::load_nestest_rom();
    let gold_log = provider::load_gold_standard_log();

    nes.load_cart_without_reset(Box::new(cart));
    nes.set_pc(0xC000);

    let mut line = 1;

    for gold_line in gold_log {
        let log = nes.step_debug();
        println!("L{:04} {}", line, log);
        let log = logparse::parse_line(&log);
        let gold_line = logparse::parse_line(&gold_line);
        logparse::assert_logs_eq(&log, &gold_line);
        line += 1;
        // illegal opcodes begin at line 5004
        if !TEST_ILLEGAL_OPCODES && line > 5003 {
            break;
        }
    }

    assert_eq!(nes.read_bus(0x0000), 0x00);
}
