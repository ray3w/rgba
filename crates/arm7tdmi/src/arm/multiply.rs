use crate::Arm7tdmi;

use super::{read_exec_reg, write_exec_reg, ExecutionResult};

pub(super) fn is_multiply(opcode: u32) -> bool {
    (opcode & 0x0fc0_00f0) == 0x0000_0090
}

pub(super) fn is_multiply_long(opcode: u32) -> bool {
    (opcode & 0x0f80_00f0) == 0x0080_0090
}

pub(super) fn execute_multiply(cpu: &mut Arm7tdmi, opcode: u32, fetch_pc: u32) -> ExecutionResult {
    let accumulate = ((opcode >> 21) & 1) != 0;
    let set_flags = ((opcode >> 20) & 1) != 0;
    let rd = ((opcode >> 16) & 0xf) as u8;
    let rn = ((opcode >> 12) & 0xf) as u8;
    let rs = ((opcode >> 8) & 0xf) as u8;
    let rm = (opcode & 0xf) as u8;

    let mut result =
        read_exec_reg(cpu, rm, fetch_pc).wrapping_mul(read_exec_reg(cpu, rs, fetch_pc));
    if accumulate {
        result = result.wrapping_add(read_exec_reg(cpu, rn, fetch_pc));
    }

    let wrote_pc = write_exec_reg(cpu, rd, result);
    if set_flags {
        let mut cpsr = cpu.cpsr();
        cpsr.set_nz(result);
        cpu.set_cpsr(cpsr);
    }

    ExecutionResult {
        cycles: 2,
        wrote_pc,
    }
}

pub(super) fn execute_multiply_long(
    cpu: &mut Arm7tdmi,
    opcode: u32,
    fetch_pc: u32,
) -> ExecutionResult {
    let signed = ((opcode >> 22) & 1) != 0;
    let accumulate = ((opcode >> 21) & 1) != 0;
    let set_flags = ((opcode >> 20) & 1) != 0;
    let rd_hi = ((opcode >> 16) & 0xf) as u8;
    let rd_lo = ((opcode >> 12) & 0xf) as u8;
    let rs = ((opcode >> 8) & 0xf) as u8;
    let rm = (opcode & 0xf) as u8;

    let product = if signed {
        (read_exec_reg(cpu, rm, fetch_pc) as i32 as i64)
            .wrapping_mul(read_exec_reg(cpu, rs, fetch_pc) as i32 as i64) as u64
    } else {
        u64::from(read_exec_reg(cpu, rm, fetch_pc))
            .wrapping_mul(u64::from(read_exec_reg(cpu, rs, fetch_pc)))
    };

    let accumulated = if accumulate {
        let current = (u64::from(cpu.read_reg(rd_hi)) << 32) | u64::from(cpu.read_reg(rd_lo));
        product.wrapping_add(current)
    } else {
        product
    };

    cpu.write_reg(rd_lo, accumulated as u32);
    cpu.write_reg(rd_hi, (accumulated >> 32) as u32);

    if set_flags {
        let mut cpsr = cpu.cpsr();
        cpsr.set_negative((accumulated >> 63) != 0);
        cpsr.set_zero(accumulated == 0);
        cpu.set_cpsr(cpsr);
    }

    ExecutionResult::sequential(3)
}

#[cfg(test)]
mod tests {
    use crate::arm::test_utils::{cpu_with_pc, FakeBus};

    fn exec(cpu: &mut crate::Arm7tdmi, opcode: u32) {
        let mut bus = FakeBus::new(64);
        bus.load32(cpu.pc(), opcode);
        cpu.step(&mut bus);
    }

    #[test]
    fn mul_produces_low_32_bits() {
        let mut cpu = cpu_with_pc(0);
        cpu.write_reg(1, 7);
        cpu.write_reg(2, 6);
        exec(&mut cpu, 0xe000_0291);
        assert_eq!(cpu.read_reg(0), 42);
    }

    #[test]
    fn mla_accumulates_into_result() {
        let mut cpu = cpu_with_pc(0);
        cpu.write_reg(1, 7);
        cpu.write_reg(2, 6);
        cpu.write_reg(3, 5);
        exec(&mut cpu, 0xe020_3291);
        assert_eq!(cpu.read_reg(0), 47);
    }

    #[test]
    fn umull_writes_hi_and_lo() {
        let mut cpu = cpu_with_pc(0);
        cpu.write_reg(2, 0xffff_ffff);
        cpu.write_reg(3, 2);
        exec(&mut cpu, 0xe082_1392);
        assert_eq!(cpu.read_reg(1), 0xffff_fffe);
        assert_eq!(cpu.read_reg(2), 1);
    }
}
