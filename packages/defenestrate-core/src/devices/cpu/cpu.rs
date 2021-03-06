//! Emulator for the MOS 6502
//!
//! This does not include support for Binary Coded Decimal, which was omitted
//! on the 2A03 variant used on the NES and Famicom. Support for BCD may be
//! added later.

use std::num::Wrapping;

use super::super::bus::Motherboard;
use super::{
    structs::{AddressingMode, CpuState, Instruction, Status, POWERON_CPU_STATE},
    utils,
};
use crate::{adj_cycles, bus, bytes_to_addr, reg};

macro_rules! op_fn {
    ($mnemonic: ident, $mb: ident, $body: expr) => {
        fn $mnemonic<T: WithCpu + Motherboard>($mb: &mut T) {
            $body
        }
    };
}

pub struct Cpu6502 {
    pub state: CpuState,
    //region internal state
    // The variables below are used as internal state by the emulator, and
    // are unrelated to the 6502.
    /// The number of cycles to wait before executing the next instruction.
    ///
    /// # Note
    ///
    /// On the 6502, most instructions took longer than 1 clock cycle. Some
    /// took quite a few more, as the instruction had to read off operands
    /// from memory. This is a counter to simulate that- if not zero,
    /// `clock` will simply decrement this and continue.
    pub cycles: u32,
    /// Whether an interrupt is pending
    pub interrupt_pending: bool,
    /// Whether that interrupt was generated by an NMI (false) or IRQ (true)
    pub maskable_interrupt: bool,
    /// Whether an 'oops' cycle occurred
    pub oops_cycle: bool,
    //endregion
}

impl Cpu6502 {
    // Statics
    /// Create a new CPU, with the default power-on state
    ///
    /// # Note
    ///
    /// Default values are the NES power-up vals
    /// cf. http://wiki.nesdev.com/w/index.php/CPU_power_up_state
    pub fn new() -> Cpu6502 {
        Cpu6502 {
            state: POWERON_CPU_STATE,
            cycles: 0,
            interrupt_pending: false,
            maskable_interrupt: false,
            oops_cycle: false,
        }
    }
}

/// Trait for a device that owns a CPU, such as the motherboard or a test harness
pub trait WithCpu {
    fn cpu(&self) -> &Cpu6502;
    fn cpu_mut(&mut self) -> &mut Cpu6502;
}

pub fn tick<T: WithCpu>(mb: &mut T) -> bool {
    let cpu = mb.cpu_mut();
    if cpu.cycles > 0 {
        cpu.state.tot_cycles += 1;
        cpu.cycles -= 1;
        return false;
    }
    true
}

pub fn exec<T: WithCpu + Motherboard>(mb: &mut T) {
    run_interrupt(mb);
    let instruction = fetch_opcode(mb);
    decode_opcode(mb, instruction);
    mb.cpu_mut().state.addr = get_addr(mb, reg!(get instruction, mb));
    exec_instr(mb);
}

pub fn debug<T: WithCpu + Motherboard>(mb: &mut T) -> String {
    let old_pc = reg!(get pc, mb);
    run_interrupt(mb);
    let instruction = fetch_opcode(mb);
    decode_opcode(mb, instruction);
    mb.cpu_mut().state.addr = get_addr(mb, reg!(get instruction, mb));
    let new_pc = reg!(get pc, mb);
    reg!(set pc, mb, old_pc);
    let debug_str = format!("{}", utils::print_debug(mb));
    reg!(set pc, mb, new_pc);
    exec_instr(mb);
    debug_str
}

/// Triggers a hardware reset of the CPU
pub fn reset<T: WithCpu + Motherboard>(mb: &mut T) {
    let fst = bus!(read mb, 0xFFFC);
    let snd = bus!(read mb, 0xFFFD);
    let cpu = mb.cpu_mut();
    cpu.state.stack -= 3;
    cpu.state.status |= Status::IRQ_DISABLE;
    cpu.state.pc = bytes_to_addr!(fst, snd);
}

/// Trigger a hard interrupt (NMI)
pub fn trigger_nmi<T: WithCpu>(mb: &mut T) {
    let cpu = mb.cpu_mut();
    cpu.interrupt_pending = true;
    cpu.maskable_interrupt = false;
}

/// Trigger a maskable interrupt (IRQ)
pub fn trigger_irq<T: WithCpu>(mb: &mut T) {
    let cpu = mb.cpu_mut();
    if cpu.state.status.contains(Status::IRQ_DISABLE) {
        return; // interrupt ignored
    }
    cpu.interrupt_pending = true;
    cpu.maskable_interrupt = true;
}

