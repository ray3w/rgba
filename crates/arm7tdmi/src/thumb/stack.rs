use crate::{Arm7tdmi, BusInterface, LR, PC};

use super::{load_word, store_word, visible_pc, write_exec_reg, ExecutionResult};

pub(super) fn is_stack(opcode: u16) -> bool {
    (opcode & 0xff00) == 0xb000 || (opcode & 0xf600) == 0xb400 || (opcode & 0xf000) == 0xc000
}

pub(super) fn execute<B: BusInterface>(
    cpu: &mut Arm7tdmi,
    bus: &mut B,
    opcode: u16,
    fetch_pc: u32,
) -> ExecutionResult {
    if (opcode & 0xff00) == 0xb000 {
        execute_add_sp(cpu, opcode)
    } else if (opcode & 0xf600) == 0xb400 {
        execute_push_pop(cpu, bus, opcode)
    } else if (opcode & 0xf000) == 0xc000 {
        execute_multiple(cpu, bus, opcode, fetch_pc)
    } else {
        unreachable!()
    }
}

fn execute_add_sp(cpu: &mut Arm7tdmi, opcode: u16) -> ExecutionResult {
    let subtract = ((opcode >> 7) & 1) != 0;
    let offset = u32::from(opcode & 0x7f) << 2;
    let sp = cpu.sp();
    cpu.set_sp(if subtract {
        sp.wrapping_sub(offset)
    } else {
        sp.wrapping_add(offset)
    });
    ExecutionResult::sequential(1)
}

fn execute_push_pop<B: BusInterface>(
    cpu: &mut Arm7tdmi,
    bus: &mut B,
    opcode: u16,
) -> ExecutionResult {
    let pop = ((opcode >> 11) & 1) != 0;
    let include_special = ((opcode >> 8) & 1) != 0;
    let reglist = opcode & 0xff;
    let count = reglist.count_ones() + u32::from(include_special);

    if count == 0 {
        return ExecutionResult::sequential(1);
    }

    if pop {
        let mut address = cpu.sp();
        let mut wrote_pc = false;

        for reg in 0..8u8 {
            if (reglist & (1 << reg)) == 0 {
                continue;
            }

            let value = load_word(bus, address);
            cpu.write_reg(reg, value);
            address = address.wrapping_add(4);
        }

        if include_special {
            let value = load_word(bus, address);
            wrote_pc = write_exec_reg(cpu, PC, value);
            address = address.wrapping_add(4);
        }

        cpu.set_sp(address);
        ExecutionResult {
            cycles: count + 1,
            wrote_pc,
        }
    } else {
        let start = cpu.sp().wrapping_sub(count * 4);
        let mut address = start;

        for reg in 0..8u8 {
            if (reglist & (1 << reg)) == 0 {
                continue;
            }

            store_word(bus, address, cpu.read_reg(reg));
            address = address.wrapping_add(4);
        }

        if include_special {
            store_word(bus, address, cpu.read_reg(LR));
        }

        cpu.set_sp(start);
        ExecutionResult::sequential(count + 1)
    }
}

fn execute_multiple<B: BusInterface>(
    cpu: &mut Arm7tdmi,
    bus: &mut B,
    opcode: u16,
    fetch_pc: u32,
) -> ExecutionResult {
    let load = ((opcode >> 11) & 1) != 0;
    let rb = ((opcode >> 8) & 0x7) as u8;
    let reglist = opcode & 0xff;
    let count = reglist.count_ones();

    let base = cpu.read_reg(rb);

    if count == 0 {
        let final_base = base.wrapping_add(0x40);
        if load {
            let value = load_word(bus, base);
            cpu.write_reg(rb, final_base);
            let wrote_pc = write_exec_reg(cpu, PC, value);
            return ExecutionResult {
                cycles: 3,
                wrote_pc,
            };
        }

        store_word(bus, base, visible_pc(fetch_pc).wrapping_add(2));
        cpu.write_reg(rb, final_base);
        return ExecutionResult::sequential(2);
    }

    let mut address = base;
    let final_base = base.wrapping_add(count * 4);

    if load {
        for reg in 0..8u8 {
            if (reglist & (1 << reg)) == 0 {
                continue;
            }

            let value = load_word(bus, address);
            cpu.write_reg(reg, value);
            address = address.wrapping_add(4);
        }
    } else {
        let first_reg = reglist.trailing_zeros() as u8;
        for reg in 0..8u8 {
            if (reglist & (1 << reg)) == 0 {
                continue;
            }

            let value = if reg == rb && reg != first_reg {
                final_base
            } else {
                cpu.read_reg(reg)
            };
            store_word(bus, address, value);
            address = address.wrapping_add(4);
        }
    }

    cpu.write_reg(rb, final_base);
    ExecutionResult::sequential(count + 1)
}

#[cfg(test)]
mod tests {
    use crate::arm::test_utils::{cpu_with_pc, FakeBus};

