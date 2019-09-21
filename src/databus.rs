pub struct Bus {
    data: Box<[u8]>,
}

impl Bus {
    /// Read from the Bus at the given address.
    pub fn read(&self, addr: u16) -> u8 {
        return self.data[addr as usize];
    }

    /// Write to the Bus at the given address.
    pub fn write(&mut self, addr: u16, value: u8) {
        self.data[addr as usize] = value;
    }
}

impl Bus {
    pub fn new() -> Bus {
        return Bus {
            data: Box::new([0; 65_535]),
        };
    }
}