/// Sets a flag in the status register
fn set_flag<T: WithCpu>(mb: &mut T, flag: Status) {
    mb.cpu_mut().state.status |= flag;
}

/// Clears a flag from the status register
fn clear_flag<T: WithCpu>(mb: &mut T, flag: Status) {
    mb.cpu_mut().state.status &= !flag;
}

/// Advance the program counter, with overflow
fn adv_pc<T: WithCpu>(mb: &mut T, increment: u16) {
    reg!(add pc, mb, increment);
}

/// Process any CPU interrupts and return whether one occurred
fn run_interrupt<T: WithCpu + Motherboard>(mb: &mut T) -> bool {
    if !mb.cpu().interrupt_pending {
        return false;
    }
    let is_maskable = mb.cpu().maskable_interrupt;
    eprintln!(
        " [INFO] CPU Interrupt: {}",
        if is_maskable { "IRQ" } else { "NMI" }
    );
    mb.cpu_mut().interrupt_pending = false;
    let addr_bytes = reg!(get pc, mb).to_le_bytes();
    push_stack(mb, addr_bytes[1]);
    push_stack(mb, addr_bytes[0]);
    clear_flag(mb, Status::BREAK);
    set_flag(mb, Status::UNUSED);
    let status = reg!(get status, mb).bits();
    push_stack(mb, status);
    let addr = if is_maskable { 0xFFFE } else { 0xFFFA };
    let addr_fst = bus!(read mb, addr);
    let addr_snd = bus!(read mb, addr.wrapping_add(1));
    reg!(set pc, mb, bytes_to_addr!(addr_fst, addr_snd));
    true
}
/// Read the next instruction word from the address bus
///
/// Instructions are read as 3 bytes, since that is the longest that a 6502
/// instruction can be (opcode + operand1 + operand2). Not all instructions will
/// use the extra bytes.
///
/// TODO: Make this read a conditional number of bytes based on the instruction
/// decode, since reads are not side-effect free
fn fetch_opcode<T: WithCpu + Motherboard>(mb: &mut T) -> u32 {
    let pc = mb.cpu().state.pc;
    // These will advance the cycle counter. If we need to make corrections
    // (eg, because an instruction isn't actually 3 bytes long), get_addr will
    // correct for that
    let opcode = bus!(read mb, pc);
    let operand1 = bus!(read mb, pc.wrapping_add(1));
    let operand2 = bus!(read mb, pc.wrapping_add(2));

    u32::from(opcode) | (u32::from(operand1) << 8) | (u32::from(operand2) << 16)
}

/// Decodes an instruction and prepares the CPU to execute it
fn decode_opcode<T: WithCpu>(mb: &mut T, instruction: u32) {
    let ops = instruction.to_le_bytes();

    let instr = utils::decode_instruction(ops[0]);
    let cpu = mb.cpu_mut();
    cpu.state.instruction = instruction;
    cpu.state.addr_mode = instr.0;
    cpu.state.instr = instr.1;
}

