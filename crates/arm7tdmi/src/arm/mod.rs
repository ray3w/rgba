//! ARM-state instruction decode and execution.

mod block_transfer;
mod branch;
mod data_proc;
mod data_swap;
mod halfword_transfer;
mod multiply;
mod psr_transfer;
mod transfer;

use crate::{Arm7tdmi, BusInterface, Mode, Psr, PC};

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
    opcode: u32,
    fetch_pc: u32,
) -> ExecutionResult {
    let cond = (opcode >> 28) as u8;
    if !condition_passed(cpu.cpsr(), cond) {
        return ExecutionResult::sequential(1);
    }

    if branch::is_bx(opcode) {
        branch::execute_bx(cpu, opcode, fetch_pc)
    } else if multiply::is_multiply_long(opcode) {
        multiply::execute_multiply_long(cpu, opcode, fetch_pc)
    } else if multiply::is_multiply(opcode) {
        multiply::execute_multiply(cpu, opcode, fetch_pc)
    } else if data_swap::is_data_swap(opcode) {
        data_swap::execute(cpu, bus, opcode, fetch_pc)
    } else if halfword_transfer::is_halfword_transfer(opcode) {
        halfword_transfer::execute(cpu, bus, opcode, fetch_pc)
    } else if psr_transfer::is_mrs(opcode) {
        psr_transfer::execute_mrs(cpu, opcode)
    } else if psr_transfer::is_msr(opcode) {
        psr_transfer::execute_msr(cpu, opcode)
    } else if data_proc::is_data_processing(opcode) {
        data_proc::execute(cpu, opcode, fetch_pc)
    } else if transfer::is_single_data_transfer(opcode) {
        transfer::execute(cpu, bus, opcode, fetch_pc)
    } else if block_transfer::is_block_data_transfer(opcode) {
        block_transfer::execute(cpu, bus, opcode, fetch_pc)
    } else if branch::is_branch(opcode) {
        branch::execute_branch(cpu, opcode, fetch_pc)
    } else if branch::is_software_interrupt(opcode) {
        branch::execute_software_interrupt(cpu, opcode, fetch_pc)
    } else {
        panic!("unimplemented ARM opcode: 0x{opcode:08x}");
    }
}

pub(super) fn condition_passed(cpsr: Psr, cond: u8) -> bool {
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
        0xe => true,
        0xf => false,
        _ => unreachable!(),
    }
}

pub(super) fn visible_pc(fetch_pc: u32) -> u32 {
    fetch_pc.wrapping_add(8)
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
        let aligned = if cpu.is_thumb() {
            value & !1
        } else {
            value & !3
        };
        cpu.set_pc(aligned);
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

pub(super) fn user_bank_mode(current: Mode) -> Mode {
    if matches!(current, Mode::System) {
        Mode::System
    } else {
        Mode::User
    }
}

#[cfg(test)]
pub(crate) mod test_utils {
    use crate::{Arm7tdmi, BusInterface};

    #[derive(Debug, Clone)]
    pub(crate) struct FakeBus {
        mem: Vec<u8>,
    }

    impl FakeBus {
        pub(crate) fn new(size: usize) -> Self {
            Self { mem: vec![0; size] }
        }

        pub(crate) fn load32(&mut self, addr: u32, value: u32) {
            self.write_32(addr, value);
        }

        pub(crate) fn load16(&mut self, addr: u32, value: u16) {
            self.write_16(addr, value);
        }

        pub(crate) fn read32(&self, addr: u32) -> u32 {
            let b0 = self.mem[addr as usize] as u32;
            let b1 = self.mem[addr as usize + 1] as u32;
            let b2 = self.mem[addr as usize + 2] as u32;
            let b3 = self.mem[addr as usize + 3] as u32;
            b0 | (b1 << 8) | (b2 << 16) | (b3 << 24)
        }

        pub(crate) fn write32(&mut self, addr: u32, value: u32) {
            self.mem[addr as usize] = value as u8;
            self.mem[addr as usize + 1] = (value >> 8) as u8;
            self.mem[addr as usize + 2] = (value >> 16) as u8;
            self.mem[addr as usize + 3] = (value >> 24) as u8;
        }
    }

    impl BusInterface for FakeBus {
        fn read_8(&mut self, addr: u32) -> u8 {
            self.mem[addr as usize]
        }

        fn read_16(&mut self, addr: u32) -> u16 {
            let lo = self.mem[addr as usize] as u16;
            let hi = self.mem[addr as usize + 1] as u16;
            lo | (hi << 8)
        }

        fn read_32(&mut self, addr: u32) -> u32 {
            let b0 = self.mem[addr as usize] as u32;
            let b1 = self.mem[addr as usize + 1] as u32;
            let b2 = self.mem[addr as usize + 2] as u32;
            let b3 = self.mem[addr as usize + 3] as u32;
            b0 | (b1 << 8) | (b2 << 16) | (b3 << 24)
        }

        fn write_8(&mut self, addr: u32, val: u8) {
            self.mem[addr as usize] = val;
        }

        fn write_16(&mut self, addr: u32, val: u16) {
            self.mem[addr as usize] = val as u8;
            self.mem[addr as usize + 1] = (val >> 8) as u8;
        }

        fn write_32(&mut self, addr: u32, val: u32) {
            self.mem[addr as usize] = val as u8;
            self.mem[addr as usize + 1] = (val >> 8) as u8;
            self.mem[addr as usize + 2] = (val >> 16) as u8;
            self.mem[addr as usize + 3] = (val >> 24) as u8;
        }
    }

    pub(crate) fn cpu_with_pc(pc: u32) -> Arm7tdmi {
        let mut cpu = Arm7tdmi::new();
        cpu.set_pc(pc);
        cpu
    }
}
