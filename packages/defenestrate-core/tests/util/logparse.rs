pub struct EmulatorState<'a> {
    pub pc: u16,
    pub instr: &'a str,
    pub disasm: &'a str,
    pub acc: u8,
    pub xreg: u8,
    pub yreg: u8,
    pub status: u8,
    pub stack: u8,
    pub ppu_col: u16,
    pub ppu_scanline: u16,
    pub cycle: u32,
}

pub fn parse_line(line: &str) -> EmulatorState {
    EmulatorState {
        pc: u16::from_str_radix(&line[0..4], 16).unwrap(),
        instr: &line[6..14],
        disasm: &line[16..48],
        acc: u8::from_str_radix(&line[50..52], 16).unwrap(),
        xreg: u8::from_str_radix(&line[55..57], 16).unwrap(),
        yreg: u8::from_str_radix(&line[60..62], 16).unwrap(),
        status: u8::from_str_radix(&line[65..67], 16).unwrap(),
        stack: u8::from_str_radix(&line[71..73], 16).unwrap(),
        ppu_col: u16::from_str_radix(&line[78..81].trim(), 10).unwrap(),
        ppu_scanline: u16::from_str_radix(&line[82..85].trim(), 10).unwrap(),
        cycle: u32::from_str_radix(&line[90..], 10).unwrap(),
    }
}

/// Test if left and right are equal to within an acceptable degree, and
/// return the delta of clock cycles.
pub fn assert_logs_eq(left: &EmulatorState, right: &EmulatorState) -> u8 {
    assert_eq!(left.pc, right.pc, "Program counter mismatch");
    assert_eq!(left.instr, right.instr, "Instruction mismatch");
    assert_eq!(left.disasm, right.disasm, "Disassembly mismatch");
    assert_eq!(left.acc, right.acc, "Accumulator mismatch");
    assert_eq!(left.xreg, right.xreg, "X register mismatch");
    assert_eq!(left.yreg, right.yreg, "Y register mismatch");
    assert_eq!(left.status, right.status, "Status register mismatch");
    assert_eq!(left.stack, right.stack, "Stack pointer mismatch");
    // disable PPU checks for now
    // assert_eq!(left.ppu_col, right.ppu_col , "PPU column counter mismatch");
    // assert_eq!(left.ppu_scanline, right.ppu_scanline , "PPU scanline counter mismatch");

    // Test that the cycle count does not deviate more than 100 cycles
    let deviation = (i64::from(left.cycle) - i64::from(right.cycle)).abs();
    assert!(deviation < 100, "Cycle count deviation");

    deviation as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_correctly() {
        let line = parse_line("D101  C1 80     CMP ($80,X) @ 80 = 0200 = 00    A:80 X:00 Y:68 P:A4 SP:FB PPU: 66, 30 CYC:3439");
        assert_eq!(line.pc, 0xD101, "Program counter mismatch");
        assert_eq!(line.instr, "C1 80   ", "Instruction mismatch");
        assert_eq!(
            line.disasm, "CMP ($80,X) @ 80 = 0200 = 00    ",
            "Disassembly mismatch"
        );
        assert_eq!(line.acc, 0x80, "Accumulator mismatch");
        assert_eq!(line.xreg, 0x00, "X register mismatch");
        assert_eq!(line.yreg, 0x68, "Y register mismatch");
        assert_eq!(line.status, 0xA4, "Status register mismatch");
        assert_eq!(line.stack, 0xFB, "Stack pointer mismatch");
        assert_eq!(line.ppu_col, 66, "PPU column counter mismatch");
        assert_eq!(line.ppu_scanline, 30, "PPU scanline counter mismatch");
        assert_eq!(line.cycle, 3439, "Cycle counter mismatch");
    }
}