/// Gets the address of the operand to read from.
///
/// # Notes
///
/// This sets the `cycles` to the average whole number of cycles any
/// instruction with this addressing mode will have. Other instructions may
/// need to add or subtract to compensate, refer to the 6502 datasheet for
/// details:
///
/// http://archive.6502.org/datasheets/mos_6501-6505_mpu_preliminary_aug_1975.pdf
///
/// A note on the so-called "oops" cycle: The "oops" cycle occurs when an
/// index instruction crosses a page boundary, as the CPU reads off the high
/// byte first without checking for a carry-out. Some instructions (like all
/// the store instructions) have some special-cased behavior that the 6502
/// datasheet details. These depend on the instruction being executed, but
/// this function is the best place to
fn get_addr<T: WithCpu + Motherboard>(mb: &mut T, instruction: u32) -> u16 {
    let ops = instruction.to_le_bytes();
    // Advance the PC at _least_ 1 byte
    adv_pc(mb, 1);
    mb.cpu_mut().oops_cycle = false;

    match mb.cpu().state.addr_mode {
        AddressingMode::Abs => {
            adv_pc(mb, 2);
            bytes_to_addr!(ops[1], ops[2])
        }
        AddressingMode::AbsInd => {
            let addr_fst = bytes_to_addr!(ops[1], ops[2]);
            let addr_snd = bytes_to_addr!(ops[1].wrapping_add(1), ops[2]);
            adv_pc(mb, 2);
            let fst = bus!(read mb, addr_fst);
            let snd = bus!(read mb, addr_snd);
            bytes_to_addr!(fst, snd)
        }
        AddressingMode::AbsX => {
            let addr = bytes_to_addr!(ops[1], ops[2]).wrapping_add(u16::from(reg!(get x, mb)));
            adv_pc(mb, 2);
            if (u16::from(reg!(get x, mb)) + u16::from(ops[1])) & 0x0100 == 0x0100 {
                adj_cycles!(mb, 1); // oops cycle
                mb.cpu_mut().oops_cycle = true;
            }
            addr
        }
        AddressingMode::AbsY => {
            let addr = bytes_to_addr!(ops[1], ops[2]).wrapping_add(u16::from(reg!(get y, mb)));
            adv_pc(mb, 2);
            if (u16::from(reg!(get y, mb)) + u16::from(ops[1])) & 0x0100 == 0x0100 {
                adj_cycles!(mb, 1); // oops cycle
                mb.cpu_mut().oops_cycle = true;
            }
            addr
        }
        AddressingMode::Accum => {
            // TODO: Make addressing Optional?
            adj_cycles!(mb, -1i32);
            0x0000
        }
        AddressingMode::Imm => {
            adv_pc(mb, 1);
            adj_cycles!(mb, -1i32);
            0x0000
        }
        AddressingMode::Impl => {
            adj_cycles!(mb, -1i32);
            0x0000
        }
        AddressingMode::IndX => {
            adj_cycles!(mb, -1i32); // lop off one of the micro-ops
                                    // I know we immediately re-add it but I want cycle corrections
                                    // to be purposeful, since we're trying for clock cycle accuracy
                                    // and the clock adjustments before now have been for instruction
                                    // length. This instruction is 2 bytes long.
            adv_pc(mb, 1);
            let val = ops[1].wrapping_add(reg!(get x, mb));
            let fst = bus!(read mb, u16::from(val));
            let snd = bus!(read mb, u16::from(val.wrapping_add(1)));
            adj_cycles!(mb, 1);
            bytes_to_addr!(fst, snd)
        }
        AddressingMode::IndY => {
            adj_cycles!(mb, -1i32);
            adv_pc(mb, 1);
            let fst = bus!(read mb, u16::from(ops[1]));
            let snd = bus!(read mb, u16::from(ops[1].wrapping_add(1)));
            if (u16::from(reg!(get y, mb)) + u16::from(fst)) & 0x0100 == 0x0100 {
                adj_cycles!(mb, 1); // oops cycle
                mb.cpu_mut().oops_cycle = true;
            }
            bytes_to_addr!(fst, snd).wrapping_add(reg!(get y, mb) as u16)
        }
        AddressingMode::Rel => {
            adv_pc(mb, 1);
            adj_cycles!(mb, -1i32);
            let bytes = reg!(get pc, mb).to_le_bytes();
            // The 'offset' is _signed_, so we need to add it as a signed
            // integer.
            let fst = bytes[0];
            let snd = bytes[1];
            let addr = bytes_to_addr!(fst, snd);
            addr.wrapping_add((ops[1] as i8) as u16)
        }
        AddressingMode::ZP => {
            adv_pc(mb, 1);
            adj_cycles!(mb, -1i32);
            bytes_to_addr!(ops[1], 0u8)
        }
        AddressingMode::ZPX => {
            adv_pc(mb, 1);
            // adj_cycles!(mb, -1i32);
            bytes_to_addr!(ops[1].wrapping_add(reg!(get x, mb)), 0u8)
        }
        AddressingMode::ZPY => {
            adv_pc(mb, 1);
            adj_cycles!(mb, -1i32);
            bytes_to_addr!(ops[1].wrapping_add(reg!(get y, mb)), 0u8)
        }
    }
}

/// Read the data at the resolved address
fn read<T: WithCpu + Motherboard>(mb: &mut T) -> u8 {
    let ops = reg!(get instruction, mb).to_le_bytes();
    match reg!(get addr_mode, mb) {
        AddressingMode::Imm => ops[1],
        AddressingMode::Accum => reg!(get acc, mb),
        _ => bus!(read mb, reg!(get addr, mb)),
    }
}

/// Write the data to the resolved address
fn write<T: WithCpu + Motherboard>(mb: &mut T, data: u8) {
    adj_cycles!(mb, 1);
    mb.write(reg!(get addr, mb), data);
}

