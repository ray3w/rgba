use crate::{Arm7tdmi, BusInterface, PC};

use super::{read_exec_reg, user_bank_mode, write_exec_reg, ExecutionResult};

pub(super) fn is_block_data_transfer(opcode: u32) -> bool {
    (opcode & 0x0e00_0000) == 0x0800_0000
}

pub(super) fn execute<B: BusInterface>(
    cpu: &mut Arm7tdmi,
    bus: &mut B,
    opcode: u32,
    fetch_pc: u32,
) -> ExecutionResult {
    let pre_index = ((opcode >> 24) & 1) != 0;
    let add_offset = ((opcode >> 23) & 1) != 0;
    let user_mode = ((opcode >> 22) & 1) != 0;
    let write_back = ((opcode >> 21) & 1) != 0;
    let load = ((opcode >> 20) & 1) != 0;
    let rn = ((opcode >> 16) & 0xf) as u8;
    let reglist = opcode & 0xffff;

    let count = reglist.count_ones();
    if count == 0 {
        return ExecutionResult::sequential(1);
    }

    let base = read_exec_reg(cpu, rn, fetch_pc);
    let start_addr = match (add_offset, pre_index) {
        (true, false) => base,
        (true, true) => base.wrapping_add(4),
        (false, false) => base.wrapping_sub((count - 1) * 4),
        (false, true) => base.wrapping_sub(count * 4),
    };
    let final_base = if add_offset {
        base.wrapping_add(count * 4)
    } else {
        base.wrapping_sub(count * 4)
    };

    let access_user_bank = user_mode && (!load || (reglist & (1 << PC)) == 0);
    let user_bank = user_bank_mode(cpu.mode());

    let mut address = start_addr;
    let mut wrote_pc = false;

    if load {
        for reg in 0..16u8 {
            if (reglist & (1 << reg)) == 0 {
                continue;
            }

            let value = bus.read_32(address);
            address = address.wrapping_add(4);

            if access_user_bank {
                cpu.write_reg_for_mode(user_bank, reg, value);
            } else {
                wrote_pc |= write_exec_reg(cpu, reg, value);
            }
        }

        if user_mode && (reglist & (1 << PC)) != 0 {
            if let Some(saved) = cpu.spsr(cpu.mode()) {
                cpu.set_cpsr(saved);
            }
        }
    } else {
        for reg in 0..16u8 {
            if (reglist & (1 << reg)) == 0 {
                continue;
            }

            let value = if access_user_bank && reg != PC {
                cpu.read_reg_for_mode(user_bank, reg)
            } else if reg == PC {
                fetch_pc.wrapping_add(12)
            } else {
                cpu.read_reg(reg)
            };

            bus.write_32(address, value);
            address = address.wrapping_add(4);
        }
    }

    if write_back {
        cpu.write_reg(rn, final_base);
    }

    ExecutionResult {
        cycles: count + 1,
        wrote_pc,
    }
}

#[cfg(test)]
mod tests {
    use crate::arm::test_utils::{cpu_with_pc, FakeBus};
    use crate::{Mode, Psr};

    fn exec(cpu: &mut crate::Arm7tdmi, bus: &mut FakeBus, opcode: u32) {
        bus.load32(cpu.pc(), opcode);
        cpu.step(bus);
    }

    #[test]
    fn stmia_stores_registers_in_ascending_order() {
        let mut cpu = cpu_with_pc(0);
        let mut bus = FakeBus::new(256);
        cpu.write_reg(0, 0x40);
        cpu.write_reg(1, 0x1111_1111);
        cpu.write_reg(2, 0x2222_2222);
        cpu.write_reg(3, 0x3333_3333);

        exec(&mut cpu, &mut bus, 0xe880_000e);

        assert_eq!(bus.read32(0x40), 0x1111_1111);
        assert_eq!(bus.read32(0x44), 0x2222_2222);
        assert_eq!(bus.read32(0x48), 0x3333_3333);
    }

    #[test]
    fn ldmdb_loads_registers_and_writes_back() {
        let mut cpu = cpu_with_pc(0);
        let mut bus = FakeBus::new(256);
        cpu.write_reg(0, 0x50);
        bus.write32(0x48, 0xaaaa_aaaa);
        bus.write32(0x4c, 0xbbbb_bbbb);

        exec(&mut cpu, &mut bus, 0xe930_0006);

        assert_eq!(cpu.read_reg(1), 0xaaaa_aaaa);
        assert_eq!(cpu.read_reg(2), 0xbbbb_bbbb);
        assert_eq!(cpu.read_reg(0), 0x48);
    }

    #[test]
    fn ldm_with_s_and_pc_restores_cpsr() {
        let mut cpu = cpu_with_pc(0);
        let mut bus = FakeBus::new(256);
        cpu.set_mode(Mode::Supervisor);
        cpu.write_reg(0, 0x40);
        bus.write32(0x40, 0x1234_5678);
        bus.write32(0x44, 0x80);

        let mut saved = Psr::new(Mode::Irq);
        saved.set_thumb(true);
        saved.set_zero(true);
        cpu.set_spsr(Mode::Supervisor, saved);

        exec(&mut cpu, &mut bus, 0xe8f0_8002);

        assert_eq!(cpu.read_reg(1), 0x1234_5678);
        assert_eq!(cpu.pc(), 0x80);
        assert_eq!(cpu.cpsr(), saved);
    }
}
