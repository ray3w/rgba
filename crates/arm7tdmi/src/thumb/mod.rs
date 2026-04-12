//! Thumb-state instruction decode and execution.
//!
//! Spec formats are grouped by behavior:
//! - formats 1-5   -> alu.rs
//! - formats 6-12  -> transfer.rs
//! - formats 13-15 -> stack.rs
//! - formats 16-19 -> branch.rs

mod alu;
mod branch;
mod stack;
mod transfer;

use crate::{Arm7tdmi, BusInterface, PC};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ExecutionResult {
    pub cycles: u32,
    pub wrote_pc: bool,
}

impl ExecutionResult {
    pub(crate) const fn sequential(cycles: u32) -> Self {
        Self {
            cycles,
            wrote_pc: false,
        }
    }

    pub(crate) const fn branched(cycles: u32) -> Self {
        Self {
            cycles,
            wrote_pc: true,
        }
    }
}

pub(crate) fn execute<B: BusInterface>(
    cpu: &mut Arm7tdmi,
    bus: &mut B,
    opcode: u16,
    fetch_pc: u32,
) -> ExecutionResult {
    if alu::is_alu(opcode) {
        alu::execute(cpu, opcode, fetch_pc)
    } else if transfer::is_transfer(opcode) {
        transfer::execute(cpu, bus, opcode, fetch_pc)
    } else if stack::is_stack(opcode) {
        stack::execute(cpu, bus, opcode, fetch_pc)
    } else if branch::is_branch(opcode) {
        branch::execute(cpu, opcode, fetch_pc)
    } else {
        panic!("unimplemented THUMB opcode: 0x{opcode:04x}");
    }
}

pub(super) fn visible_pc(fetch_pc: u32) -> u32 {
    fetch_pc.wrapping_add(4)
}

pub(super) fn aligned_visible_pc(fetch_pc: u32) -> u32 {
    visible_pc(fetch_pc) & !2
}

pub(super) fn read_exec_reg(cpu: &Arm7tdmi, reg: u8, fetch_pc: u32) -> u32 {
    if reg == PC {
        visible_pc(fetch_pc)
    } else {
        cpu.read_reg(reg)
    }
}

pub(super) fn write_exec_reg(cpu: &mut Arm7tdmi, reg: u8, value: u32) -> bool {
    if reg == PC {
        cpu.set_pc(value & !1);
        true
    } else {
        cpu.write_reg(reg, value);
        false
    }
}

pub(super) fn branch_to(cpu: &mut Arm7tdmi, target: u32, thumb: bool) -> ExecutionResult {
    cpu.set_thumb(thumb);
    let aligned = if thumb { target & !1 } else { target & !3 };
    cpu.set_pc(aligned);
    ExecutionResult::branched(2)
}

pub(super) fn load_word<B: BusInterface>(bus: &mut B, address: u32) -> u32 {
    bus.read_32(address & !3)
        .rotate_right((address & 0x3).wrapping_mul(8))
}

pub(super) fn store_word<B: BusInterface>(bus: &mut B, address: u32, value: u32) {
    bus.write_32(address & !3, value);
}

pub(super) fn load_halfword<B: BusInterface>(bus: &mut B, address: u32) -> u32 {
    let value = u32::from(bus.read_16(address & !1));
    if (address & 1) == 0 {
        value
    } else {
        value.rotate_right(8)
    }
}

pub(super) fn store_halfword<B: BusInterface>(bus: &mut B, address: u32, value: u16) {
    bus.write_16(address & !1, value);
}

pub(super) fn sign_extend8(value: u8) -> u32 {
    i32::from(value as i8) as u32
}

pub(super) fn sign_extend16(value: u16) -> u32 {
    i32::from(value as i16) as u32
}

#[cfg(test)]
mod tests {
    use crate::arm::test_utils::{cpu_with_pc, FakeBus};

    #[test]
    fn sequential_thumb_execution_advances_pc_by_two() {
        let mut cpu = cpu_with_pc(0);
        cpu.set_thumb(true);
        let mut bus = FakeBus::new(32);
        bus.load16(0, 0x2001); // MOVS r0, #1

        let cycles = cpu.step(&mut bus);

        assert_eq!(cycles, 1);
        assert_eq!(cpu.read_reg(0), 1);
        assert_eq!(cpu.pc(), 2);
    }
}
