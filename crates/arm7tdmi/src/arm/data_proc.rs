use crate::{
    add_with_carry, shift_immediate, shift_register, ArithmeticResult, Arm7tdmi, Psr, ShiftKind,
    ShiftResult, PC,
};

use super::{read_exec_reg, write_exec_reg, ExecutionResult};

pub(super) fn is_data_processing(opcode: u32) -> bool {
    (opcode & 0x0c00_0000) == 0
}

pub(super) fn execute(cpu: &mut Arm7tdmi, opcode: u32, fetch_pc: u32) -> ExecutionResult {
    let op = ((opcode >> 21) & 0xf) as u8;
    let set_flags = ((opcode >> 20) & 1) != 0;
    let rn = ((opcode >> 16) & 0xf) as u8;
    let rd = ((opcode >> 12) & 0xf) as u8;
    let register_shift = ((opcode >> 25) & 1) == 0 && ((opcode >> 4) & 1) != 0;

    let operand2 = decode_operand2(cpu, opcode, fetch_pc, register_shift);
    let rn_value = read_data_proc_reg(cpu, rn, fetch_pc, register_shift);

    let wrote_pc = match op {
        0x0 => write_result(
            cpu,
            rd,
            rn_value & operand2.value,
            set_flags,
            operand2.carry_out,
        ),
        0x1 => write_result(
            cpu,
            rd,
            rn_value ^ operand2.value,
            set_flags,
            operand2.carry_out,
        ),
        0x2 => write_arithmetic_result(
            cpu,
            rd,
            add_with_carry(rn_value, !operand2.value, true),
            set_flags,
        ),
        0x3 => write_arithmetic_result(
            cpu,
            rd,
            add_with_carry(operand2.value, !rn_value, true),
            set_flags,
        ),
        0x4 => write_arithmetic_result(
            cpu,
            rd,
            add_with_carry(rn_value, operand2.value, false),
            set_flags,
        ),
        0x5 => write_arithmetic_result(
            cpu,
            rd,
            add_with_carry(rn_value, operand2.value, cpu.cpsr().carry()),
            set_flags,
        ),
        0x6 => write_arithmetic_result(
            cpu,
            rd,
            add_with_carry(rn_value, !operand2.value, cpu.cpsr().carry()),
            set_flags,
        ),
        0x7 => write_arithmetic_result(
            cpu,
            rd,
            add_with_carry(operand2.value, !rn_value, cpu.cpsr().carry()),
            set_flags,
        ),
        0x8 => {
            update_test_flags(cpu, rd, rn_value & operand2.value, operand2.carry_out);
            false
        }
        0x9 => {
            update_test_flags(cpu, rd, rn_value ^ operand2.value, operand2.carry_out);
            false
        }
        0xa => {
            update_compare_flags(cpu, rd, add_with_carry(rn_value, !operand2.value, true));
            false
        }
        0xb => {
            update_compare_flags(cpu, rd, add_with_carry(rn_value, operand2.value, false));
            false
        }
        0xc => write_result(
            cpu,
            rd,
            rn_value | operand2.value,
            set_flags,
            operand2.carry_out,
        ),
        0xd => write_result(cpu, rd, operand2.value, set_flags, operand2.carry_out),
        0xe => write_result(
            cpu,
            rd,
            rn_value & !operand2.value,
            set_flags,
            operand2.carry_out,
        ),
        0xf => write_result(cpu, rd, !operand2.value, set_flags, operand2.carry_out),
        _ => unreachable!(),
    };

    ExecutionResult {
        cycles: 1,
        wrote_pc,
    }
}

fn decode_operand2(
    cpu: &Arm7tdmi,
    opcode: u32,
    fetch_pc: u32,
    register_shift: bool,
) -> ShiftResult {
    if ((opcode >> 25) & 1) != 0 {
        let imm8 = opcode & 0xff;
        let rotate = (((opcode >> 8) & 0xf) * 2) as u32;
        let value = imm8.rotate_right(rotate);
        let carry_out = if rotate == 0 {
            cpu.cpsr().carry()
        } else {
            (value >> 31) != 0
        };

        ShiftResult { value, carry_out }
    } else {
        let rm = (opcode & 0xf) as u8;
        let value = read_data_proc_reg(cpu, rm, fetch_pc, register_shift);
        let shift_kind = match (opcode >> 5) & 0x3 {
            0 => ShiftKind::Lsl,
            1 => ShiftKind::Lsr,
            2 => ShiftKind::Asr,
            3 => ShiftKind::Ror,
            _ => unreachable!(),
        };

        if ((opcode >> 4) & 1) != 0 {
            let rs = ((opcode >> 8) & 0xf) as u8;
            let amount = (read_data_proc_reg(cpu, rs, fetch_pc, register_shift) & 0xff) as u8;
            shift_register(shift_kind, value, amount, cpu.cpsr().carry())
        } else {
            let amount = ((opcode >> 7) & 0x1f) as u8;
            shift_immediate(shift_kind, value, amount, cpu.cpsr().carry())
        }
    }
}

fn read_data_proc_reg(cpu: &Arm7tdmi, reg: u8, fetch_pc: u32, register_shift: bool) -> u32 {
    if reg == PC && register_shift {
        fetch_pc.wrapping_add(12)
    } else {
        read_exec_reg(cpu, reg, fetch_pc)
    }
}

