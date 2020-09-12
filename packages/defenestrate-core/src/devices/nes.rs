use super::bus::{cpu_memory_map, BusDevice, BusPeekResult, Motherboard};
use super::cartridge::ICartridge;
use super::cpu;
use super::mem::Ram;

/// A struct representing the NES as a whole unit
pub struct Nes {
    /// The NES CPU
    cpu: cpu::Cpu6502,
    /// The 2k RAM installed on the NES
    ram: Ram,
    /// The last value on the main address bus
    last_bus_value: u8,
    /// A tracking var for the number of cycles executed
    ///
    /// This is used for things like DMA synchronization and PPU/CPU clock timing
    cycles: usize,
    /// Whether the CPU is ready to execute a new instruction
    is_cpu_idle: bool,
    /// The cartridge containing the game to be played
    cart: Box<dyn ICartridge>,
}

impl Motherboard for Nes {
    fn read(&mut self, addr: u16) -> u8 {
        let (device, addr) = cpu_memory_map::match_addr(addr);
        let res = match device {
            cpu_memory_map::Device::Cartridge => self.cart.read_prg(addr, self.last_bus_value),
            cpu_memory_map::Device::RAM => self.ram.read(addr, self.last_bus_value),
            cpu_memory_map::Device::Unmapped => self.last_bus_value,
        };
        self.last_bus_value = res;
        res
    }

    fn peek(&self, addr: u16) -> Option<u8> {
        let (device, addr) = cpu_memory_map::match_addr(addr);
        match device {
            cpu_memory_map::Device::Cartridge => self.cart.peek_prg(addr),
            cpu_memory_map::Device::RAM => self.ram.peek(addr),
            cpu_memory_map::Device::Unmapped => BusPeekResult::Unmapped,
        }
        .to_optional()
    }

    fn write(&mut self, addr: u16, data: u8) {
        let (device, addr) = cpu_memory_map::match_addr(addr);
        match device {
            cpu_memory_map::Device::Cartridge => self.cart.write_prg(addr, data),
            cpu_memory_map::Device::RAM => self.ram.write(addr, data),
            cpu_memory_map::Device::Unmapped => {}
        };
        self.last_bus_value = data;
    }
}

impl Nes {
    pub fn new(cart: Box<dyn ICartridge>) -> Nes {
        let cpu = cpu::Cpu6502::new();
        let ram = Ram::new(2048);
        Nes {
            cpu,
            ram,
            last_bus_value: 0x00,
            cycles: 0,
            is_cpu_idle: true,
            cart,
        }
    }

    /// Advance the emulator 1 PPU cycle at a time, executing CPU instructions
    /// when appropriate (3 cycles in NTSC mode)
    pub fn tick(&mut self) {
        self.cycles += 1;
        // TODO: PPU ticks here
        if self.cycles % 3 != 0 {
            return; // no CPU ticks required
        }
        // TODO: Tick the gamepad and OAM DMA controllers
        // TODO: test here for oam_dma inactive
        if self.is_cpu_idle {
            cpu::exec(self);
        }
        self.is_cpu_idle = cpu::tick(self);
    }
}

impl cpu::WithCpu for Nes {
    fn cpu(&self) -> &cpu::Cpu6502 {
        &self.cpu
    }

    fn cpu_mut(&mut self) -> &mut cpu::Cpu6502 {
        &mut self.cpu
    }
}
