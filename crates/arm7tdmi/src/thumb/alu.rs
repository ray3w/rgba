use crate::{add_with_carry, shift_immediate, shift_register, ArithmeticResult, Arm7tdmi, ShiftKind};

use super::{branch_to, read_exec_reg, write_exec_reg, ExecutionResult};

pub(super) fn is_alu(opcode: u16) -> bool {
    (opcode & 0xe000) == 0x0000
        || (opcode & 0xe000) == 0x2000
        || (opcode & 0xfc00) == 0x4000
        || (opcode & 0xfc00) == 0x4400
}

pub(super) fn execute(cpu: &mut Arm7tdmi, opcode: u16, fetch_pc: u32) -> ExecutionResult {
    if (opcode & 0xe000) == 0x0000 {
        match opcode >> 11 {
            0b00000..=0b00010 => execute_shift_immediate(cpu, opcode),
            0b00011 => execute_add_sub(cpu, opcode),
            _ => unreachable!(),
        }
    } else if (opcode & 0xe000) == 0x2000 {
        execute_immediate(cpu, opcode)
    } else if (opcode & 0xfc00) == 0x4000 {
        execute_alu_op(cpu, opcode)
    } else if (opcode & 0xfc00) == 0x4400 {
        execute_high_reg(cpu, opcode, fetch_pc)
    } else {
        unreachable!()
    }
}

fn execute_shift_immediate(cpu: &mut Arm7tdmi, opcode: u16) -> ExecutionResult {
    let rd = (opcode & 0x7) as u8;
    let rs = ((opcode >> 3) & 0x7) as u8;
    let amount = ((opcode >> 6) & 0x1f) as u8;
    let kind = match (opcode >> 11) & 0x3 {
        0 => ShiftKind::Lsl,
        1 => ShiftKind::Lsr,
        2 => ShiftKind::Asr,
        _ => unreachable!(),
    };

    let result = shift_immediate(kind, cpu.read_reg(rs), amount, cpu.cpsr().carry());
    cpu.write_reg(rd, result.value);
    set_logical_flags(cpu, result.value, Some(result.carry_out));
    ExecutionResult::sequential(1)
}

fn execute_add_sub(cpu: &mut Arm7tdmi, opcode: u16) -> ExecutionResult {
    let immediate = ((opcode >> 10) & 1) != 0;
    let subtract = ((opcode >> 9) & 1) != 0;
    let operand = ((opcode >> 6) & 0x7) as u8;
    let rs = ((opcode >> 3) & 0x7) as u8;
    let rd = (opcode & 0x7) as u8;

    let lhs = cpu.read_reg(rs);
    let rhs = if immediate {
        u32::from(operand)
    } else {
        cpu.read_reg(operand)
    };

    let result = if subtract {
        add_with_carry(lhs, !rhs, true)
    } else {
        add_with_carry(lhs, rhs, false)
    };

    cpu.write_reg(rd, result.value);
    set_arithmetic_flags(cpu, result);
    ExecutionResult::sequential(1)
}

fn execute_immediate(cpu: &mut Arm7tdmi, opcode: u16) -> ExecutionResult {
    let op = (opcode >> 11) & 0x3;
    let rd = ((opcode >> 8) & 0x7) as u8;
    let imm = u32::from(opcode & 0xff);

    match op {
        0 => {
            cpu.write_reg(rd, imm);
            set_logical_flags(cpu, imm, None);
        }
        1 => {
            let result = add_with_carry(cpu.read_reg(rd), !imm, true);
            set_arithmetic_flags(cpu, result);
        }
        2 => {
            let result = add_with_carry(cpu.read_reg(rd), imm, false);
            cpu.write_reg(rd, result.value);
            set_arithmetic_flags(cpu, result);
        }
        3 => {
            let result = add_with_carry(cpu.read_reg(rd), !imm, true);
            cpu.write_reg(rd, result.value);
            set_arithmetic_flags(cpu, result);
        }
        _ => unreachable!(),
    }

    ExecutionResult::sequential(1)
}