fn write_result(cpu: &mut Arm7tdmi, rd: u8, value: u32, set_flags: bool, carry_out: bool) -> bool {
    let wrote_pc = write_exec_reg(cpu, rd, value);

    if set_flags {
        if rd == PC {
            if let Some(saved) = cpu.spsr(cpu.mode()) {
                cpu.set_cpsr(saved);
            }
        } else {
            let mut cpsr = cpu.cpsr();
            cpsr.set_nz(value);
            cpsr.set_carry(carry_out);
            cpu.set_cpsr(cpsr);
        }
    }

    wrote_pc
}

fn write_arithmetic_result(
    cpu: &mut Arm7tdmi,
    rd: u8,
    result: ArithmeticResult,
    set_flags: bool,
) -> bool {
    let wrote_pc = write_exec_reg(cpu, rd, result.value);

    if set_flags {
        if rd == PC {
            if let Some(saved) = cpu.spsr(cpu.mode()) {
                cpu.set_cpsr(saved);
            }
        } else {
            cpu.set_cpsr(with_arithmetic_flags(cpu.cpsr(), result));
        }
    }

    wrote_pc
}

fn update_logical_flags(cpu: &mut Arm7tdmi, value: u32, carry_out: bool) {
    let mut cpsr = cpu.cpsr();
    cpsr.set_nz(value);
    cpsr.set_carry(carry_out);
    cpu.set_cpsr(cpsr);
}

fn update_test_flags(cpu: &mut Arm7tdmi, rd: u8, value: u32, carry_out: bool) {
    if rd == PC {
        if let Some(saved) = cpu.spsr(cpu.mode()) {
            cpu.set_cpsr(saved);
        }
    } else {
        update_logical_flags(cpu, value, carry_out);
    }
}

fn update_compare_flags(cpu: &mut Arm7tdmi, rd: u8, result: ArithmeticResult) {
    if rd == PC {
        if let Some(saved) = cpu.spsr(cpu.mode()) {
            cpu.set_cpsr(saved);
        }
    } else {
        cpu.set_cpsr(with_arithmetic_flags(cpu.cpsr(), result));
    }
}

fn with_arithmetic_flags(mut cpsr: Psr, result: ArithmeticResult) -> Psr {
    cpsr.set_nzcv(result.value, result.carry_out, result.overflow);
    cpsr
}

#[cfg(test)]
mod tests {
    use crate::arm::test_utils::{cpu_with_pc, FakeBus};
    use crate::{Mode, Psr};

    fn exec(cpu: &mut crate::Arm7tdmi, opcode: u32) {
        let mut bus = FakeBus::new(64);
        bus.load32(cpu.pc(), opcode);
        cpu.step(&mut bus);
    }

    #[test]
    fn mov_immediate_uses_rotated_immediate() {
        let mut cpu = cpu_with_pc(0);
        exec(&mut cpu, 0xe3a0_0123);
        assert_eq!(cpu.read_reg(0), 0xc000_0008);
    }

    #[test]
    fn add_register_shift_updates_flags() {
        let mut cpu = cpu_with_pc(0);
        cpu.write_reg(1, 2);
        cpu.write_reg(2, 1);

        exec(&mut cpu, 0xe091_0082);

        assert_eq!(cpu.read_reg(0), 4);
        assert!(!cpu.cpsr().zero());
        assert!(!cpu.cpsr().negative());
    }

    #[test]
    fn cmp_updates_flags_without_writing_destination() {
        let mut cpu = cpu_with_pc(0);
        cpu.write_reg(1, 5);
        cpu.write_reg(0, 0xdead_beef);

        exec(&mut cpu, 0xe351_0005);

        assert_eq!(cpu.read_reg(0), 0xdead_beef);
        assert!(cpu.cpsr().zero());
        assert!(cpu.cpsr().carry());
    }

    #[test]
    fn movs_pc_restores_cpsr_from_spsr() {
        let mut cpu = cpu_with_pc(0);
        let mut current = Psr::new(Mode::Supervisor);
        current.set_negative(true);
        cpu.set_cpsr(current);

        let mut saved = Psr::new(Mode::Irq);
        saved.set_thumb(true);
        saved.set_zero(true);
        cpu.set_spsr(Mode::Supervisor, saved);
        cpu.write_reg(1, 0x40);

        exec(&mut cpu, 0xe1b0_f001);

        assert_eq!(cpu.pc(), 0x40);
        assert_eq!(cpu.cpsr(), saved);
    }

    #[test]
    fn pc_used_in_register_shift_reads_as_fetch_pc_plus_twelve() {
        let mut cpu = cpu_with_pc(0);
        cpu.write_reg(0, 0);

        exec(&mut cpu, 0xe1a0_001f); // MOV r0, pc, LSL r0

        assert_eq!(cpu.read_reg(0), 12);
    }

    #[test]
    fn bad_cmp_with_rd_pc_restores_cpsr_without_flushing_pipeline() {
        let mut cpu = cpu_with_pc(0);
        cpu.write_reg(0, 0);
        cpu.write_reg_for_mode(Mode::System, 8, 32);
        cpu.write_reg_for_mode(Mode::Fiq, 8, 64);
        cpu.set_mode(Mode::Fiq);
        cpu.set_spsr(Mode::Fiq, Psr::new(Mode::System));

        let mut bus = FakeBus::new(16);
        bus.load32(0, 0xe15f_f000); // CMP pc, pc, r0
        bus.load32(4, 0xe3a0_1001); // MOV r1, #1

        cpu.step(&mut bus);
        cpu.step(&mut bus);

        assert_eq!(cpu.mode(), Mode::System);
        assert_eq!(cpu.read_reg(8), 32);
        assert_eq!(cpu.read_reg(1), 1);
        assert_eq!(cpu.pc(), 8);
    }
}
