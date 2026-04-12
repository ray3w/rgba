use crate::{Arm7tdmi, BusInterface};

use super::{
    aligned_visible_pc, load_halfword, load_word, sign_extend16, sign_extend8, store_halfword,
    store_word, write_exec_reg, ExecutionResult,
};

pub(super) fn is_transfer(opcode: u16) -> bool {
    (opcode & 0xf800) == 0x4800
        || (opcode & 0xf000) == 0x5000
        || (opcode & 0xe000) == 0x6000
        || (opcode & 0xf000) == 0x8000
        || (opcode & 0xf000) == 0x9000
        || (opcode & 0xf000) == 0xa000
}

pub(super) fn execute<B: BusInterface>(
    cpu: &mut Arm7tdmi,
    bus: &mut B,
    opcode: u16,
    fetch_pc: u32,
) -> ExecutionResult {
    if (opcode & 0xf800) == 0x4800 {
        execute_pc_relative_load(cpu, bus, opcode, fetch_pc)
    } else if (opcode & 0xf000) == 0x5000 {
        execute_register_offset(cpu, bus, opcode)
    } else if (opcode & 0xe000) == 0x6000 {
        execute_immediate_offset(cpu, bus, opcode)
    } else if (opcode & 0xf000) == 0x8000 {
        execute_halfword_immediate(cpu, bus, opcode)
    } else if (opcode & 0xf000) == 0x9000 {
        execute_sp_relative(cpu, bus, opcode)
    } else if (opcode & 0xf000) == 0xa000 {
        execute_load_address(cpu, opcode, fetch_pc)
    } else {
        unreachable!()
    }
}

fn execute_pc_relative_load<B: BusInterface>(
    cpu: &mut Arm7tdmi,
    bus: &mut B,
    opcode: u16,
    fetch_pc: u32,
) -> ExecutionResult {
    let rd = ((opcode >> 8) & 0x7) as u8;
    let address = aligned_visible_pc(fetch_pc).wrapping_add(u32::from(opcode & 0xff) << 2);
    let value = load_word(bus, address);
    let wrote_pc = write_exec_reg(cpu, rd, value);

    ExecutionResult {
        cycles: 3,
        wrote_pc,
    }
}

fn execute_register_offset<B: BusInterface>(
    cpu: &mut Arm7tdmi,
    bus: &mut B,
    opcode: u16,
) -> ExecutionResult {
    let ro = ((opcode >> 6) & 0x7) as u8;
    let rb = ((opcode >> 3) & 0x7) as u8;
    let rd = (opcode & 0x7) as u8;
    let address = cpu.read_reg(rb).wrapping_add(cpu.read_reg(ro));

    if ((opcode >> 9) & 1) == 0 {
        let load = ((opcode >> 11) & 1) != 0;
        let byte = ((opcode >> 10) & 1) != 0;

        if load {
            let value = if byte {
                u32::from(bus.read_8(address))
            } else {
                load_word(bus, address)
            };
            let wrote_pc = write_exec_reg(cpu, rd, value);
            ExecutionResult {
                cycles: 3,
                wrote_pc,
            }
        } else {
            let value = cpu.read_reg(rd);
            if byte {
                bus.write_8(address, value as u8);
            } else {
                store_word(bus, address, value);
            }
            ExecutionResult::sequential(2)
        }
    } else {
        let h = ((opcode >> 11) & 1) != 0;
        let s = ((opcode >> 10) & 1) != 0;

        match (h, s) {
            (false, false) => {
                store_halfword(bus, address, cpu.read_reg(rd) as u16);
                ExecutionResult::sequential(2)
            }
            (true, false) => {
                let value = load_halfword(bus, address);
                let wrote_pc = write_exec_reg(cpu, rd, value);
                ExecutionResult {
                    cycles: 3,
                    wrote_pc,
                }
            }
            (false, true) => {
                let value = sign_extend8(bus.read_8(address));
                let wrote_pc = write_exec_reg(cpu, rd, value);
                ExecutionResult {
                    cycles: 3,
                    wrote_pc,
                }
            }
            (true, true) => {
                let value = if (address & 1) == 0 {
                    sign_extend16(bus.read_16(address & !1))
                } else {
                    sign_extend8(bus.read_8(address))
                };
                let wrote_pc = write_exec_reg(cpu, rd, value);
                ExecutionResult {
                    cycles: 3,
                    wrote_pc,
                }
            }
        }
    }
}

fn execute_immediate_offset<B: BusInterface>(
    cpu: &mut Arm7tdmi,
    bus: &mut B,
    opcode: u16,
) -> ExecutionResult {
    let byte = ((opcode >> 12) & 1) != 0;
    let load = ((opcode >> 11) & 1) != 0;
    let offset5 = u32::from((opcode >> 6) & 0x1f);
    let rb = ((opcode >> 3) & 0x7) as u8;
    let rd = (opcode & 0x7) as u8;
    let offset = if byte { offset5 } else { offset5 << 2 };
    let address = cpu.read_reg(rb).wrapping_add(offset);

    if load {
        let value = if byte {
            u32::from(bus.read_8(address))
        } else {
            load_word(bus, address)
        };
        let wrote_pc = write_exec_reg(cpu, rd, value);
        ExecutionResult {
            cycles: 3,
            wrote_pc,
        }
    } else {
        let value = cpu.read_reg(rd);
        if byte {
            bus.write_8(address, value as u8);
        } else {
            store_word(bus, address, value);
        }
        ExecutionResult::sequential(2)
    }
}

