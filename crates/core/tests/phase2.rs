use rgba_arm7tdmi::BusInterface;
use rgba_core::{Cartridge, Gba};

fn push_word(rom: &mut Vec<u8>, value: u32) {
    rom.extend_from_slice(&value.to_le_bytes());
}

#[test]
fn cpu_fetches_instructions_through_core_bus() {
    let mut rom = Vec::new();
    push_word(&mut rom, 0xe580_1000); // STR r1, [r0]
    push_word(&mut rom, 0xe590_2000); // LDR r2, [r0]

    let mut gba = Gba::new(Cartridge::new(rom));
    gba.cpu_mut().set_pc(0x0800_0000);
    gba.cpu_mut().write_reg(0, 0x0200_0000);
    gba.cpu_mut().write_reg(1, 0x1234_5678);

    gba.step();
    gba.step();

    assert_eq!(gba.cpu().read_reg(2), 0x1234_5678);
    assert_eq!(gba.bus_mut().read_32(0x0200_0000), 0x1234_5678);
}

#[test]
fn bus_exposes_gamepak_mirrors_to_the_cpu() {
    let mut rom = Vec::new();
    push_word(&mut rom, 0xe3a0_0000); // MOV r0, #0

    let mut gba = Gba::with_rom(rom);
    gba.cpu_mut().set_pc(0x0a00_0000);

    gba.step();

    assert_eq!(gba.cpu().read_reg(0), 0);
    assert_eq!(gba.cpu().pc(), 0x0a00_0004);
}
