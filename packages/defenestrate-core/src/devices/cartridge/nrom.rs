use super::ines::{INesFlags6, INesHeader};
use super::utils::ICartridge;
use crate::devices::bus::BusPeekResult;

pub struct NROMCartridge {
    chr: Vec<u8>,
    prg: Vec<u8>,
    nametable: Vec<u8>,
    use_horizontal_mirroring: bool,
    is_16k: bool,
}

impl NROMCartridge {
    pub fn new(header: INesHeader, buf: &[u8]) -> NROMCartridge {
        let INesHeader {
            prg_size, flags_6, ..
        } = header;
        let prg_end = 16 + 0x4000 * prg_size;
        let mut prg_buffer = vec![0u8; 0x4000 * prg_size];
        prg_buffer.clone_from_slice(&buf[16..prg_end]);
        let mut chr_buffer = vec![0u8; 0x2000];
        chr_buffer.clone_from_slice(&buf[prg_end..(prg_end + 0x2000)]);
        NROMCartridge {
            chr: chr_buffer,
            prg: prg_buffer,
            nametable: vec![0u8; 0x800],
            use_horizontal_mirroring: !flags_6.contains(INesFlags6::MIRRORING),
            is_16k: prg_size == 1,
        }
    }
}

impl ICartridge for NROMCartridge {
    fn read_chr(&mut self, addr: u16, last_bus_value: u8) -> u8 {
        return self.peek_chr(addr).unwrap(last_bus_value);
    }

    fn peek_chr(&self, addr: u16) -> BusPeekResult {
        if addr < 0x2000 {
            return BusPeekResult::Result(self.chr[addr as usize]);
        }
        let nt_addr = addr - 0x2000;
        let nt_addr = if self.use_horizontal_mirroring {
            // horizontal mirroring is done by wiring address pin 11 to
            // CIRAM 10, meaning bit 11 is moved to where bit 10 is and
            // the old bit 10 is dropped into the shadow realm
            (nt_addr & 0x3FF) | ((0x800 & addr) >> 1)
        } else {
            nt_addr & 0x7FF
        };
        return BusPeekResult::Result(self.nametable[nt_addr as usize]);
    }

    fn write_chr(&mut self, addr: u16, value: u8) {
        if addr < 0x2000 {
            return; // no-op: this is a ROM
        }
        let nt_addr = addr - 0x2000;
        let nt_addr = if self.use_horizontal_mirroring {
            (nt_addr & 0x3FF) | ((0x800 & addr) >> 1)
        } else {
            nt_addr & 0x7FF
        };
        self.nametable[nt_addr as usize] = value;
    }

    fn read_prg(&mut self, addr: u16, last_bus_value: u8) -> u8 {
        self.peek_prg(addr).unwrap(last_bus_value)
    }

    fn peek_prg(&self, addr: u16) -> crate::devices::bus::BusPeekResult {
        // 0x3FE0 is 0x8000 - CART_START_ADDR, since NROM starts at $8000
        BusPeekResult::Result(
            self.prg[if self.is_16k {
                (addr - 0x3FE0) & 0x3FFF
            } else {
                addr - 0x3FE0
            } as usize],
        )
    }

    fn write_prg(&mut self, _addr: u16, _value: u8) {
        return; // no-op: NROM PRG is read-only
    }

    fn dump_chr(&self) -> &[u8] {
        return &self.chr;
    }

    fn dump_nametables(&self) -> &[u8] {
        return &self.nametable;
    }
}

#[cfg(test)]
mod tests {
    use super::super::ines::parse_ines_header;
    use super::*;
    use std::fs::File;
    use std::io::prelude::*;
    use std::path::Path;

    const NESTEST_PATH: &str = "./tests/data/nestest.nes";
    const NROM_OFFSET: u16 = 0x3FE0;
    // it's convenient to test in global addresses, but the carts use local addrs
    const GLOBAL_ADDR_OFFSET: u16 = 0x4020;

    fn read_nestest() -> NROMCartridge {
        let path = Path::new(&NESTEST_PATH);
        let mut file = File::open(path).expect("Could not read NESTEST rom");

        let mut buf = Vec::new();

        file.read_to_end(&mut buf)
            .expect("Couldn't read NESTEST rom to end");

        let header = parse_ines_header(&buf);
        NROMCartridge::new(header, &buf)
    }

    #[test]
    fn should_map_prg_reads() {
        let cart = read_nestest();
        let data = cart.peek_prg(0xC000 - GLOBAL_ADDR_OFFSET).unwrap(0);
        // 0x4C is what we expect to be at this location in PRG, and can be
        // verified in xxd
        assert_eq!(data, 0x4C);
    }

    #[test]
    fn should_mirror_prg_reads_in_16k() {
        let cart = read_nestest();

        // $3FFF and $7FFF should be mirrors in 16k PRGs like NESTEST
        // In full address space, these addresses map to the reset vector
        let left = cart.peek_prg(0x3FFF + NROM_OFFSET).unwrap(0);
        let right = cart.peek_prg(0x7FFF + NROM_OFFSET).unwrap(0);
        assert_eq!(left, 0xC5, "Initial address doesn't match expected result");
        assert_eq!(left, right, "Mirrors don't align");
    }

    #[test]
    fn should_read_chr_correctly() {
        let cart = read_nestest();
        let data = cart.peek_chr(0x0020).unwrap(0);

        // $0020 should be 0x80, which can be verified by looking in xxd
        assert_eq!(data, 0x80);
    }
}
