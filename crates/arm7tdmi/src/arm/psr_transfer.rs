use crate::{Arm7tdmi, Mode, Psr};

use super::{write_exec_reg, ExecutionResult};

pub(super) fn is_mrs(opcode: u32) -> bool {
    (opcode & 0x0fbf_0fff) == 0x010f_0000
}

pub(super) fn is_msr(opcode: u32) -> bool {
    (opcode & 0x0db0_f000) == 0x0120_f000 || (opcode & 0x0db0_f000) == 0x0320_f000
}

pub(super) fn execute_mrs(cpu: &mut Arm7tdmi, opcode: u32) -> ExecutionResult {
    let use_spsr = ((opcode >> 22) & 1) != 0;
    let rd = ((opcode >> 12) & 0xf) as u8;
    let value = if use_spsr {
        cpu.spsr(cpu.mode()).unwrap_or_else(|| cpu.cpsr()).bits()
    } else {
        cpu.cpsr().bits()
    };

    let wrote_pc = write_exec_reg(cpu, rd, value);
    ExecutionResult {
        cycles: 1,
        wrote_pc,
    }
}

pub(super) fn execute_msr(cpu: &mut Arm7tdmi, opcode: u32) -> ExecutionResult {
    let immediate = ((opcode >> 25) & 1) != 0;
    let use_spsr = ((opcode >> 22) & 1) != 0;
    let mut field_mask = ((opcode >> 16) & 0xf) as u8;

    if !cpu.mode().is_privileged() {
        field_mask &= 0b1000;
    }

    let source = if immediate {
        decode_immediate(opcode)
    } else {
        cpu.read_reg((opcode & 0xf) as u8)
    };

    if use_spsr {
        if let Some(current) = cpu.spsr(cpu.mode()) {
            let updated = apply_field_mask(current, source, field_mask, true);
            cpu.set_spsr(cpu.mode(), updated);
        }
    } else {
        let updated = apply_field_mask(cpu.cpsr(), source, field_mask, cpu.mode().is_privileged());
        cpu.set_cpsr(updated);
    }

    ExecutionResult::sequential(1)
}

fn decode_immediate(opcode: u32) -> u32 {
    let imm = opcode & 0xff;
    let rotate = (((opcode >> 8) & 0xf) * 2) as u32;
    imm.rotate_right(rotate)
}

fn apply_field_mask(current: Psr, source: u32, field_mask: u8, allow_control: bool) -> Psr {
    let mut bits = current.bits();

    if (field_mask & 0b1000) != 0 {
        bits = (bits & 0x00ff_ffff) | (source & 0xff00_0000);
    }
    if (field_mask & 0b0100) != 0 {
        bits = (bits & 0xff00_ffff) | (source & 0x00ff_0000);
    }
    if (field_mask & 0b0010) != 0 {
        bits = (bits & 0xffff_00ff) | (source & 0x0000_ff00);
    }
    if allow_control && (field_mask & 0b0001) != 0 {
        bits = (bits & 0xffff_ff00) | (source & 0x0000_00ff);
    }

    if Mode::from_bits((bits & 0x1f) as u8).is_none() {
        bits = (bits & !0x1f) | (current.bits() & 0x1f);
    }

    Psr::from_bits(bits)
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
    fn mrs_reads_cpsr() {
        let mut cpu = cpu_with_pc(0);
        let mut cpsr = Psr::new(Mode::Supervisor);
        cpsr.set_negative(true);
        cpu.set_cpsr(cpsr);

        exec(&mut cpu, 0xe10f_0000);
        assert_eq!(cpu.read_reg(0), cpsr.bits());
    }

    #[test]
    fn msr_register_updates_selected_cpsr_fields() {
        let mut cpu = cpu_with_pc(0);
        cpu.write_reg(0, 0xf000_0000);
        exec(&mut cpu, 0xe128_f000);

        assert!(cpu.cpsr().negative());
        assert!(cpu.cpsr().zero());
        assert!(cpu.cpsr().carry());
        assert!(cpu.cpsr().overflow());
        assert_eq!(cpu.mode(), Mode::User);
    }

    #[test]
    fn msr_immediate_can_update_spsr() {
        let mut cpu = cpu_with_pc(0);
        cpu.set_mode(Mode::Supervisor);
        cpu.set_spsr(Mode::Supervisor, Psr::new(Mode::Irq));

        exec(&mut cpu, 0xe368_f4f0);
        let spsr = cpu.spsr(Mode::Supervisor).unwrap();
        assert!(spsr.negative());
        assert!(spsr.zero());
        assert!(spsr.carry());
        assert!(spsr.overflow());
    }

    #[test]
    fn msr_cpsr_f_updates_flag_byte() {
        let mut cpu = cpu_with_pc(0);
        cpu.write_reg(0, 0xf000_0000);

        exec(&mut cpu, 0xe128_f000);

        assert_eq!(cpu.cpsr().bits() & 0xff00_0000, 0xf000_0000);
    }

    #[test]
    fn user_mode_msr_cpsr_c_does_not_change_control_bits() {
        let mut cpu = cpu_with_pc(0);
        cpu.write_reg(0, 0x0000_0013);

        exec(&mut cpu, 0xe121_f000);

        assert_eq!(cpu.mode(), Mode::User);
        assert!(!cpu.is_thumb());
    }
}