    fn exec(cpu: &mut crate::Arm7tdmi, bus: &mut FakeBus, opcode: u16) {
        cpu.set_thumb(true);
        bus.load16(cpu.pc(), opcode);
        cpu.step(bus);
    }

    #[test]
    fn add_and_subtract_sp_work() {
        let mut cpu = cpu_with_pc(0);
        let mut bus = FakeBus::new(64);
        cpu.set_sp(0x80);

        exec(&mut cpu, &mut bus, 0xb001); // ADD SP, #4
        assert_eq!(cpu.sp(), 0x84);

        cpu.set_pc(2);
        exec(&mut cpu, &mut bus, 0xb081); // SUB SP, #4
        assert_eq!(cpu.sp(), 0x80);
    }

    #[test]
    fn push_and_pop_with_pc_round_trip() {
        let mut cpu = cpu_with_pc(0);
        let mut bus = FakeBus::new(256);
        cpu.set_sp(0x80);
        cpu.write_reg(0, 0x1111_1111);
        cpu.write_reg(1, 0x2222_2222);
        cpu.set_lr(0x41);

        exec(&mut cpu, &mut bus, 0xb503); // PUSH {r0, r1, lr}
        assert_eq!(cpu.sp(), 0x74);

        cpu.write_reg(0, 0);
        cpu.write_reg(1, 0);
        cpu.set_pc(2);
        exec(&mut cpu, &mut bus, 0xbd03); // POP {r0, r1, pc}

        assert_eq!(cpu.read_reg(0), 0x1111_1111);
        assert_eq!(cpu.read_reg(1), 0x2222_2222);
        assert_eq!(cpu.pc(), 0x40);
        assert_eq!(cpu.sp(), 0x80);
    }

    #[test]
    fn stmia_and_ldmia_update_base() {
        let mut cpu = cpu_with_pc(0);
        let mut bus = FakeBus::new(256);
        cpu.write_reg(0, 0x40);
        cpu.write_reg(1, 0xaaaa_aaaa);
        cpu.write_reg(2, 0xbbbb_bbbb);

        exec(&mut cpu, &mut bus, 0xc006); // STMIA r0!, {r1, r2}
        assert_eq!(bus.read32(0x40), 0xaaaa_aaaa);
        assert_eq!(bus.read32(0x44), 0xbbbb_bbbb);
        assert_eq!(cpu.read_reg(0), 0x48);

        cpu.write_reg(1, 0);
        cpu.write_reg(2, 0);
        cpu.write_reg(3, 0x40);
        cpu.set_pc(2);
        exec(&mut cpu, &mut bus, 0xcb06); // LDMIA r3!, {r1, r2}
        assert_eq!(cpu.read_reg(1), 0xaaaa_aaaa);
        assert_eq!(cpu.read_reg(2), 0xbbbb_bbbb);
        assert_eq!(cpu.read_reg(3), 0x48);
    }

    #[test]
    fn ldmia_empty_rlist_loads_pc_and_increments_base_by_0x40() {
        let mut cpu = cpu_with_pc(0);
        let mut bus = FakeBus::new(256);
        cpu.set_thumb(true);
        cpu.write_reg(0, 0x40);
        bus.write32(0x40, 0x81);

        exec(&mut cpu, &mut bus, 0xc800); // LDMIA r0!, {}

        assert_eq!(cpu.pc(), 0x80);
        assert_eq!(cpu.read_reg(0), 0x80);
    }

    #[test]
    fn stmia_empty_rlist_stores_pc_and_increments_base_by_0x40() {
        let mut cpu = cpu_with_pc(0);
        let mut bus = FakeBus::new(256);
        cpu.set_thumb(true);
        cpu.write_reg(0, 0x40);

        bus.load16(0, 0xc000); // STMIA r0!, {}
        bus.load16(2, 0x4679); // MOV r1, pc
        cpu.step(&mut bus);
        cpu.step(&mut bus);

        assert_eq!(bus.read32(0x40), cpu.read_reg(1));
        assert_eq!(cpu.read_reg(0), 0x80);
    }

    #[test]
    fn stmia_with_base_in_rlist_stores_new_base_unless_base_is_first() {
        let mut cpu = cpu_with_pc(0);
        let mut bus = FakeBus::new(256);
        cpu.set_thumb(true);
        cpu.write_reg(0, 0xaaaa_aaaa);
        cpu.write_reg(1, 0x40);
        cpu.write_reg(2, 0xbbbb_bbbb);
        cpu.write_reg(3, 0xcccc_cccc);

        exec(&mut cpu, &mut bus, 0xc10f); // STMIA r1!, {r0-r3}

        assert_eq!(bus.read32(0x40), 0xaaaa_aaaa);
        assert_eq!(bus.read32(0x44), 0x50);
        assert_eq!(cpu.read_reg(1), 0x50);

        cpu.write_reg(1, 0x80);
        exec(&mut cpu, &mut bus, 0xc11e); // STMIA r1!, {r1-r4}

        assert_eq!(bus.read32(0x80), 0x80);
        assert_eq!(cpu.read_reg(1), 0x90);
    }
}
