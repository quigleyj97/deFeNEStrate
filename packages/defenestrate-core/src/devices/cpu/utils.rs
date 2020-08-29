use super::super::bus::Motherboard;
use super::{
    cpu::WithCpu,
    structs::{AddressingMode, Instruction},
};

#[macro_export]
macro_rules! bytes_to_addr {
    ($fst: expr, $snd: expr) => {{
        (u16::from($snd) << 8) | u16::from($fst)
    }};
}

#[macro_export]
macro_rules! bus {
    (read $mb: expr, $addr: expr) => {{
        $mb.cpu_mut().cycles += 1;
        $mb.read($addr)
    }};
    (write $mb: expr, $addr: expr, $data: expr) => {{
        $mb.cpu_mut().cycles += 1;
        $mb.write($addr, $data)
    }};
}

#[macro_export]
macro_rules! adj_cycles {
    ($mb: expr, $delta: expr) => {{
        $mb.cpu_mut().cycles = $mb.cpu_mut().cycles.wrapping_add($delta as u32)
    }};
}

#[macro_export]
macro_rules! reg {
    (get $reg: ident, $mb: expr) => {{
        $mb.cpu().state.$reg
    }};

    (set $reg: ident, $mb: expr, $val: expr) => {{
        $mb.cpu_mut().state.$reg = $val
    }};

    (add $reg: ident, $mb: expr, $val: expr) => {{
        $mb.cpu_mut().state.$reg = $mb.cpu().state.$reg.wrapping_add($val)
    }};

    (sub $reg: ident, $mb: expr, $val: expr) => {{
        $mb.cpu_mut().state.$reg = $mb.cpu().state.$reg.wrapping_sub($val)
    }};
}

pub fn print_debug<T: WithCpu + Motherboard>(mb: &T) -> String {
    let bytes = reg!(get instruction, mb).to_le_bytes();
    let ops = match reg!(get addr_mode, mb) {
        AddressingMode::Abs
        | AddressingMode::AbsX
        | AddressingMode::AbsY
        | AddressingMode::AbsInd => format!("{:02X} {:02X} {:02X}", bytes[0], bytes[1], bytes[2]),
        AddressingMode::Accum | AddressingMode::Impl => format!("{:8<02X}", bytes[0]),
        _ => format!("{:02X} {:02X}   ", bytes[0], bytes[1]),
    };

    let operand_bytes = bytes_to_addr!(bytes[1], bytes[2]);
    let data = mb.peek(reg!(get addr, mb)).unwrap_or(0xA5); // 0xA5 is a debug pattern
    let addr = reg!(get addr, mb);
    let instr = reg!(get instr, mb);
    let is_jmp = instr == Instruction::JMP || instr == Instruction::JSR;
    let instr = match reg!(get addr_mode, mb) {
        AddressingMode::Abs => {
            if !is_jmp {
                format!("{:3?} ${:04X} = {:02X}", instr, addr, data)
            } else {
                format!("{:3?} ${:04X}", instr, addr)
            }
        }
        AddressingMode::AbsX => format!(
            "{:3?} ${:04X},X @ {:04X} = {:02X}",
            instr, operand_bytes, addr, data
        ),
        AddressingMode::AbsY => format!(
            "{:3?} ${:04X},Y @ {:04X} = {:02X}",
            instr, operand_bytes, addr, data
        ),
        AddressingMode::AbsInd => format!("{:3?} (${:04X}) = {:04X}", instr, operand_bytes, addr),
        AddressingMode::Imm => format!("{:3?} #${:02X}", instr, bytes[1]),
        AddressingMode::ZP => format!("{:3?} ${:02X} = {:02X}", instr, addr, data),
        AddressingMode::ZPX => format!(
            "{:3?} ${:02X},X @ {:02X} = {:02X}",
            instr, bytes[1], addr, data
        ),
        AddressingMode::ZPY => format!(
            "{:3?} ${:02X},Y @ {:02X} = {:02X}",
            instr, bytes[1], addr, data
        ),
        AddressingMode::Impl => format!("{:3?}", instr),
        AddressingMode::Rel => format!("{:3?} ${:04X}", instr, addr),
        AddressingMode::Accum => format!("{:3?} A", instr),
        AddressingMode::IndX => {
            let sum = reg!(get x, mb).wrapping_add(bytes[1]);
            format!(
                "{:3?} (${:02X},X) @ {:02X} = {:04X} = {:02X}",
                instr, bytes[1], sum, addr, data
            )
        }
        AddressingMode::IndY => {
            let ind = bytes_to_addr!(
                mb.peek(u16::from(bytes[1])).unwrap_or(0xA5),
                mb.peek(0xFF & (u16::from(bytes[1]) + 1)).unwrap_or(0xA5)
            );
            format!(
                "{:3?} (${:02X}),Y = {:04X} @ {:04X} = {:02X}",
                instr, bytes[1], ind, addr, data
            )
        }
    };
    format!(
        //PC     Ops   Inst Accum    X reg    Y reg    Status   Stack     PPU.row...line  tot_cycles
        "{:04X}  {:8}  {:32}A:{:02X} X:{:02X} Y:{:02X} P:{:02X} SP:{:02X} PPU:{:>3},{:>3} CYC:{}",
        reg!(get pc, mb),
        ops,
        instr,
        reg!(get acc, mb),
        reg!(get x, mb),
        reg!(get y, mb),
        reg!(get status, mb),
        reg!(get stack, mb),
        0,
        0,
        reg!(get tot_cycles, mb)
    )
}