fn push_stack<T: WithCpu + Motherboard>(mb: &mut T, data: u8) {
    let addr = bytes_to_addr!(reg!(get stack, mb), 0x01u8);
    bus!(write mb, addr, data);
    reg!(sub stack, mb, 1);
}

fn pop_stack<T: WithCpu + Motherboard>(mb: &mut T) -> u8 {
    reg!(add stack, mb, 1);
    let addr = bytes_to_addr!(reg!(get stack, mb), 0x01u8);
    bus!(read mb, addr)
}

fn check_carry<T: WithCpu>(mb: &mut T, val: u16) {
    if val & 0x100 == 0x100 {
        // an overflow occured
        set_flag(mb, Status::CARRY);
    } else {
        clear_flag(mb, Status::CARRY);
    }
}

fn check_zero<T: WithCpu>(mb: &mut T, val: u8) {
    if val == 0 {
        set_flag(mb, Status::ZERO);
    } else {
        clear_flag(mb, Status::ZERO);
    }
}

fn check_overflow<T: WithCpu>(mb: &mut T, left: u8, right: u8) {
    let left = u16::from(left);
    let right = u16::from(right);
    let res = left + right;
    if ((left ^ res) & (right ^ res)) & 0x80 != 0 {
        set_flag(mb, Status::OVERFLOW);
    } else {
        clear_flag(mb, Status::OVERFLOW);
    }
}

fn check_negative<T: WithCpu>(mb: &mut T, op: u8) {
    if op & 0x80 != 0 {
        set_flag(mb, Status::NEGATIVE);
    } else {
        clear_flag(mb, Status::NEGATIVE);
    }
}

fn exec_instr<T: WithCpu + Motherboard>(mb: &mut T) {
    let handler = match_handler(reg!(get instr, mb));
    handler(mb);
}

#[allow(type_alias_bounds)] // leaving this in for self-documenting reasons
type OpcodeHandler<T: WithCpu + Motherboard> = fn(mb: &mut T);

fn match_handler<T: WithCpu + Motherboard>(mnemonic: Instruction) -> OpcodeHandler<T> {
    match mnemonic {
        Instruction::ADC => op_adc,
        Instruction::AND => op_and,
        Instruction::ASL => op_asl,
        Instruction::BIT => op_bit,
        Instruction::BPL => op_bpl,
        Instruction::BMI => op_bmi,
        Instruction::BVC => op_bvc,
        Instruction::BVS => op_bvs,
        Instruction::BCC => op_bcc,
        Instruction::BCS => op_bcs,
        Instruction::BNE => op_bne,
        Instruction::BEQ => op_beq,
        Instruction::BRK => op_brk,
        Instruction::CMP => op_cmp,
        Instruction::CPX => op_cpx,
        Instruction::CPY => op_cpy,
        Instruction::DEC => op_dec,
        Instruction::EOR => op_eor,
        Instruction::CLC => op_clc,
        Instruction::SEC => op_sec,
        Instruction::CLI => op_cli,
        Instruction::SEI => op_sei,
        Instruction::CLV => op_clv,
        Instruction::CLD => op_cld,
        Instruction::SED => op_sed,
        Instruction::INC => op_inc,
        Instruction::JMP => op_jmp,
        Instruction::JSR => op_jsr,
        Instruction::LDA => op_lda,
        Instruction::LDX => op_ldx,
        Instruction::LDY => op_ldy,
        Instruction::LSR => op_lsr,
        Instruction::NOP => op_nop,
        Instruction::ORA => op_ora,
        Instruction::TAX => op_tax,
        Instruction::TXA => op_txa,
        Instruction::DEX => op_dex,
        Instruction::INX => op_inx,
        Instruction::TAY => op_tay,
        Instruction::TYA => op_tya,
        Instruction::DEY => op_dey,
        Instruction::INY => op_iny,
        Instruction::ROL => op_rol,
        Instruction::ROR => op_ror,
        Instruction::RTI => op_rti,
        Instruction::RTS => op_rts,
        Instruction::SBC => op_sbc,
        Instruction::STA => op_sta,
        Instruction::STX => op_stx,
        Instruction::STY => op_sty,
        Instruction::TXS => op_txs,
        Instruction::TSX => op_tsx,
        Instruction::PHA => op_pha,
        Instruction::PLA => op_pla,
        Instruction::PHP => op_php,
        Instruction::PLP => op_plp,
    }
}

