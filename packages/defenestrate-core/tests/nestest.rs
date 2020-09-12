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

extern crate defenestrate_core;

mod util;

use util::{logparse, provider};

use defenestrate_core::devices::cpu::WithCpu;
use defenestrate_core::devices::nes::Nes;
use provider::NESTEST_ROM_PATH;

// If true, test Nestest to completion
const TEST_ILLEGAL_OPCODES: bool = false;

#[test]
fn nestest_exec() {
    let mut nes = Nes::new_from_file(&NESTEST_ROM_PATH).expect("Could not read NESTEST rom");

    let gold_log = provider::load_gold_standard_log();

    nes.cpu_mut().state.pc = 0xC000;

    let mut line = 1;

    for gold_line in gold_log {
        let raw_log = nes.dbg_step_cpu();
        let log = logparse::parse_line(&raw_log);
        let gold_line = logparse::parse_line(&gold_line);
        println!(
            "L{:04} {}  DELTA {}",
            line,
            raw_log,
            (log.cycle as i64) - (gold_line.cycle as i64)
        );
        logparse::assert_logs_eq(&log, &gold_line);
        line += 1;
        // illegal opcodes begin at line 5004
        if !TEST_ILLEGAL_OPCODES && line > 5003 {
            break;
        }
    }
}
