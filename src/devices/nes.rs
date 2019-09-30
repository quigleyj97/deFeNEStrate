/// A module representing the Nintendo Entertainment System.
///
/// Internally, the NES is a Bus with associated devices, and directly manages
/// linkages between the 2A03 and it's various peripherals, as well as the CPU
/// and PPU.

use std::cell::{RefCell};
use std::rc::{Rc};
use super::{cpu::Cpu6502, cartridge::Cartridge};
use crate::databus::Bus;

// This is what owns the CPU and bus
pub struct NesEmulator {
    cpu: Cpu6502<NesBus>,
    busref: Rc<RefCell<NesBus>>,
}

impl NesEmulator {
    pub fn step_emulator(&mut self) {
        self.cpu.exec();
        loop {
            let done_spinning = self.cpu.tick();
            if done_spinning { break; }
        }
    }

    pub fn step_debug(&mut self) -> String {
        let status = self.cpu.debug();
        loop {
            let done_spinning = self.cpu.tick();
            if done_spinning { break; }
        }
        status
    }

    /// Jump the CPU program counter to the given address.
    ///
    /// This is mainly useful for test automation.
    pub fn set_pc(&mut self, addr: u16) {
        self.cpu.jmp(addr);
    }

    pub fn load_cart(&mut self, cart: Box<dyn Cartridge>) {
        let mut bus = self.busref.borrow_mut();
        bus.cart = Option::Some(cart);
    }
}

impl Default for NesEmulator {
    fn default() -> NesEmulator {
        let bus = NesBus::default();
        let busref = Rc::new(RefCell::new(bus));
        NesEmulator {
            busref: Rc::clone(&busref),
            cpu: Cpu6502::new(busref),
        }
    }
}

/// This is what owns the various bus devices
struct NesBus {
    /// The 2kb of ram installed on the NES
    ram: Box<[u8; 2048]>,
    /// The currently loaded Cart image, or None
    cart: Option<Box<dyn Cartridge>>,
}

impl Bus for NesBus {
    fn read(&self, addr: u16) -> u8 {
        if addr < 0x2000 {
            // AND with 0x07FF to implement the RAM mirrors
            return self.ram[(addr & 0x07FF) as usize];
        }
        else if addr > 0x401F {
            // Cart
            return match &self.cart {
                Option::None => 0,
                Option::Some(cart) => cart.read_prg(addr)
            };
        }
        // Open bus
        // Technically this should be the last read byte, randomly decayed. But
        // I'm lazy, and hope that nothing reasonable actually relies on that...
        0
    }

    fn write(&mut self, addr: u16, data: u8) {
        if addr < 0x2000 {
            self.ram[(addr & 0x07FF) as usize] = data;
        }
        else if addr > 0x401F {
            // Cart
            match &mut self.cart {
                Option::None => {},
                Option::Some(cart) => cart.write_prg(addr, data)
            }
        }
    }
}

impl Default for NesBus {
    fn default() -> NesBus {
        NesBus {
            ram: Box::new([0u8; 2048]),
            cart: Option::None
        }
    }
}