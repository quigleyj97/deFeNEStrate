struct Ram<len> {
    /// The internal RAM buffer
    memory: Vec<u8>,
    /// The declared size of the RAM. This is assumed to be constant.
    size: usize,
}

impl Ram {
    fn new(size: usize) -> Ram {
        Ram {
            memory: vec![0, size],
            size,
        }
    }

    pub fn len(&self) -> usize {
        return self.size;
    }
}

impl BusDevice for Ram {
    fn read(&self, addr: u16) -> u8 {
        assert!(
            addr < self.size,
            "Precondition failed: Addr exceeds RAM size"
        );
        self.memory[addr as usize]
    }

    fn write(&mut self, addr: u16, data: u8) {
        assert!(
            addr < self.size,
            "Precondition failed: Addr exceeds RAM size"
        );
        self.memory[addr as usize] = data;
    }
}