fn execute_alu_op(cpu: &mut Arm7tdmi, opcode: u16) -> ExecutionResult {
    let op = ((opcode >> 6) & 0xf) as u8;
    let rs = ((opcode >> 3) & 0x7) as u8;
    let rd = (opcode & 0x7) as u8;

    let lhs = cpu.read_reg(rd);
    let rhs = cpu.read_reg(rs);

    match op {
        0x0 => {
            let value = lhs & rhs;
            cpu.write_reg(rd, value);
            set_logical_flags(cpu, value, None);
        }
        0x1 => {
            let value = lhs ^ rhs;
            cpu.write_reg(rd, value);
            set_logical_flags(cpu, value, None);
        }
        0x2 => {
            let result =
                shift_register(ShiftKind::Lsl, lhs, (rhs & 0xff) as u8, cpu.cpsr().carry());
            cpu.write_reg(rd, result.value);
            set_logical_flags(cpu, result.value, Some(result.carry_out));
        }
        0x3 => {
            let result =
                shift_register(ShiftKind::Lsr, lhs, (rhs & 0xff) as u8, cpu.cpsr().carry());
            cpu.write_reg(rd, result.value);
            set_logical_flags(cpu, result.value, Some(result.carry_out));
        }
        0x4 => {
            let result =
                shift_register(ShiftKind::Asr, lhs, (rhs & 0xff) as u8, cpu.cpsr().carry());
            cpu.write_reg(rd, result.value);
            set_logical_flags(cpu, result.value, Some(result.carry_out));
        }
        0x5 => {
            let result = add_with_carry(lhs, rhs, cpu.cpsr().carry());
            cpu.write_reg(rd, result.value);
            set_arithmetic_flags(cpu, result);
        }
        0x6 => {
            let result = add_with_carry(lhs, !rhs, cpu.cpsr().carry());
            cpu.write_reg(rd, result.value);
            set_arithmetic_flags(cpu, result);
        }
        0x7 => {
            let result =
                shift_register(ShiftKind::Ror, lhs, (rhs & 0xff) as u8, cpu.cpsr().carry());
            cpu.write_reg(rd, result.value);
            set_logical_flags(cpu, result.value, Some(result.carry_out));
        }
        0x8 => {
            set_logical_flags(cpu, lhs & rhs, None);
        }
        0x9 => {
            let result = add_with_carry(0, !rhs, true);
            cpu.write_reg(rd, result.value);
            set_arithmetic_flags(cpu, result);
        }
        0xa => {
            let result = add_with_carry(lhs, !rhs, true);
            set_arithmetic_flags(cpu, result);
        }
        0xb => {
            let result = add_with_carry(lhs, rhs, false);
            set_arithmetic_flags(cpu, result);
        }
        0xc => {
            let value = lhs | rhs;
            cpu.write_reg(rd, value);
            set_logical_flags(cpu, value, None);
        }
        0xd => {
            let value = lhs.wrapping_mul(rhs);
            cpu.write_reg(rd, value);
            let mut cpsr = cpu.cpsr();
            cpsr.set_nz(value);
            cpu.set_cpsr(cpsr);
        }
        0xe => {
            let value = lhs & !rhs;
            cpu.write_reg(rd, value);
            set_logical_flags(cpu, value, None);
        }
        0xf => {
            let value = !rhs;
            cpu.write_reg(rd, value);
            set_logical_flags(cpu, value, None);
        }
        _ => unreachable!(),
    }

    ExecutionResult::sequential(1)
}

