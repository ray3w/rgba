use crate::{Arm7tdmi, BusInterface, PC};

use super::{read_exec_reg, write_exec_reg, ExecutionResult};

pub(super) fn is_data_swap(opcode: u32) -> bool {
    (opcode & 0x0fb0_0ff0) == 0x0100_0090
}

pub(super) fn execute<B: BusInterface>(
    cpu: &mut Arm7tdmi,
    bus: &mut B,
    opcode: u32,
    fetch_pc: u32,
) -> ExecutionResult {
    let byte = ((opcode >> 22) & 1) != 0;
    let rn = ((opcode >> 16) & 0xf) as u8;
    let rd = ((opcode >> 12) & 0xf) as u8;
    let rm = (opcode & 0xf) as u8;

    let address = read_exec_reg(cpu, rn, fetch_pc);
    let store_value = if rm == PC {
        fetch_pc.wrapping_add(12)
    } else {
        read_exec_reg(cpu, rm, fetch_pc)
    };
    let loaded_value = if byte {
        u32::from(bus.read_8(address))
    } else {
        bus.read_32(address & !3).rotate_right((address & 0x3) * 8)
    };

    if byte {
        bus.write_8(address, store_value as u8);
    } else {
        bus.write_32(address & !3, store_value);
    }

    let wrote_pc = write_exec_reg(cpu, rd, loaded_value);
    ExecutionResult {
        cycles: 4,
        wrote_pc,
    }
}

#[cfg(test)]
mod tests {
    use crate::arm::test_utils::{cpu_with_pc, FakeBus};

    fn exec(cpu: &mut crate::Arm7tdmi, bus: &mut FakeBus, opcode: u32) {
        bus.load32(cpu.pc(), opcode);
        cpu.step(bus);
    }

    #[test]
    fn swp_word_exchanges_register_and_memory() {
        let mut cpu = cpu_with_pc(0);
        let mut bus = FakeBus::new(256);
        cpu.write_reg(0, 0x40);
        cpu.write_reg(1, 0xffff_ffff);
        bus.write32(0x40, 0x1234_5678);

        exec(&mut cpu, &mut bus, 0xe100_1091);

        assert_eq!(cpu.read_reg(1), 0x1234_5678);
        assert_eq!(bus.read32(0x40), 0xffff_ffff);
    }

    #[test]
    fn swpb_byte_zero_extends_loaded_value() {
        let mut cpu = cpu_with_pc(0);
        let mut bus = FakeBus::new(256);
        cpu.write_reg(0, 0x40);
        cpu.write_reg(1, 0xffff_ffff);
        bus.write32(0x40, 0x1122_3344);

        exec(&mut cpu, &mut bus, 0xe140_1091);

        assert_eq!(cpu.read_reg(1), 0x44);
        assert_eq!(bus.read32(0x40), 0x1122_33ff);
    }

    #[test]
    fn swp_word_reads_rotated_value_from_misaligned_address() {
        let mut cpu = cpu_with_pc(0);
        let mut bus = FakeBus::new(256);
        cpu.write_reg(0, 0x41);
        cpu.write_reg(1, 32);
        bus.write32(0x40, 64);

        exec(&mut cpu, &mut bus, 0xe100_3091);

        assert_eq!(cpu.read_reg(3), 64u32.rotate_right(8));
        assert_eq!(bus.read32(0x40), 32);
    }

    #[test]
    fn swp_with_same_source_and_destination_uses_old_source_value_for_store() {
        let mut cpu = cpu_with_pc(0);
        let mut bus = FakeBus::new(256);
        cpu.write_reg(0, 0x40);
        bus.write32(0x40, 32);
        cpu.write_reg(1, 64);

        exec(&mut cpu, &mut bus, 0xe100_1091);

        assert_eq!(cpu.read_reg(1), 32);
        assert_eq!(bus.read32(0x40), 64);
    }
}
