use crate::{Arm7tdmi, Exception, Psr};

use super::{branch_to, visible_pc, ExecutionResult};

pub(super) fn is_branch(opcode: u16) -> bool {
    (opcode & 0xf000) == 0xd000 || (opcode & 0xf800) == 0xe000 || (opcode & 0xf000) == 0xf000
}

pub(super) fn execute(cpu: &mut Arm7tdmi, opcode: u16, fetch_pc: u32) -> ExecutionResult {
    if (opcode & 0xf000) == 0xd000 {
        execute_conditional_or_swi(cpu, opcode, fetch_pc)
    } else if (opcode & 0xf800) == 0xe000 {
        execute_unconditional_branch(cpu, opcode, fetch_pc)
    } else if (opcode & 0xf800) == 0xf000 {
        execute_bl_high(cpu, opcode, fetch_pc)
    } else if (opcode & 0xf800) == 0xf800 {
        execute_bl_low(cpu, opcode, fetch_pc)
    } else {
        unreachable!()
    }
}

fn execute_conditional_or_swi(cpu: &mut Arm7tdmi, opcode: u16, fetch_pc: u32) -> ExecutionResult {
    let cond = ((opcode >> 8) & 0xf) as u8;

    match cond {
        0xf => {
            cpu.enter_exception(Exception::SoftwareInterrupt, fetch_pc.wrapping_add(2));
            ExecutionResult::branched(2)
        }
        0xe => {
            cpu.enter_exception(Exception::UndefinedInstruction, fetch_pc.wrapping_add(2));
            ExecutionResult::branched(2)
        }
        _ => {
            if !condition_passed(cpu.cpsr(), cond) {
                return ExecutionResult::sequential(1);
            }

            let offset = sign_extend((u32::from(opcode & 0xff)) << 1, 9);
            let target = visible_pc(fetch_pc).wrapping_add(offset);
            branch_to(cpu, target, true)
        }
    }
}

fn execute_unconditional_branch(
    cpu: &mut Arm7tdmi,
    opcode: u16,
    fetch_pc: u32,
) -> ExecutionResult {
    let offset = sign_extend((u32::from(opcode & 0x7ff)) << 1, 12);
    let target = visible_pc(fetch_pc).wrapping_add(offset);
    branch_to(cpu, target, true)
}

fn execute_bl_high(cpu: &mut Arm7tdmi, opcode: u16, fetch_pc: u32) -> ExecutionResult {
    let offset = sign_extend(u32::from(opcode & 0x07ff) << 12, 23);
    let base = visible_pc(fetch_pc);
    cpu.set_lr(base.wrapping_add(offset));
    ExecutionResult::sequential(1)
}

fn execute_bl_low(cpu: &mut Arm7tdmi, opcode: u16, fetch_pc: u32) -> ExecutionResult {
    let offset = u32::from(opcode & 0x07ff) << 1;
    let target = cpu.lr().wrapping_add(offset);
    cpu.set_lr(fetch_pc.wrapping_add(2) | 1);
    branch_to(cpu, target, true)
}

fn condition_passed(cpsr: Psr, cond: u8) -> bool {
    match cond & 0xf {
        0x0 => cpsr.zero(),
        0x1 => !cpsr.zero(),
        0x2 => cpsr.carry(),
        0x3 => !cpsr.carry(),
        0x4 => cpsr.negative(),
        0x5 => !cpsr.negative(),
        0x6 => cpsr.overflow(),
        0x7 => !cpsr.overflow(),
        0x8 => cpsr.carry() && !cpsr.zero(),
        0x9 => !cpsr.carry() || cpsr.zero(),
        0xa => cpsr.negative() == cpsr.overflow(),
        0xb => cpsr.negative() != cpsr.overflow(),
        0xc => !cpsr.zero() && (cpsr.negative() == cpsr.overflow()),
        0xd => cpsr.zero() || (cpsr.negative() != cpsr.overflow()),
        _ => false,
    }
}

fn sign_extend(value: u32, bits: u8) -> u32 {
    let shift = 32 - bits;
    (((value << shift) as i32) >> shift) as u32
}

#[cfg(test)]
mod tests {
    use crate::arm::test_utils::{cpu_with_pc, FakeBus};
    use crate::Mode;

    fn exec(cpu: &mut crate::Arm7tdmi, bus: &mut FakeBus, opcode: u16) {
        cpu.set_thumb(true);
        bus.load16(cpu.pc(), opcode);
        cpu.step(bus);
    }

    #[test]
    fn conditional_branch_uses_thumb_visible_pc() {
        let mut cpu = cpu_with_pc(0);
        let mut bus = FakeBus::new(64);
        let mut cpsr = cpu.cpsr();
        cpsr.set_zero(true);
        cpu.set_cpsr(cpsr);

        exec(&mut cpu, &mut bus, 0xd001); // BEQ +2

        assert_eq!(cpu.pc(), 6);
    }

    #[test]
    fn unconditional_branch_sign_extends_offset() {
        let mut cpu = cpu_with_pc(4);
        let mut bus = FakeBus::new(64);

        exec(&mut cpu, &mut bus, 0xe002); // B +4

        assert_eq!(cpu.pc(), 12);
    }

    #[test]
    fn swi_enters_supervisor_mode_from_thumb() {
        let mut cpu = cpu_with_pc(0x20);
        let mut bus = FakeBus::new(64);

        exec(&mut cpu, &mut bus, 0xdf00); // SWI 0

        assert_eq!(cpu.mode(), Mode::Supervisor);
        assert_eq!(cpu.pc(), 0x8);
        assert_eq!(cpu.lr(), 0x22);
        assert!(!cpu.is_thumb());
    }

    #[test]
    fn bl_uses_two_halfwords() {
        let mut cpu = cpu_with_pc(0);
        let mut bus = FakeBus::new(64);
        cpu.set_thumb(true);
        bus.load16(0, 0xf000); // BL first half, offset high = 0
        bus.load16(2, 0xf802); // BL second half, offset low = 4

        cpu.step(&mut bus);
        assert_eq!(cpu.lr(), 4);
        assert_eq!(cpu.pc(), 2);

        cpu.step(&mut bus);
        assert_eq!(cpu.pc(), 8);
        assert_eq!(cpu.lr(), 5);
        assert!(cpu.is_thumb());
    }
}
