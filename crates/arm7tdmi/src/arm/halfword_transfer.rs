use crate::{Arm7tdmi, BusInterface, PC};

use super::{read_exec_reg, write_exec_reg, ExecutionResult};

pub(super) fn is_halfword_transfer(opcode: u32) -> bool {
    (opcode & 0x0e00_0090) == 0x0000_0090 && ((opcode >> 5) & 0x3) != 0
}

pub(super) fn execute<B: BusInterface>(
    cpu: &mut Arm7tdmi,
    bus: &mut B,
    opcode: u32,
    fetch_pc: u32,
) -> ExecutionResult {
    let pre_index = ((opcode >> 24) & 1) != 0;
    let add_offset = ((opcode >> 23) & 1) != 0;
    let immediate_offset = ((opcode >> 22) & 1) != 0;
    let write_back = ((opcode >> 21) & 1) != 0;
    let load = ((opcode >> 20) & 1) != 0;
    let rn = ((opcode >> 16) & 0xf) as u8;
    let rd = ((opcode >> 12) & 0xf) as u8;
    let signed = ((opcode >> 6) & 1) != 0;
    let halfword = ((opcode >> 5) & 1) != 0;

    let base = read_exec_reg(cpu, rn, fetch_pc);
    let offset = if immediate_offset {
        (((opcode >> 8) & 0xf) << 4) | (opcode & 0xf)
    } else {
        read_exec_reg(cpu, (opcode & 0xf) as u8, fetch_pc)
    };
    let indexed = if add_offset {
        base.wrapping_add(offset)
    } else {
        base.wrapping_sub(offset)
    };
    let address = if pre_index { indexed } else { base };

    let mut wrote_pc = false;

    if load {
        let value = match (signed, halfword) {
            (false, true) => load_halfword(bus, address),
            (true, false) => sign_extend8(bus.read_8(address)),
            (true, true) => {
                if (address & 1) == 0 {
                    sign_extend16(bus.read_16(address & !1))
                } else {
                    sign_extend8(bus.read_8(address))
                }
            }
            (false, false) => panic!("unimplemented ARM halfword-transfer opcode: 0x{opcode:08x}"),
        };
        wrote_pc = write_exec_reg(cpu, rd, value);
    } else {
        let value = if rd == PC {
            fetch_pc.wrapping_add(12)
        } else {
            read_exec_reg(cpu, rd, fetch_pc)
        };
        store_halfword(bus, address, value as u16);
    }

    if (!pre_index || write_back) && !(load && rn == rd) {
        cpu.write_reg(rn, indexed);
    }

    ExecutionResult {
        cycles: if load { 3 } else { 2 },
        wrote_pc,
    }
}

fn load_halfword<B: BusInterface>(bus: &mut B, address: u32) -> u32 {
    let value = u32::from(bus.read_16(address & !1));
    if (address & 1) == 0 {
        value
    } else {
        value.rotate_right(8)
    }
}

fn store_halfword<B: BusInterface>(bus: &mut B, address: u32, value: u16) {
    bus.write_16(address & !1, value);
}

fn sign_extend8(value: u8) -> u32 {
    i32::from(value as i8) as u32
}

fn sign_extend16(value: u16) -> u32 {
    i32::from(value as i16) as u32
}

#[cfg(test)]
mod tests {
    use crate::arm::test_utils::{cpu_with_pc, FakeBus};
    use crate::BusInterface;

    fn exec(cpu: &mut crate::Arm7tdmi, bus: &mut FakeBus, opcode: u32) {
        bus.load32(cpu.pc(), opcode);
        cpu.step(bus);
    }

    #[test]
    fn strh_stores_the_low_halfword() {
        let mut cpu = cpu_with_pc(0);
        let mut bus = FakeBus::new(256);
        cpu.write_reg(0, 0x40);
        cpu.write_reg(1, 0xffff_1234);

        exec(&mut cpu, &mut bus, 0xe1c0_10b0);

        assert_eq!(bus.read32(0x40), 0x0000_1234);
    }

    #[test]
    fn ldrh_rotates_misaligned_loads() {
        let mut cpu = cpu_with_pc(0);
        let mut bus = FakeBus::new(256);
        cpu.write_reg(0, 0x40);
        bus.write_16(0x40, 0x0020);

        exec(&mut cpu, &mut bus, 0xe1d0_10b1);

        assert_eq!(cpu.read_reg(1), 0x2000_0000);
    }

    #[test]
    fn ldrsh_from_odd_address_sign_extends_the_byte() {
        let mut cpu = cpu_with_pc(0);
        let mut bus = FakeBus::new(256);
        cpu.write_reg(0, 0x40);
        bus.write_16(0x40, 0xff00);

        exec(&mut cpu, &mut bus, 0xe1d0_10f1);

        assert_eq!(cpu.read_reg(1), 0xffff_ffff);
    }

    #[test]
    fn load_with_writeback_same_register_keeps_loaded_value() {
        let mut cpu = cpu_with_pc(0);
        let mut bus = FakeBus::new(256);
        cpu.write_reg(0, 0x40);
        bus.write32(0x44, 32);

        exec(&mut cpu, &mut bus, 0xe1f0_00b4);

        assert_eq!(cpu.read_reg(0), 32);
    }

    #[test]
    fn store_with_writeback_same_register_updates_base_after_storing_old_value() {
        let mut cpu = cpu_with_pc(0);
        let mut bus = FakeBus::new(256);
        cpu.write_reg(0, 0x40);

        exec(&mut cpu, &mut bus, 0xe1e0_00b4);

        assert_eq!(cpu.read_reg(0), 0x44);
        assert_eq!(bus.read32(0x44), 0x40);
    }
}