macro_rules! illegal_opcode {
    ( $opcode: expr, $mnemonic: expr, $addressingMode: expr ) => {{
        eprintln!("Invalid opcode: {:02X} ({})", $opcode, $mnemonic);
        ($addressingMode, Instruction::NOP)
    }};
}

macro_rules! unmapped_opcode {
    ($opcode: expr) => {{
        eprintln!("Unsupported opcode used: {:02X}", $opcode);
        (AddressingMode::Impl, Instruction::NOP)
    }};
}

#[inline]
pub fn decode_instruction(instr: u8) -> (AddressingMode, Instruction) {
    // and now for a great big mess of generated code
    // never before in my life would I have thought generating code with Excel
    // was a good idea
    //
    // but here I am, two months into a global pandemic
    //
    // and even the cleanest opcode table I could find needed manual adjustment
    //
    // I need a shower
    match instr {
        // Macros delinate illegal and unmapped opcodes

        // 0x0_
        0x00 => (AddressingMode::Impl, Instruction::BRK),
        0x01 => (AddressingMode::IndX, Instruction::ORA),
        0x03 => illegal_opcode!(instr, "SLO", AddressingMode::IndX),
        0x04 => (AddressingMode::ZP, Instruction::NOP),
        0x05 => (AddressingMode::ZP, Instruction::ORA),
        0x06 => (AddressingMode::ZP, Instruction::ASL),
        0x07 => illegal_opcode!(instr, "SLO", AddressingMode::ZP),
        0x08 => (AddressingMode::Impl, Instruction::PHP),
        0x09 => (AddressingMode::Imm, Instruction::ORA),
        0x0A => (AddressingMode::Accum, Instruction::ASL),
        0x0B => illegal_opcode!(instr, "ANC", AddressingMode::Imm),
        0x0C => (AddressingMode::Abs, Instruction::NOP),
        0x0D => (AddressingMode::Abs, Instruction::ORA),
        0x0E => (AddressingMode::Abs, Instruction::ASL),
        0x0F => illegal_opcode!(instr, "SLO", AddressingMode::Abs),

        // 0x1_
        0x10 => (AddressingMode::Rel, Instruction::BPL),
        0x11 => (AddressingMode::IndY, Instruction::ORA),
        0x13 => illegal_opcode!(instr, "SLO", AddressingMode::IndY),
        0x14 => (AddressingMode::ZPX, Instruction::NOP),
        0x15 => (AddressingMode::ZPX, Instruction::ORA),
        0x16 => (AddressingMode::ZPX, Instruction::ASL),
        0x17 => illegal_opcode!(instr, "SLO", AddressingMode::ZPX),
        0x18 => (AddressingMode::Impl, Instruction::CLC),
        0x19 => (AddressingMode::AbsY, Instruction::ORA),
        0x1A => (AddressingMode::Impl, Instruction::NOP), // unofficial dup
        0x1B => illegal_opcode!(instr, "SLO", AddressingMode::AbsY),
        0x1C => (AddressingMode::AbsX, Instruction::NOP),
        0x1D => (AddressingMode::AbsX, Instruction::ORA),
        0x1E => (AddressingMode::AbsX, Instruction::ASL),
        0x1F => illegal_opcode!(instr, "SLO", AddressingMode::AbsX),

        // 0x2_
        0x20 => (AddressingMode::Abs, Instruction::JSR),
        0x21 => (AddressingMode::IndX, Instruction::AND),
        0x23 => illegal_opcode!(instr, "RLA", AddressingMode::IndX),
        0x24 => (AddressingMode::ZP, Instruction::BIT),
        0x25 => (AddressingMode::ZP, Instruction::AND),
        0x26 => (AddressingMode::ZP, Instruction::ROL),
        0x27 => illegal_opcode!(instr, "RLA", AddressingMode::ZP),
        0x28 => (AddressingMode::Impl, Instruction::PLP),
        0x29 => (AddressingMode::Imm, Instruction::AND),
        0x2A => (AddressingMode::Accum, Instruction::ROL),
        0x2B => illegal_opcode!(instr, "ANC", AddressingMode::Imm),
        0x2C => (AddressingMode::Abs, Instruction::BIT),
        0x2D => (AddressingMode::Abs, Instruction::AND),
        0x2E => (AddressingMode::Abs, Instruction::ROL),
        0x2F => illegal_opcode!(instr, "RLA", AddressingMode::Abs),

        // 0x3_
        0x30 => (AddressingMode::Rel, Instruction::BMI),
        0x31 => (AddressingMode::IndY, Instruction::AND),
        0x33 => illegal_opcode!(instr, "RLA", AddressingMode::IndY),
        0x34 => (AddressingMode::ZPX, Instruction::NOP),
        0x35 => (AddressingMode::ZPX, Instruction::AND),
        0x36 => (AddressingMode::ZPX, Instruction::ROL),
        0x37 => illegal_opcode!(instr, "RLA", AddressingMode::ZPX),
        0x38 => (AddressingMode::Impl, Instruction::SEC),
        0x39 => (AddressingMode::AbsY, Instruction::AND),
        0x3A => (AddressingMode::Impl, Instruction::NOP), // unofficial dup
        0x3B => illegal_opcode!(instr, "RLA", AddressingMode::AbsY),
        0x3C => (AddressingMode::AbsX, Instruction::NOP),
        0x3D => (AddressingMode::AbsX, Instruction::AND),
        0x3E => (AddressingMode::AbsX, Instruction::ROL),
        0x3F => illegal_opcode!(instr, "RLA", AddressingMode::AbsX),

        // 0x4_
        0x40 => (AddressingMode::Impl, Instruction::RTI),
        0x41 => (AddressingMode::IndX, Instruction::EOR),
        0x43 => illegal_opcode!(instr, "SRE", AddressingMode::IndX),
        0x44 => (AddressingMode::ZP, Instruction::NOP),
        0x45 => (AddressingMode::ZP, Instruction::EOR),
        0x46 => (AddressingMode::ZP, Instruction::LSR),
        0x47 => illegal_opcode!(instr, "SRE", AddressingMode::ZP),
        0x48 => (AddressingMode::Impl, Instruction::PHA),
        0x49 => (AddressingMode::Imm, Instruction::EOR),
        0x4A => (AddressingMode::Accum, Instruction::LSR),
        0x4B => illegal_opcode!(instr, "ALR", AddressingMode::Imm),
        0x4C => (AddressingMode::Abs, Instruction::JMP),
        0x4D => (AddressingMode::Abs, Instruction::EOR),
        0x4E => (AddressingMode::Abs, Instruction::LSR),
        0x4F => illegal_opcode!(instr, "SRE", AddressingMode::Abs),

        // 0x5_
        0x50 => (AddressingMode::Rel, Instruction::BVC),
        0x51 => (AddressingMode::IndY, Instruction::EOR),
        0x53 => illegal_opcode!(instr, "SRE", AddressingMode::IndY),
        0x54 => (AddressingMode::ZPX, Instruction::NOP),
        0x55 => (AddressingMode::ZPX, Instruction::EOR),
        0x56 => (AddressingMode::ZPX, Instruction::LSR),
        0x57 => illegal_opcode!(instr, "SRE", AddressingMode::ZPX),
        0x58 => (AddressingMode::Impl, Instruction::CLI),
        0x59 => (AddressingMode::AbsY, Instruction::EOR),
        0x5A => (AddressingMode::Impl, Instruction::NOP), // unofficial dup
        0x5B => illegal_opcode!(instr, "SRE", AddressingMode::AbsY),
        0x5C => (AddressingMode::AbsX, Instruction::NOP),
        0x5D => (AddressingMode::AbsX, Instruction::EOR),
        0x5E => (AddressingMode::AbsX, Instruction::LSR),
        0x5F => illegal_opcode!(instr, "SRE", AddressingMode::AbsX),

        // 0x6_
        0x60 => (AddressingMode::Impl, Instruction::RTS),
        0x61 => (AddressingMode::IndX, Instruction::ADC),
        0x63 => illegal_opcode!(instr, "RRA", AddressingMode::IndX),
        0x64 => (AddressingMode::ZP, Instruction::NOP),
        0x65 => (AddressingMode::ZP, Instruction::ADC),
        0x66 => (AddressingMode::ZP, Instruction::ROR),
        0x67 => illegal_opcode!(instr, "RRA", AddressingMode::ZP),
        0x68 => (AddressingMode::Impl, Instruction::PLA),
        0x69 => (AddressingMode::Imm, Instruction::ADC),
        0x6A => (AddressingMode::Accum, Instruction::ROR),
        0x6B => illegal_opcode!(instr, "ARR", AddressingMode::Imm),
        0x6C => (AddressingMode::AbsInd, Instruction::JMP),
        0x6D => (AddressingMode::Abs, Instruction::ADC),
        0x6E => (AddressingMode::Abs, Instruction::ROR),
        0x6F => illegal_opcode!(instr, "RRA", AddressingMode::Abs),

        // 0x7_
        0x70 => (AddressingMode::Rel, Instruction::BVS),
        0x71 => (AddressingMode::IndY, Instruction::ADC),
        0x73 => illegal_opcode!(instr, "RRA", AddressingMode::IndY),
        0x74 => (AddressingMode::ZPX, Instruction::NOP),
        0x75 => (AddressingMode::ZPX, Instruction::ADC),
        0x76 => (AddressingMode::ZPX, Instruction::ROR),
        0x77 => illegal_opcode!(instr, "RRA", AddressingMode::ZPX),
        0x78 => (AddressingMode::Impl, Instruction::SEI),
        0x79 => (AddressingMode::AbsY, Instruction::ADC),
        0x7A => (AddressingMode::Impl, Instruction::NOP), // unofficial dup
        0x7B => illegal_opcode!(instr, "RRA", AddressingMode::AbsY),
        0x7C => (AddressingMode::AbsX, Instruction::NOP),
        0x7D => (AddressingMode::AbsX, Instruction::ADC),
        0x7E => (AddressingMode::AbsX, Instruction::ROR),
        0x7F => illegal_opcode!(instr, "RRA", AddressingMode::AbsX),

        // 0x8_
        0x80 => (AddressingMode::Imm, Instruction::NOP),
        0x81 => (AddressingMode::IndX, Instruction::STA),
        0x82 => (AddressingMode::Imm, Instruction::NOP),
        0x83 => illegal_opcode!(instr, "SAX", AddressingMode::IndX),
        0x84 => (AddressingMode::ZP, Instruction::STY),
        0x85 => (AddressingMode::ZP, Instruction::STA),
        0x86 => (AddressingMode::ZP, Instruction::STX),
        0x87 => illegal_opcode!(instr, "SAX", AddressingMode::ZP),
        0x88 => (AddressingMode::Impl, Instruction::DEY),
        0x89 => (AddressingMode::Imm, Instruction::NOP),
        0x8A => (AddressingMode::Impl, Instruction::TXA),
        0x8B => illegal_opcode!(instr, "XAA", AddressingMode::Imm),
        0x8C => (AddressingMode::Abs, Instruction::STY),
        0x8D => (AddressingMode::Abs, Instruction::STA),
        0x8E => (AddressingMode::Abs, Instruction::STX),
        0x8F => illegal_opcode!(instr, "SAX", AddressingMode::Abs),

        // 0x9_
        0x90 => (AddressingMode::Rel, Instruction::BCC),
        0x91 => (AddressingMode::IndY, Instruction::STA),
        0x93 => illegal_opcode!(instr, "AHX", AddressingMode::IndY),
        0x94 => (AddressingMode::ZPX, Instruction::STY),
        0x95 => (AddressingMode::ZPX, Instruction::STA),
        0x96 => (AddressingMode::ZPY, Instruction::STX),
        0x97 => illegal_opcode!(instr, "SAX", AddressingMode::ZPY),
        0x98 => (AddressingMode::Impl, Instruction::TYA),
        0x99 => (AddressingMode::AbsY, Instruction::STA),
        0x9A => (AddressingMode::Impl, Instruction::TXS),
        0x9B => illegal_opcode!(instr, "TAS", AddressingMode::AbsY),
        0x9C => illegal_opcode!(instr, "SHY", AddressingMode::AbsX),
        0x9D => (AddressingMode::AbsX, Instruction::STA),
        0x9E => illegal_opcode!(instr, "SHX", AddressingMode::AbsY),
        0x9F => illegal_opcode!(instr, "AHX", AddressingMode::AbsY),

        // 0xA_
        0xA0 => (AddressingMode::Imm, Instruction::LDY),
        0xA1 => (AddressingMode::IndX, Instruction::LDA),
        0xA2 => (AddressingMode::Imm, Instruction::LDX),
        0xA3 => illegal_opcode!(instr, "LAX", AddressingMode::IndX),
        0xA4 => (AddressingMode::ZP, Instruction::LDY),
        0xA5 => (AddressingMode::ZP, Instruction::LDA),
        0xA6 => (AddressingMode::ZP, Instruction::LDX),
        0xA7 => illegal_opcode!(instr, "LAX", AddressingMode::ZP),
        0xA8 => (AddressingMode::Impl, Instruction::TAY),
        0xA9 => (AddressingMode::Imm, Instruction::LDA),
        0xAA => (AddressingMode::Impl, Instruction::TAX),
        0xAB => illegal_opcode!(instr, "LAX", AddressingMode::Imm),
        0xAC => (AddressingMode::Abs, Instruction::LDY),
        0xAD => (AddressingMode::Abs, Instruction::LDA),
        0xAE => (AddressingMode::Abs, Instruction::LDX),
        0xAF => illegal_opcode!(instr, "LAX", AddressingMode::Abs),

        // 0xB_
        0xB0 => (AddressingMode::Rel, Instruction::BCS),
        0xB1 => (AddressingMode::IndY, Instruction::LDA),
        0xB3 => illegal_opcode!(instr, "LAX", AddressingMode::IndY),
        0xB4 => (AddressingMode::ZPX, Instruction::LDY),
        0xB5 => (AddressingMode::ZPX, Instruction::LDA),
        0xB6 => (AddressingMode::ZPY, Instruction::LDX),
        0xB7 => illegal_opcode!(instr, "LAX", AddressingMode::ZPY),
        0xB8 => (AddressingMode::Impl, Instruction::CLV),
        0xB9 => (AddressingMode::AbsY, Instruction::LDA),
        0xBA => (AddressingMode::Impl, Instruction::TSX),
        0xBB => illegal_opcode!(instr, "LAS", AddressingMode::AbsY),
        0xBC => (AddressingMode::AbsX, Instruction::LDY),
        0xBD => (AddressingMode::AbsX, Instruction::LDA),
        0xBE => (AddressingMode::AbsY, Instruction::LDX),
        0xBF => illegal_opcode!(instr, "LAX", AddressingMode::AbsY),

        // 0xC_
        0xC0 => (AddressingMode::Imm, Instruction::CPY),
        0xC1 => (AddressingMode::IndX, Instruction::CMP),
        0xC2 => (AddressingMode::Imm, Instruction::NOP),
        0xC3 => illegal_opcode!(instr, "DCP", AddressingMode::IndX),
        0xC4 => (AddressingMode::ZP, Instruction::CPY),
        0xC5 => (AddressingMode::ZP, Instruction::CMP),
        0xC6 => (AddressingMode::ZP, Instruction::DEC),
        0xC7 => illegal_opcode!(instr, "DCP", AddressingMode::ZP),
        0xC8 => (AddressingMode::Impl, Instruction::INY),
        0xC9 => (AddressingMode::Imm, Instruction::CMP),
        0xCA => (AddressingMode::Impl, Instruction::DEX),
        0xCB => illegal_opcode!(instr, "AXS", AddressingMode::Imm),
        0xCC => (AddressingMode::Abs, Instruction::CPY),
        0xCD => (AddressingMode::Abs, Instruction::CMP),
        0xCE => (AddressingMode::Abs, Instruction::DEC),
        0xCF => illegal_opcode!(instr, "DCP", AddressingMode::Abs),

        // 0xD_
        0xD0 => (AddressingMode::Rel, Instruction::BNE),
        0xD1 => (AddressingMode::IndY, Instruction::CMP),
        0xD3 => illegal_opcode!(instr, "DCP", AddressingMode::IndY),
        0xD4 => (AddressingMode::ZPX, Instruction::NOP),
        0xD5 => (AddressingMode::ZPX, Instruction::CMP),
        0xD6 => (AddressingMode::ZPX, Instruction::DEC),
        0xD7 => illegal_opcode!(instr, "DCP", AddressingMode::ZPX),
        0xD8 => (AddressingMode::Impl, Instruction::CLD),
        0xD9 => (AddressingMode::AbsY, Instruction::CMP),
        0xDA => (AddressingMode::Impl, Instruction::NOP), // unofficial dup
        0xDB => illegal_opcode!(instr, "DCP", AddressingMode::AbsY),
        0xDC => (AddressingMode::AbsX, Instruction::NOP),
        0xDD => (AddressingMode::AbsX, Instruction::CMP),
        0xDE => (AddressingMode::AbsX, Instruction::DEC),
        0xDF => illegal_opcode!(instr, "DCP", AddressingMode::AbsX),
        // 0xE_
        0xE0 => (AddressingMode::Imm, Instruction::CPX),
        0xE1 => (AddressingMode::IndX, Instruction::SBC),
        0xE2 => (AddressingMode::Imm, Instruction::NOP),
        0xE3 => illegal_opcode!(instr, "ISC", AddressingMode::IndX),
        0xE4 => (AddressingMode::ZP, Instruction::CPX),
        0xE5 => (AddressingMode::ZP, Instruction::SBC),
        0xE6 => (AddressingMode::ZP, Instruction::INC),
        0xE7 => illegal_opcode!(instr, "ISC", AddressingMode::ZP),
        0xE8 => (AddressingMode::Impl, Instruction::INX),
        0xE9 => (AddressingMode::Imm, Instruction::SBC),
        0xEA => (AddressingMode::Impl, Instruction::NOP),
        0xEB => (AddressingMode::Imm, Instruction::SBC),
        0xEC => (AddressingMode::Abs, Instruction::CPX),
        0xED => (AddressingMode::Abs, Instruction::SBC),
        0xEE => (AddressingMode::Abs, Instruction::INC),
        0xEF => illegal_opcode!(instr, "ISC", AddressingMode::Abs),

        // 0xF_
        0xF0 => (AddressingMode::Rel, Instruction::BEQ),
        0xF1 => (AddressingMode::IndY, Instruction::SBC),
        0xF3 => illegal_opcode!(instr, "ISC", AddressingMode::IndY),
        0xF4 => (AddressingMode::ZPX, Instruction::NOP),
        0xF5 => (AddressingMode::ZPX, Instruction::SBC),
        0xF6 => (AddressingMode::ZPX, Instruction::INC),
        0xF7 => illegal_opcode!(instr, "ISC", AddressingMode::ZPX),
        0xF8 => (AddressingMode::Impl, Instruction::SED),
        0xF9 => (AddressingMode::AbsY, Instruction::SBC),
        0xFA => (AddressingMode::Impl, Instruction::NOP), // unofficial dup
        0xFB => illegal_opcode!(instr, "ISC", AddressingMode::AbsY),
        0xFC => (AddressingMode::AbsX, Instruction::NOP),
        0xFD => (AddressingMode::AbsX, Instruction::SBC),
        0xFE => (AddressingMode::AbsX, Instruction::INC),
        0xFF => illegal_opcode!(instr, "ISC", AddressingMode::AbsX),

        _ => unmapped_opcode!(instr),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_instruction_correctly() {
        let res = decode_instruction(0xEA);
        assert_eq!(res.0, AddressingMode::Impl);
        assert_eq!(res.1, Instruction::NOP);
    }

    #[test]
    fn decodes_illegal_opcode_correctly() {
        let res = decode_instruction(0xFB);
        assert_eq!(res.0, AddressingMode::AbsY);
        assert_eq!(res.1, Instruction::NOP);
    }

    #[test]
    fn decodes_unmapped_opcode() {
        let res = decode_instruction(0xF2);
        assert_eq!(res.0, AddressingMode::Impl);
        assert_eq!(res.1, Instruction::NOP);
    }
}
