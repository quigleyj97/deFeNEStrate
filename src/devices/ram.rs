use crate::devices::bus::BusDevice;

pub struct Ram {
    /// The internal RAM buffer
    memory: Vec<u8>,
    /// The declared size of the RAM. This is assumed to be constant.
    size: usize,
}

#[allow(clippy::len_without_is_empty)]
impl Ram {
    pub fn new(size: usize) -> Ram {
        Ram {
            memory: vec![0; size],
            size,
        }
    }

    pub fn len(&self) -> usize {
        self.size
    }
}

impl BusDevice for Ram {
    fn read(&mut self, addr: u16) -> u8 {
        assert!(
            (addr as usize) < self.size,
            "Precondition failed: Addr exceeds RAM size"
        );
        self.memory[addr as usize]
    }

    fn write(&mut self, addr: u16, data: u8) {
        assert!(
            (addr as usize) < self.size,
            "Precondition failed: Addr exceeds RAM size"
        );
        self.memory[addr as usize] = data;
    }
}
