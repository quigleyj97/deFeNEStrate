mod ines;
mod nrom;
mod utils;

pub use utils::{ICartridge, WithCartridge};

/// Given a buffer to an iNES ROM, return an ICartridge representing that ROM
pub fn from_rom(buf: &[u8]) -> impl utils::ICartridge {
    let header = ines::parse_ines_header(&buf);
    let lower_mapper_nibble: u8 = (header.flags_6 & ines::INesFlags6::LOWER_MAPPER_NIBBLE).bits();
    let upper_mapper_nibble: u8 = (header.flags_7 & ines::INesFlags7::UPPER_MAPPER_NIBBLE).bits();
    let mapper = lower_mapper_nibble | (upper_mapper_nibble >> 4);

    match mapper {
        0 => nrom::NROMCartridge::new(header, &buf),
        _ => unimplemented!("Mapper {} not implemented", mapper),
    }
}
