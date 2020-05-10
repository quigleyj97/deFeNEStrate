//! A module representing the Nintendo Entertainment System.
//!
//! Internally, the NES is a Bus with associated devices, and directly manages
//! linkages between the 2A03 and it's various peripherals, as well as the CPU
//! and PPU.

use crate::devices::bus::BusDevice;
use crate::devices::cartridge::{CartCpuBridge, CartPpuBridge, Cartridge, CART_START_ADDR};
use crate::devices::cpu::Cpu6502;
use crate::devices::ppu::Ppu2C02;
use crate::devices::ram::Ram;

// This is what owns the CPU and bus
pub struct NesEmulator {
    cpu: Cpu6502,
    ppu: Ppu2C02,
    ram: Ram,
    cart: Box<dyn Cartridge>,
    cartCpu: CartCpuBridge<dyn Cartridge>,
    cartPpu: CartPpuBridge<dyn Cartridge>,
    /// The total number of cycles that have been executed.
    cycles: u16,
    is_cpu_idle: bool,
    is_frame_ready: bool,
}

impl NesEmulator {
    /// Run the emulator for one whole frame, and return a reference to that frame data
    pub fn run_frame(&mut self) -> Box<[u8; 240 * 256 * 3]> {
        loop {
            self.tick();
            if self.is_frame_ready {
                break;
            }
        }
        self.ppu.get_buffer()
    }

    pub fn tick(&mut self) {
        self.cycles += 1;
        // u16 max happens to be divisible by 3
        if self.cycles == u16::max_value() {
            self.cycles = 0;
        }
        self.ppu.clock();
        self.is_frame_ready = self.ppu.is_frame_ready();
        if self.ppu.is_vblank() {
            self.cpu.trigger_nmi();
            self.ppu.ack_vblank();
        }
        if self.cycles % 3 == 0 {
            if self.is_cpu_idle {
                self.cpu.exec();
            }
            self.is_cpu_idle = self.cpu.tick();
        }
    }

    pub fn step_emulator(&mut self) {
        self.cpu.exec();
        loop {
            let done_spinning = self.cpu.tick();
            if done_spinning {
                break;
            }
        }
    }

    pub fn step_debug(&mut self) -> String {
        let status = self.cpu.debug();
        loop {
            let done_spinning = self.cpu.tick();
            if done_spinning {
                break;
            }
        }
        status
    }

    //region Test automation helpers
    /// Jump the CPU program counter to the given address.
    ///
    /// This is mainly useful for test automation.
    pub fn set_pc(&mut self, addr: u16) {
        self.cpu.jmp(addr);
    }

    /// Read from the bus at a given address
    ///
    /// This is for test automation to read specific addresses and check for
    /// errors in some comprehensive test ROMs
    pub fn read_bus(&mut self, addr: u16) -> u8 {
        self.cpu.bus.read(addr)
    }

    pub fn get_status(&self) -> String {
        format!("{}", self.cpu)
    }

    pub fn get_chr(&self, use_pallete: bool) -> Box<[u8; 256 * 128 * 3]> {
        self.ppu.dump_pattern_table(use_pallete)
    }

    pub fn get_palletes(&self) -> [u8; 128 * 2 * 3] {
        self.ppu.dump_palettes()
    }
    //endregion

    pub fn load_cart(&mut self, cart: Box<dyn Cartridge>) {
        self.load_cart_without_reset(cart);
        self.cpu.reset();
    }

    // TODO: Move the reset responsibility to the CPU directly
    pub fn load_cart_without_reset(&mut self, cart: Box<dyn Cartridge>) {
        self.cpu.bus.unmap_device(&self.cartCpu);
        self.cart = cart;
        let cpuBridge = CartCpuBridge::new(cart.as_ref());
        self.cartCpu = cpuBridge;
        self.cpu
            .bus
            .map_device(&self.cartCpu, CART_START_ADDR, 0xFFFF, 0xFFFF);
    }
}

impl NesEmulator {
    fn new(cart: Box<dyn Cartridge>) -> NesEmulator {
        let cartCpu = CartCpuBridge::new(cart.as_ref());
        let cartPpu = CartPpuBridge::new(cart.as_ref());
        let ram = Ram::new(2048);
        let cpu = Cpu6502::new();
        let ppu = Ppu2C02::new();
        cpu.bus
            .map_device(&cartCpu, CART_START_ADDR, 0xFFFF, 0xFFFF);
        cpu.bus.map_device(&ram, 0x0000, 0x2000, 0x07FF);
        NesEmulator {
            cpu,
            ppu,
            cart,
            cartCpu,
            cartPpu,
            cycles: 0,
            is_cpu_idle: false,
            is_frame_ready: false,
        }
    }
}

const PPU_REGISTER_START_ADDR: u16 = 0x2000;
const PPU_REGISTER_END_ADDR: u16 = 0x3FFF;
const PPU_REGISTER_MASK: u16 = 0x0007;

struct PpuRegisters {
    ppu: *const Ppu2C02,
}

impl PpuRegisters {
    fn new(ppu: *const Ppu2C02) -> PpuRegisters {
        PpuRegisters { ppu }
    }
}

impl BusDevice for PpuRegisters {
    fn read(&self, addr: u16) -> u8 {
        assert!(
            addr < 8,
            "Precondition failed: Addr exceeds PPU register size"
        );
        unsafe { (*self.ppu).read_ppu(addr + PPU_REGISTER_START_ADDR) }
    }

    fn write(&mut self, addr: u16, data: u8) {
        assert!(
            addr < 8,
            "Precondition failed: Addr exceeds PPU register size"
        );
        unsafe { (*self.ppu).write_ppu(addr + PPU_REGISTER_START_ADDR, data) }
    }
}