//region Arithmetic ops
// ADC SBC
op_fn!(op_adc, mb, {
    if reg!(get status, mb).contains(Status::DECIMAL) {
        eprintln!(" [WARN] This emulator doesn't support BCD, but the BCD flag is set");
    }
    let op = read(mb);
    let val = Wrapping(u16::from(reg!(get acc, mb)))
        + Wrapping(u16::from(op))
        + Wrapping(if reg!(get status, mb).contains(Status::CARRY) {
            1
        } else {
            0
        });
    check_carry(mb, val.0);
    check_overflow(mb, reg!(get acc, mb), op);
    reg!(set acc, mb, (0xFF & val.0) as u8);
    check_zero(mb, reg!(get acc, mb));
    check_negative(mb, reg!(get acc, mb));
});
op_fn!(op_sbc, mb, {
    if reg!(get status, mb).contains(Status::DECIMAL) {
        eprintln!(" [WARN] This emulator doesn't support BCD, but the BCD flag is set");
    }
    let op = read(mb);
    let val = Wrapping(u16::from(reg!(get acc, mb)))
        - Wrapping(u16::from(op))
        - Wrapping(if !reg!(get status, mb).contains(Status::CARRY) {
            1
        } else {
            0
        });
    check_carry(mb, !val.0);
    check_overflow(mb, reg!(get acc, mb), !op);
    reg!(set acc, mb, (0xFF & val.0) as u8);
    check_zero(mb, reg!(get acc, mb));
    check_negative(mb, reg!(get acc, mb));
});
//endregion

//region Bitwise ops
// AND BIT EOR ORA
op_fn!(op_and, mb, {
    mb.cpu_mut().state.acc &= read(mb);
    check_zero(mb, reg!(get acc, mb));
    check_negative(mb, reg!(get acc, mb));
});
op_fn!(op_bit, mb, {
    let op = read(mb);
    let res = reg!(get acc, mb) & op;
    check_zero(mb, res);
    reg!(set status, mb, Status::from_bits_truncate((reg!(get status, mb).bits() & 0x3F) | (0xC0 & op)));
});
op_fn!(op_eor, mb, {
    mb.cpu_mut().state.acc ^= read(mb);
    check_zero(mb, reg!(get acc, mb));
    check_negative(mb, reg!(get acc, mb));
});
op_fn!(op_ora, mb, {
    mb.cpu_mut().state.acc |= read(mb);
    check_zero(mb, reg!(get acc, mb));
    check_negative(mb, reg!(get acc, mb));
});
//endregion
op_fn!(op_asl, mb, {
    let op = read(mb);
    let res = u16::from(op) << 1;
    check_carry(mb, res);
    let res = (0xFF & res) as u8;
    check_zero(mb, res);
    check_negative(mb, res);
    // Cycle corrections
    match reg!(get addr_mode, mb) {
        AddressingMode::ZP => adj_cycles!(mb, 1),
        AddressingMode::ZPX => adj_cycles!(mb, 1),
        AddressingMode::Abs => adj_cycles!(mb, 1),
        AddressingMode::AbsX => adj_cycles!(mb, 2),
        _ => {}
    };
    match reg!(get addr_mode, mb) {
        AddressingMode::Accum => reg!(set acc, mb, res),
        _ => write(mb, res),
    }
});

