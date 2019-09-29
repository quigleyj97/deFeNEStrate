/// A module representing the Nintendo Entertainment System.
///
/// Internally, the NES is a Bus with associated devices, and directly manages
/// linkages between the 2A03 and it's various peripherals, as well as the CPU
/// and PPU.

use std::cell::{RefCell};
use std::rc::{Rc};
use super::cpu::Cpu6502;
use crate::databus::Bus;

// This is what owns the CPU and bus
pub struct NesEmulator {
    cpu: Cpu6502<NesBus>
}

impl NesEmulator {
    pub fn step_emulator(&mut self) {
        self.cpu.exec();
        loop {
            let cycles_remaining = self.cpu.tick();
            if !cycles_remaining { break; }
        }
    }

    pub fn step_debug(&mut self) -> String {
        let status = self.cpu.debug();
        loop {
            let cycles_remaining = self.cpu.tick();
            if !cycles_remaining { break; }
        }
        status
    }
}

impl Default for NesEmulator {
    fn default() -> NesEmulator {
        let bus = NesBus { };
        let busref = Rc::new(RefCell::new(bus));
        NesEmulator {
            cpu: Cpu6502::new(busref)
        }
    }
}

/// This is what owns the various bus devices
struct NesBus {

}

impl Bus for NesBus {
    fn read(&self, addr: u16) -> u8 {
        0
    }

    fn write(&mut self, addr: u16, data: u8) {
        println!("write ${:04X} = 0x{:02X}", addr, data);
    }
}
