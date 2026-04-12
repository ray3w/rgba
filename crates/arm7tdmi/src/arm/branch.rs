use crate::{Arm7tdmi, Exception};

use super::{branch_to, read_exec_reg, visible_pc, ExecutionResult};

pub(super) fn is_bx(opcode: u32) -> bool {
    (opcode & 0x0fff_fff0) == 0x012f_ff10
}

pub(super) fn is_branch(opcode: u32) -> bool {
    (opcode & 0x0e00_0000) == 0x0a00_0000
}

pub(super) fn is_software_interrupt(opcode: u32) -> bool {
    (opcode & 0x0f00_0000) == 0x0f00_0000
}

pub(super) fn execute_bx(cpu: &mut Arm7tdmi, opcode: u32, fetch_pc: u32) -> ExecutionResult {
    let rn = (opcode & 0xf) as u8;
    let target = read_exec_reg(cpu, rn, fetch_pc);
    branch_to(cpu, target, (target & 1) != 0)
}

pub(super) fn execute_branch(cpu: &mut Arm7tdmi, opcode: u32, fetch_pc: u32) -> ExecutionResult {
    if ((opcode >> 24) & 1) != 0 {
        cpu.set_lr(fetch_pc.wrapping_add(4));
    }

    let offset = (((opcode & 0x00ff_ffff) << 2) as i32) << 6 >> 6;
    let target = visible_pc(fetch_pc).wrapping_add(offset as u32);
    branch_to(cpu, target, false)
}

pub(super) fn execute_software_interrupt(
    cpu: &mut Arm7tdmi,
    _opcode: u32,
    fetch_pc: u32,
) -> ExecutionResult {
    cpu.enter_exception(Exception::SoftwareInterrupt, fetch_pc.wrapping_add(4));
    ExecutionResult::branched(2)
}

#[cfg(test)]
mod tests {
    use crate::arm::test_utils::{cpu_with_pc, FakeBus};
    use crate::Mode;

    fn exec(cpu: &mut crate::Arm7tdmi, opcode: u32) {
        let mut bus = FakeBus::new(512);
        bus.load32(cpu.pc(), opcode);
        cpu.step(&mut bus);
    }

    #[test]
    fn branch_updates_pc_from_visible_pc() {
        let mut cpu = cpu_with_pc(0);
        exec(&mut cpu, 0xea00_0001);
        assert_eq!(cpu.pc(), 12);
    }

    #[test]
    fn branch_with_link_stashes_return_address_in_lr() {
        let mut cpu = cpu_with_pc(0x100);
        exec(&mut cpu, 0xeb00_0000);
        assert_eq!(cpu.lr(), 0x104);
        assert_eq!(cpu.pc(), 0x108);
    }

    #[test]
    fn bx_can_switch_to_thumb_state() {
        let mut cpu = cpu_with_pc(0);
        cpu.write_reg(1, 0x201);
        exec(&mut cpu, 0xe12f_ff11);
        assert!(cpu.is_thumb());
        assert_eq!(cpu.pc(), 0x200);
    }

    #[test]
    fn swi_enters_supervisor_mode() {
        let mut cpu = cpu_with_pc(0x80);
        exec(&mut cpu, 0xef00_0000);
        assert_eq!(cpu.mode(), Mode::Supervisor);
        assert_eq!(cpu.pc(), 0x8);
        assert_eq!(cpu.lr(), 0x84);
    }
}