//region Branch instructions
// BPL BMI BVC BVS BCC BCS BEQ BNE
op_fn!(op_bpl, mb, {
    if reg!(get status, mb).contains(Status::NEGATIVE) {
        return;
    }
    adj_cycles!(mb, 1);
    reg!(set pc, mb, reg!(get addr, mb));
});
op_fn!(op_bmi, mb, {
    if !reg!(get status, mb).contains(Status::NEGATIVE) {
        return;
    }
    adj_cycles!(mb, 1);
    reg!(set pc, mb, reg!(get addr, mb));
});
op_fn!(op_bvc, mb, {
    if reg!(get status, mb).contains(Status::OVERFLOW) {
        return;
    }
    adj_cycles!(mb, 1);
    reg!(set pc, mb, reg!(get addr, mb));
});
op_fn!(op_bvs, mb, {
    if !reg!(get status, mb).contains(Status::OVERFLOW) {
        return;
    }
    adj_cycles!(mb, 1);
    reg!(set pc, mb, reg!(get addr, mb));
});
op_fn!(op_bcc, mb, {
    if reg!(get status, mb).contains(Status::CARRY) {
        return;
    }
    adj_cycles!(mb, 1);
    reg!(set pc, mb, reg!(get addr, mb));
});
op_fn!(op_bcs, mb, {
    if !reg!(get status, mb).contains(Status::CARRY) {
        return;
    }
    adj_cycles!(mb, 1);
    reg!(set pc, mb, reg!(get addr, mb));
});
op_fn!(op_beq, mb, {
    if !reg!(get status, mb).contains(Status::ZERO) {
        return;
    }
    adj_cycles!(mb, 1);
    reg!(set pc, mb, reg!(get addr, mb));
});
op_fn!(op_bne, mb, {
    if reg!(get status, mb).contains(Status::ZERO) {
        return;
    }
    adj_cycles!(mb, 1);
    reg!(set pc, mb, reg!(get addr, mb));
});
//endregion
op_fn!(op_brk, mb, {
    let addr_bytes = reg!(get pc, mb).to_le_bytes();
    push_stack(mb, addr_bytes[1]);
    push_stack(mb, addr_bytes[0]);
    set_flag(mb, Status::BREAK);
    set_flag(mb, Status::UNUSED);
    let status = reg!(get status, mb).bits();
    push_stack(mb, status);
    let addr_fst = bus!(read mb, 0xFFFE);
    let addr_snd = bus!(read mb, 0xFFFF);
    reg!(set pc, mb, bytes_to_addr!(addr_fst, addr_snd));
});

//region Compare functions
// CMP CPX CPY
op_fn!(op_cmp, mb, {
    let data = read(mb);
    let res = Wrapping(reg!(get acc, mb)) - Wrapping(data);
    let acc = reg!(get acc, mb);
    mb.cpu_mut().state.status.set(Status::CARRY, acc >= data);
    check_zero(mb, res.0);
    check_negative(mb, res.0);
});
op_fn!(op_cpx, mb, {
    let data = read(mb);
    let res = Wrapping(reg!(get x, mb)) - Wrapping(data);
    let x = reg!(get x, mb);
    mb.cpu_mut().state.status.set(Status::CARRY, x >= data);
    check_zero(mb, res.0);
    check_negative(mb, res.0);
});
op_fn!(op_cpy, mb, {
    let data = read(mb);
    let res = Wrapping(reg!(get y, mb)) - Wrapping(data);
    let y = reg!(get y, mb);
    mb.cpu_mut().state.status.set(Status::CARRY, y >= data);
    check_zero(mb, res.0);
    check_negative(mb, res.0);
});
// endregion