fn execute_halfword_immediate<B: BusInterface>(
    cpu: &mut Arm7tdmi,
    bus: &mut B,
    opcode: u16,
) -> ExecutionResult {
    let load = ((opcode >> 11) & 1) != 0;
    let offset = u32::from((opcode >> 6) & 0x1f) << 1;
    let rb = ((opcode >> 3) & 0x7) as u8;
    let rd = (opcode & 0x7) as u8;
    let address = cpu.read_reg(rb).wrapping_add(offset);

    if load {
        let value = load_halfword(bus, address);
        let wrote_pc = write_exec_reg(cpu, rd, value);
        ExecutionResult {
            cycles: 3,
            wrote_pc,
        }
    } else {
        store_halfword(bus, address, cpu.read_reg(rd) as u16);
        ExecutionResult::sequential(2)
    }
}

fn execute_sp_relative<B: BusInterface>(
    cpu: &mut Arm7tdmi,
    bus: &mut B,
    opcode: u16,
) -> ExecutionResult {
    let load = ((opcode >> 11) & 1) != 0;
    let rd = ((opcode >> 8) & 0x7) as u8;
    let address = cpu.sp().wrapping_add(u32::from(opcode & 0xff) << 2);

    if load {
        let value = load_word(bus, address);
        let wrote_pc = write_exec_reg(cpu, rd, value);
        ExecutionResult {
            cycles: 3,
            wrote_pc,
        }
    } else {
        store_word(bus, address, cpu.read_reg(rd));
        ExecutionResult::sequential(2)
    }
}

fn execute_load_address(cpu: &mut Arm7tdmi, opcode: u16, fetch_pc: u32) -> ExecutionResult {
    let use_sp = ((opcode >> 11) & 1) != 0;
    let rd = ((opcode >> 8) & 0x7) as u8;
    let offset = u32::from(opcode & 0xff) << 2;
    let base = if use_sp {
        cpu.sp()
    } else {
        aligned_visible_pc(fetch_pc)
    };
    cpu.write_reg(rd, base.wrapping_add(offset));
    ExecutionResult::sequential(1)
}

#[cfg(test)]
mod tests {
    use crate::arm::test_utils::{cpu_with_pc, FakeBus};
    use crate::BusInterface;

    fn exec(cpu: &mut crate::Arm7tdmi, bus: &mut FakeBus, opcode: u16) {
        cpu.set_thumb(true);
        bus.load16(cpu.pc(), opcode);
        cpu.step(bus);
    }

    #[test]
    fn pc_relative_load_uses_aligned_visible_pc() {
        let mut cpu = cpu_with_pc(2);
        cpu.set_thumb(true);
        let mut bus = FakeBus::new(128);
        bus.write32(4, 0x1122_3344);

        exec(&mut cpu, &mut bus, 0x4800); // LDR r0, [PC, #0]

        assert_eq!(cpu.read_reg(0), 0x1122_3344);
    }

    #[test]
    fn register_offset_signed_byte_load_sign_extends() {
        let mut cpu = cpu_with_pc(0);
        let mut bus = FakeBus::new(128);
        cpu.write_reg(1, 0x40);
        cpu.write_reg(2, 1);
        bus.write_8(0x41, 0xf0);

        exec(&mut cpu, &mut bus, 0x5688); // LDSB r0, [r1, r2]

        assert_eq!(cpu.read_reg(0), 0xffff_fff0);
    }

    #[test]
    fn immediate_word_store_and_load_work() {
        let mut cpu = cpu_with_pc(0);
        let mut bus = FakeBus::new(256);
        cpu.write_reg(1, 0x40);
        cpu.write_reg(0, 0x1234_5678);

        exec(&mut cpu, &mut bus, 0x6008); // STR r0, [r1, #0]
        cpu.set_pc(2);
        exec(&mut cpu, &mut bus, 0x680a); // LDR r2, [r1, #0]

        assert_eq!(cpu.read_reg(2), 0x1234_5678);
    }

    #[test]
    fn load_address_from_pc_uses_word_aligned_base() {
        let mut cpu = cpu_with_pc(2);
        let mut bus = FakeBus::new(64);

        exec(&mut cpu, &mut bus, 0xa000); // ADD r0, PC, #0

        assert_eq!(cpu.read_reg(0), 4);
    }

    #[test]
    fn sp_relative_store_and_load_work() {
        let mut cpu = cpu_with_pc(0);
        let mut bus = FakeBus::new(256);
        cpu.set_sp(0x80);
        cpu.write_reg(0, 0xaabb_ccdd);

        exec(&mut cpu, &mut bus, 0x9000); // STR r0, [SP, #0]
        cpu.set_pc(2);
        exec(&mut cpu, &mut bus, 0x9900); // LDR r1, [SP, #0]

        assert_eq!(bus.read32(0x80), 0xaabb_ccdd);
        assert_eq!(cpu.read_reg(1), 0xaabb_ccdd);
    }
}
