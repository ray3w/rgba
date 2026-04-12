use rgba_arm7tdmi::BusInterface;
use rgba_core::{Cartridge, EventKind, Gba};

const DISPCNT_ADDR: u32 = 0x0400_0000;
const DISPSTAT_ADDR: u32 = 0x0400_0004;
const IE_ADDR: u32 = 0x0400_0200;

fn push_word(rom: &mut Vec<u8>, value: u32) {
    rom.extend_from_slice(&value.to_le_bytes());
}

#[test]
fn cpu_reads_and_writes_mmio_through_core_bus() {
    let mut rom = Vec::new();
    push_word(&mut rom, 0xe580_1000); // STR r1, [r0]
    push_word(&mut rom, 0xe590_2000); // LDR r2, [r0]

    let mut gba = Gba::new(Cartridge::new(rom));
    gba.cpu_mut().set_pc(0x0800_0000);
    gba.cpu_mut().write_reg(0, IE_ADDR);
    gba.cpu_mut().write_reg(1, 0x00f3);

    gba.step();
    gba.step();

    assert_eq!(gba.bus().io().ie(), 0x00f3);
    assert_eq!(gba.cpu().read_reg(2), 0x00f3);
}

#[test]
fn scheduled_events_are_consumed_after_step_advances_time() {
    let mut rom = Vec::new();
    push_word(&mut rom, 0xe3a0_0000); // MOV r0, #0

    let mut gba = Gba::new(Cartridge::new(rom));
    gba.cpu_mut().set_pc(0x0800_0000);
    gba.bus_mut().write_16(DISPSTAT_ADDR, 1 << 3); // enable VBlank IRQ
    gba.schedule_event_in(1, EventKind::VBlank);

    let cycles = gba.step();

    assert_eq!(cycles, 1);
    assert_eq!(gba.scheduler().timestamp(), 1);
    assert_ne!(gba.bus().io().dispstat() & 0x0001, 0);
    assert_ne!(gba.bus().io().if_() & 0x0001, 0);
}

#[test]
fn hblank_event_updates_mmio_state() {
    let mut rom = Vec::new();
    push_word(&mut rom, 0xe3a0_0000); // MOV r0, #0

    let mut gba = Gba::new(Cartridge::new(rom));
    gba.cpu_mut().set_pc(0x0800_0000);
    gba.bus_mut().write_16(DISPCNT_ADDR, 3);
    gba.schedule_event_in(1, EventKind::HBlank);

    gba.step();

    assert_ne!(gba.bus().io().dispstat() & 0x0002, 0);
}