//region Memory functions
// DEC INC LSR ROL ROR
op_fn!(op_dec, mb, {
    let op = (Wrapping(read(mb)) - Wrapping(1)).0;
    adj_cycles!(mb, 1);
    write(mb, op);
    check_zero(mb, op);
    check_negative(mb, op);
    if reg!(get addr_mode, mb) == AddressingMode::AbsX {
        adj_cycles!(mb, 1);
    }
});
op_fn!(op_inc, mb, {
    let op = (Wrapping(read(mb)) + Wrapping(1)).0;
    adj_cycles!(mb, 1);
    write(mb, op);
    check_zero(mb, op);
    check_negative(mb, op);
    if reg!(get addr_mode, mb) == AddressingMode::AbsX {
        adj_cycles!(mb, 1);
    }
});
op_fn!(op_lsr, mb, {
    // I'm doing a bit of a trick here
    // If we look at the *high* byte, then functionally there's no
    // difference between (u16 << 7) and (u8 >> 1). But by casting
    // to u16 and doing it 'backwards', we preserve the lopped off
    // bit so that we can use it to set the carry bit
    let data = u16::from(read(mb)) << 7;
    // we want the last bit for the carry -----v
    mb.cpu_mut()
        .state
        .status
        .set(Status::CARRY, data & 0x00_80 == 0x00_80);
    // throw out the extra byte now that we're done with it
    let data = data.to_be_bytes()[0];
    check_zero(mb, data);
    check_negative(mb, data);
    // Finally, since this _could_ go to the accumulator, we need to
    // check for that addressing mode
    match reg!(get addr_mode, mb) {
        AddressingMode::ZP => {
            write(mb, data);
        }
        AddressingMode::Accum => reg!(set acc, mb, data),
        _ => write(mb, data),
    };
    // cycle count correction
    match reg!(get addr_mode, mb) {
        AddressingMode::Abs => adj_cycles!(mb, 1),
        AddressingMode::AbsX => adj_cycles!(mb, 2),
        AddressingMode::ZP => adj_cycles!(mb, 1),
        AddressingMode::ZPX => adj_cycles!(mb, 1),
        _ => {}
    };
});
op_fn!(op_ror, mb, {
    // See my notes on the LSR instruction, I do a similar trick
    // here (for similar reasons)
    let data = u16::from(read(mb)) << 7
        | if reg!(get status, mb).contains(Status::CARRY) {
            0x80_00
        } else {
            0x0
        };
    mb.cpu_mut()
        .state
        .status
        .set(Status::CARRY, data & 0x00_80 == 0x00_80);
    let data = data.to_be_bytes()[0];
    check_zero(mb, data);
    check_negative(mb, data);
    // Even the caveat on addressing is the same
    match reg!(get addr_mode, mb) {
        AddressingMode::Accum => reg!(set acc, mb, data),
        _ => write(mb, data),
    };
    // cycle count correction
    match reg!(get addr_mode, mb) {
        AddressingMode::Abs => adj_cycles!(mb, 1),
        AddressingMode::AbsX => adj_cycles!(mb, 2),
        AddressingMode::ZP => adj_cycles!(mb, 1),
        AddressingMode::ZPX => adj_cycles!(mb, 1),
        _ => {}
    };
});
op_fn!(op_rol, mb, {
    let data = (u16::from(read(mb)) << 1)
        | if reg!(get status, mb).contains(Status::CARRY) {
            0x01
        } else {
            0x00
        };
    mb.cpu_mut()
        .state
        .status
        .set(Status::CARRY, data & 0x01_00 == 0x01_00);
    let data: u8 = (data & 0xFF) as u8;
    check_zero(mb, data);
    check_negative(mb, data);
    match reg!(get addr_mode, mb) {
        AddressingMode::Accum => reg!(set acc, mb, data),
        _ => write(mb, data),
    };
    // cycle count correction
    match reg!(get addr_mode, mb) {
        AddressingMode::Abs => adj_cycles!(mb, 1),
        AddressingMode::AbsX => adj_cycles!(mb, 2),
        AddressingMode::ZP => adj_cycles!(mb, 1),
        AddressingMode::ZPX => adj_cycles!(mb, 1),
        _ => {}
    };
});
//endregion

//region Flag operations
// CLC SEC CLI SEI CLV CLD SED
op_fn!(op_clc, mb, clear_flag(mb, Status::CARRY));
op_fn!(op_sec, mb, set_flag(mb, Status::CARRY));
op_fn!(op_cli, mb, clear_flag(mb, Status::IRQ_DISABLE));
op_fn!(op_sei, mb, set_flag(mb, Status::IRQ_DISABLE));
op_fn!(op_clv, mb, clear_flag(mb, Status::OVERFLOW));
op_fn!(op_cld, mb, clear_flag(mb, Status::DECIMAL));
op_fn!(op_sed, mb, set_flag(mb, Status::DECIMAL));
//endregion

//region Jumps
// JMP JSR RTI RTS
op_fn!(op_jmp, mb, {
    let addr_mode = reg!(get addr_mode, mb);
    if addr_mode != AddressingMode::Abs && addr_mode != AddressingMode::AbsInd {
        adj_cycles!(mb, 1);
    }
    reg!(set pc, mb, reg!(get addr, mb));
});
op_fn!(op_jsr, mb, {
    if reg!(get addr_mode, mb) != AddressingMode::Abs {
        adj_cycles!(mb, 1);
    }
    let addr_bytes = (reg!(get pc, mb) - 1).to_le_bytes();
    push_stack(mb, addr_bytes[1]);
    push_stack(mb, addr_bytes[0]);
    reg!(set pc, mb, reg!(get addr, mb));
    adj_cycles!(mb, 1);
});
op_fn!(op_rti, mb, {
    let flags = pop_stack(mb);
    reg!(set status, mb, Status::from_bits_truncate(flags) | Status::UNUSED);
    let fst = pop_stack(mb);
    let snd = pop_stack(mb);
    reg!(set pc, mb, bytes_to_addr!(fst, snd));
    adj_cycles!(mb, 1);
});
op_fn!(op_rts, mb, {
    let fst = pop_stack(mb);
    let snd = pop_stack(mb);
    reg!(set pc, mb, bytes_to_addr!(fst, snd).wrapping_add(1));
    adj_cycles!(mb, 2);
});
//endregion

