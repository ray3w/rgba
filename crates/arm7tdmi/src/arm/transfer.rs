use crate::{shift_immediate, Arm7tdmi, BusInterface, ShiftKind, PC};

use super::{read_exec_reg, write_exec_reg, ExecutionResult};

pub(super) fn is_single_data_transfer(opcode: u32) -> bool {
    (opcode & 0x0c00_0000) == 0x0400_0000
}

pub(super) fn execute<B: BusInterface>(
    cpu: &mut Arm7tdmi,
    bus: &mut B,
    opcode: u32,
    fetch_pc: u32,
) -> ExecutionResult {
    let immediate_offset = ((opcode >> 25) & 1) == 0;
    let pre_index = ((opcode >> 24) & 1) != 0;
    let add_offset = ((opcode >> 23) & 1) != 0;
    let byte = ((opcode >> 22) & 1) != 0;
    let write_back = ((opcode >> 21) & 1) != 0;
    let load = ((opcode >> 20) & 1) != 0;
    let rn = ((opcode >> 16) & 0xf) as u8;
    let rd = ((opcode >> 12) & 0xf) as u8;

    let base = read_exec_reg(cpu, rn, fetch_pc);
    let offset = if immediate_offset {
        opcode & 0xfff
    } else {
        decode_register_offset(cpu, opcode, fetch_pc)
    };

    let indexed = if add_offset {
        base.wrapping_add(offset)
    } else {
        base.wrapping_sub(offset)
    };
    let address = if pre_index { indexed } else { base };

    let mut wrote_pc = false;

    if load {
        let value = if byte {
            u32::from(bus.read_8(address))
        } else {
            bus.read_32(address & !3).rotate_right((address & 0x3) * 8)
        };
        wrote_pc = write_exec_reg(cpu, rd, value);
    } else {
        let value = if rd == PC {
            fetch_pc.wrapping_add(12)
        } else {
            read_exec_reg(cpu, rd, fetch_pc)
        };

        if byte {
            bus.write_8(address, value as u8);
        } else {
            bus.write_32(address & !3, value);
        }
    }

    if !pre_index || write_back {
        cpu.write_reg(rn, indexed);
    }

    ExecutionResult {
        cycles: if load { 3 } else { 2 },
        wrote_pc,
    }
}

fn decode_register_offset(cpu: &Arm7tdmi, opcode: u32, fetch_pc: u32) -> u32 {
    let rm = (opcode & 0xf) as u8;
    let shift_kind = match (opcode >> 5) & 0x3 {
        0 => ShiftKind::Lsl,
        1 => ShiftKind::Lsr,
        2 => ShiftKind::Asr,
        3 => ShiftKind::Ror,
        _ => unreachable!(),
    };
    let shift_amount = ((opcode >> 7) & 0x1f) as u8;
    shift_immediate(
        shift_kind,
        read_exec_reg(cpu, rm, fetch_pc),
        shift_amount,
        false,
    )
    .value
}

#[cfg(test)]
mod tests {
    use crate::arm::test_utils::{cpu_with_pc, FakeBus};

    fn exec(cpu: &mut crate::Arm7tdmi, bus: &mut FakeBus, opcode: u32) {
        bus.load32(cpu.pc(), opcode);
        cpu.step(bus);
    }

    #[test]
    fn str_and_ldr_word_with_immediate_offsets_work() {
        let mut cpu = cpu_with_pc(0);
        let mut bus = FakeBus::new(256);
        cpu.write_reg(0, 0x80);
        cpu.write_reg(1, 0x1234_5678);

        exec(&mut cpu, &mut bus, 0xe580_1004);
        cpu.set_pc(4);
        exec(&mut cpu, &mut bus, 0xe590_2004);

        assert_eq!(cpu.read_reg(2), 0x1234_5678);
    }

    #[test]
    fn post_indexed_load_writes_back_base() {
        let mut cpu = cpu_with_pc(0);
        let mut bus = FakeBus::new(256);
        cpu.write_reg(0, 0x40);
        bus.write32(0x40, 0xdead_beef);

        exec(&mut cpu, &mut bus, 0xe490_1004);

        assert_eq!(cpu.read_reg(1), 0xdead_beef);
        assert_eq!(cpu.read_reg(0), 0x44);
    }

    #[test]
    fn register_offset_store_uses_shifted_offset() {
        let mut cpu = cpu_with_pc(0);
        let mut bus = FakeBus::new(256);
        cpu.write_reg(0, 0x20);
        cpu.write_reg(1, 0xa5a5_5a5a);
        cpu.write_reg(2, 3);

        exec(&mut cpu, &mut bus, 0xe780_1102);

        assert_eq!(bus.read32(0x2c), 0xa5a5_5a5a);
    }

    #[test]
    fn ldr_word_rotates_unaligned_reads() {
        let mut cpu = cpu_with_pc(0);
        let mut bus = FakeBus::new(256);
        cpu.write_reg(0, 0x80);
        bus.write32(0x80, 0x1122_3344);

        exec(&mut cpu, &mut bus, 0xe590_1001);

        assert_eq!(cpu.read_reg(1), 0x4411_2233);
    }
}
