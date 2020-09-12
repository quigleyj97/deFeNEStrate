//! Module for memory devices, such as RAM and ROM

use super::bus::{BusDevice, BusPeekResult};

pub struct Ram {
    buf: Vec<u8>,
    len: usize,
}

impl BusDevice for Ram {
    fn read(&mut self, addr: u16, last_bus_value: u8) -> u8 {
        self.peek(addr).unwrap(last_bus_value)
    }

    fn peek(&self, addr: u16) -> BusPeekResult {
        if (addr as usize) > self.len {
            BusPeekResult::Unmapped
        } else {
            BusPeekResult::Result(self.buf[addr as usize])
        }
    }

    fn write(&mut self, addr: u16, value: u8) {
        self.buf[addr as usize] = value;
    }
}

impl Ram {
    pub fn new(size: usize) -> Ram {
        Ram {
            len: size,
            buf: vec![0u8; size],
        }
    }

    pub fn new_from_buf(size: usize, buf: &[u8]) -> Ram {
        Ram {
            len: size,
            buf: Vec::from(buf),
        }
    }
}