//region Loads
op_fn!(op_lda, mb, {
    reg!(set acc, mb, read(mb));
    check_zero(mb, reg!(get acc, mb));
    check_negative(mb, reg!(get acc, mb));
});
op_fn!(op_ldx, mb, {
    reg!(set x, mb, read(mb));
    check_zero(mb, reg!(get x, mb));
    check_negative(mb, reg!(get x, mb));

    // cycle count correction
    match reg!(get addr_mode, mb) {
        AddressingMode::ZPX => adj_cycles!(mb, 1),
        AddressingMode::ZPY => adj_cycles!(mb, 1),
        _ => {}
    };
});
op_fn!(op_ldy, mb, {
    reg!(set y, mb, read(mb));
    check_zero(mb, reg!(get y, mb));
    check_negative(mb, reg!(get y, mb));
});
//endregion
op_fn!(op_nop, _mb, {
    // no operation
});

//region Register instructions
op_fn!(op_tax, mb, {
    reg!(set x, mb, reg!(get acc, mb));
    check_zero(mb, reg!(get x, mb));
    check_negative(mb, reg!(get x, mb));
});
op_fn!(op_txa, mb, {
    reg!(set acc, mb, reg!(get x, mb));
    check_zero(mb, reg!(get acc, mb));
    check_negative(mb, reg!(get acc, mb));
});
op_fn!(op_tay, mb, {
    reg!(set y, mb, reg!(get acc, mb));
    check_zero(mb, reg!(get y, mb));
    check_negative(mb, reg!(get y, mb));
});
op_fn!(op_tya, mb, {
    reg!(set acc, mb, reg!(get y, mb));
    check_zero(mb, reg!(get acc, mb));
    check_negative(mb, reg!(get acc, mb));
});
op_fn!(op_inx, mb, {
    reg!(set x, mb, (Wrapping(reg!(get x, mb)) + Wrapping(1)).0);
    check_zero(mb, reg!(get x, mb));
    check_negative(mb, reg!(get x, mb));
});
op_fn!(op_dex, mb, {
    reg!(set x, mb, (Wrapping(reg!(get x, mb)) - Wrapping(1)).0);
    check_zero(mb, reg!(get x, mb));
    check_negative(mb, reg!(get x, mb));
});
op_fn!(op_iny, mb, {
    reg!(set y, mb, (Wrapping(reg!(get y, mb)) + Wrapping(1)).0);
    check_zero(mb, reg!(get y, mb));
    check_negative(mb, reg!(get y, mb));
});
op_fn!(op_dey, mb, {
    reg!(set y, mb, (Wrapping(reg!(get y, mb)) - Wrapping(1)).0);
    check_zero(mb, reg!(get y, mb));
    check_negative(mb, reg!(get y, mb));
});
//endregion

//region Storage instruction
op_fn!(op_sta, mb, {
    write(mb, reg!(get acc, mb));
    if mb.cpu().oops_cycle {
        // undo the oops cycle since this instruction doesn't suffer from it
        adj_cycles!(mb, -1i32);
    }
    // Cycle count corrections
    match reg!(get addr_mode, mb) {
        AddressingMode::IndY => adj_cycles!(mb, 1),
        AddressingMode::AbsX => adj_cycles!(mb, 1),
        AddressingMode::AbsY => adj_cycles!(mb, 1),
        _ => {}
    };
});
op_fn!(op_stx, mb, {
    write(mb, reg!(get x, mb));

    // cycle count correction
    match reg!(get addr_mode, mb) {
        AddressingMode::ZPY => adj_cycles!(mb, 1),
        _ => {}
    };
});
op_fn!(op_sty, mb, {
    write(mb, reg!(get y, mb));
});
//endregion

//region Stack instructions
op_fn!(op_txs, mb, {
    reg!(set stack, mb, reg!(get x, mb));
});
op_fn!(op_tsx, mb, {
    reg!(set x, mb, reg!(get stack, mb));
    check_zero(mb, reg!(get x, mb));
    check_negative(mb, reg!(get x, mb));
});
op_fn!(op_pha, mb, {
    push_stack(mb, reg!(get acc, mb));
});
op_fn!(op_pla, mb, {
    reg!(set acc, mb, pop_stack(mb));
    check_zero(mb, reg!(get acc, mb));
    check_negative(mb, reg!(get acc, mb));
    adj_cycles!(mb, 1);
});
op_fn!(op_php, mb, {
    push_stack(mb, reg!(get status, mb).bits() | 0x30)
});
op_fn!(op_plp, mb, {
    reg!(set status, mb, Status::from_bits_truncate((pop_stack(mb) & 0xEF) | 0x20));
    adj_cycles!(mb, 1);
});
//endregion