fn execute_high_reg(cpu: &mut Arm7tdmi, opcode: u16, fetch_pc: u32) -> ExecutionResult {
    let op = ((opcode >> 8) & 0x3) as u8;
    let rd = ((opcode & 0x7) | (((opcode >> 4) & 0x8) as u16)) as u8;
    let rs = ((opcode >> 3) & 0xf) as u8;

    let lhs = read_exec_reg(cpu, rd, fetch_pc);
    let rhs = read_exec_reg(cpu, rs, fetch_pc);

    match op {
        0x0 => {
            let value = lhs.wrapping_add(rhs);
            let wrote_pc = write_exec_reg(cpu, rd, value);
            if wrote_pc {
                ExecutionResult::branched(2)
            } else {
                ExecutionResult::sequential(1)
            }
        }
        0x1 => {
            let result = add_with_carry(lhs, !rhs, true);
            set_arithmetic_flags(cpu, result);
            ExecutionResult::sequential(1)
        }
        0x2 => {
            let wrote_pc = write_exec_reg(cpu, rd, rhs);
            if wrote_pc {
                ExecutionResult::branched(2)
            } else {
                ExecutionResult::sequential(1)
            }
        }
        0x3 => branch_to(cpu, rhs, (rhs & 1) != 0),
        _ => unreachable!(),
    }
}

fn set_logical_flags(cpu: &mut Arm7tdmi, value: u32, carry: Option<bool>) {
    let mut cpsr = cpu.cpsr();
    cpsr.set_nz(value);
    if let Some(carry_out) = carry {
        cpsr.set_carry(carry_out);
    }
    cpu.set_cpsr(cpsr);
}

fn set_arithmetic_flags(cpu: &mut Arm7tdmi, result: ArithmeticResult) {
    let mut cpsr = cpu.cpsr();
    cpsr.set_nzcv(result.value, result.carry_out, result.overflow);
    cpu.set_cpsr(cpsr);
}

#[cfg(test)]
mod tests {
    use crate::arm::test_utils::{cpu_with_pc, FakeBus};

    fn exec(cpu: &mut crate::Arm7tdmi, opcode: u16) {
        let mut bus = FakeBus::new(64);
        cpu.set_thumb(true);
        bus.load16(cpu.pc(), opcode);
        cpu.step(&mut bus);
    }

    #[test]
    fn shift_immediate_updates_carry() {
        let mut cpu = cpu_with_pc(0);
        cpu.write_reg(1, 0x8000_0001);

        exec(&mut cpu, 0x0048); // LSLS r0, r1, #1

        assert_eq!(cpu.read_reg(0), 0x0000_0002);
        assert!(cpu.cpsr().carry());
    }

    #[test]
    fn add_sub_format_updates_flags() {
        let mut cpu = cpu_with_pc(0);
        cpu.write_reg(1, 5);
        cpu.write_reg(2, 3);

        exec(&mut cpu, 0x1888); // ADDS r0, r1, r2

        assert_eq!(cpu.read_reg(0), 8);
        assert!(!cpu.cpsr().zero());
        assert!(!cpu.cpsr().negative());
    }

    #[test]
    fn immediate_cmp_only_updates_flags() {
        let mut cpu = cpu_with_pc(0);
        cpu.write_reg(0, 7);

        exec(&mut cpu, 0x2807); // CMP r0, #7

        assert!(cpu.cpsr().zero());
        assert!(cpu.cpsr().carry());
        assert_eq!(cpu.read_reg(0), 7);
    }

    #[test]
    fn alu_neg_uses_subtract_semantics() {
        let mut cpu = cpu_with_pc(0);
        cpu.write_reg(1, 3);

        exec(&mut cpu, 0x4248); // NEG r0, r1

        assert_eq!(cpu.read_reg(0), 0xffff_fffd);
        assert!(cpu.cpsr().negative());
    }

    #[test]
    fn high_register_mov_can_write_pc() {
        let mut cpu = cpu_with_pc(0);
        cpu.write_reg(14, 0x21);

        exec(&mut cpu, 0x46f7); // MOV pc, lr

        assert_eq!(cpu.pc(), 0x20);
        assert!(cpu.is_thumb());
    }

    #[test]
    fn bx_can_switch_back_to_arm() {
        let mut cpu = cpu_with_pc(0);
        cpu.write_reg(1, 0x80);

        exec(&mut cpu, 0x4708); // BX r1

        assert_eq!(cpu.pc(), 0x80);
        assert!(!cpu.is_thumb());
    }
}
