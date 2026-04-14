use rgba_arm7tdmi::BusInterface;
use rgba_core::arm7tdmi::Mode;
use rgba_core::{BiosBackend, Cartridge, Gba};

const DISPCNT_ADDR: u32 = 0x0400_0000;
const DISPSTAT_ADDR: u32 = 0x0400_0004;
const PALETTE_ADDR: u32 = 0x0500_0000;
const VRAM_ADDR: u32 = 0x0600_0000;
const OAM_ADDR: u32 = 0x0700_0000;
const IE_ADDR: u32 = 0x0400_0200;
const IME_ADDR: u32 = 0x0400_0208;
const REG_IFBIOS_ADDR: u32 = 0x03ff_fff8;
const ROM_ENTRY: u32 = 0x0800_0000;
const IRQ_VBLANK: u16 = 1 << 0;

fn arm_rom(words: &[u32]) -> Vec<u8> {
    let mut rom = Vec::with_capacity(words.len() * 4);
    for word in words {
        rom.extend_from_slice(&word.to_le_bytes());
    }
    rom
}

#[test]
fn swi_div_hle_executes_and_returns_to_arm_caller() {
    let rom = arm_rom(&[
        0xef00_0006, // SWI 0x06
        0xeaff_fffe, // idle loop
    ]);
    let mut gba = Gba::new(Cartridge::new(rom));
    gba.cpu_mut().set_pc(ROM_ENTRY);
    gba.cpu_mut().write_reg(0, 27);
    gba.cpu_mut().write_reg(1, 5);

    gba.step();
    assert_eq!(gba.cpu().mode(), Mode::Supervisor);
    assert_eq!(gba.cpu().pc(), 0x0000_0008);

    gba.step();

    assert_eq!(gba.bios().backend(), BiosBackend::Hle);
    assert_eq!(gba.cpu().mode(), Mode::User);
    assert_eq!(gba.cpu().pc(), ROM_ENTRY + 4);
    assert_eq!(gba.cpu().read_reg(0), 5);
    assert_eq!(gba.cpu().read_reg(1), 2);
    assert_eq!(gba.cpu().read_reg(3), 5);
}

#[test]
fn thumb_register_ram_reset_hle_returns_to_thumb_and_clears_memory() {
    let mut rom = vec![0; 4];
    rom[0..2].copy_from_slice(&0xdf01u16.to_le_bytes()); // SWI 0x01
    rom[2..4].copy_from_slice(&0xe7feu16.to_le_bytes()); // idle loop
    let mut gba = Gba::new(Cartridge::new(rom));
    gba.cpu_mut().set_pc(ROM_ENTRY);
    gba.cpu_mut().set_thumb(true);
    gba.cpu_mut().write_reg(0, 0xff);

    gba.bus_mut().write_32(0x0200_0000, 0x1234_5678);
    gba.bus_mut().write_32(0x0300_0000, 0x89ab_cdef);
    gba.bus_mut().write_16(DISPCNT_ADDR, 0x0403);
    gba.bus_mut().write_16(PALETTE_ADDR, 0x001f);
    gba.bus_mut().write_16(VRAM_ADDR, 0x03e0);
    gba.bus_mut().write_16(OAM_ADDR, 0x7c00);

    gba.step();
    assert_eq!(gba.cpu().mode(), Mode::Supervisor);
    assert_eq!(gba.cpu().pc(), 0x0000_0008);

    gba.step();

    assert_eq!(gba.cpu().mode(), Mode::User);
    assert!(gba.cpu().is_thumb());
    assert_eq!(gba.cpu().pc(), ROM_ENTRY + 2);
    assert_eq!(gba.bus().read_32_debug(0x0200_0000), 0);
    assert_eq!(gba.bus().read_32_debug(0x0300_0000), 0);
    assert_eq!(gba.bus().read_32_debug(DISPCNT_ADDR), 0);
    assert_eq!(gba.bus().read_32_debug(PALETTE_ADDR), 0);
    assert_eq!(gba.bus().read_32_debug(VRAM_ADDR), 0);
}

#[test]
fn loading_external_bios_switches_bios_backend() {
    let mut gba = Gba::new(Cartridge::new(vec![0; 4]));
    assert_eq!(gba.bios().backend(), BiosBackend::Hle);

    gba.load_bios(&[0; 16]).unwrap();

    assert_eq!(gba.bios().backend(), BiosBackend::External);
}

#[test]
fn vblank_intr_wait_blocks_until_irq_and_returns_to_thumb_caller() {
    let mut rom = vec![0; 6];
    rom[0..2].copy_from_slice(&0xdf05u16.to_le_bytes()); // SWI 0x05
    rom[2..4].copy_from_slice(&0x1c08u16.to_le_bytes()); // ADDS r0, r1, #0
    rom[4..6].copy_from_slice(&0xe7feu16.to_le_bytes()); // B .
    let mut gba = Gba::new(Cartridge::new(rom));
    gba.cpu_mut().set_pc(ROM_ENTRY);
    gba.cpu_mut().set_thumb(true);
    gba.cpu_mut().write_reg(1, 7);
    gba.bus_mut().write_16(DISPSTAT_ADDR, 1 << 3);
    gba.bus_mut().write_16(IE_ADDR, IRQ_VBLANK);
    gba.bus_mut().write_16(IME_ADDR, 1);

    gba.step();
    assert_eq!(gba.cpu().mode(), Mode::Supervisor);
    assert_eq!(gba.cpu().pc(), 0x0000_0008);

    gba.step();
    assert!(gba.cpu().is_thumb());
    assert_eq!(gba.cpu().pc(), ROM_ENTRY + 2);

    gba.step();
    assert_eq!(gba.cpu().pc(), ROM_ENTRY + 2);

    gba.bus_mut().io_mut().request_interrupt(IRQ_VBLANK);
    gba.step();
    assert_eq!(gba.cpu().mode(), Mode::Irq);
    assert_eq!(gba.cpu().pc(), 0x0000_0018);

    gba.step();
    assert_eq!(gba.cpu().mode(), Mode::User);
    assert!(gba.cpu().is_thumb());
    assert_eq!(gba.cpu().pc(), ROM_ENTRY + 2);
    assert_eq!(gba.bus_mut().read_16(REG_IFBIOS_ADDR), IRQ_VBLANK);

    gba.step();
    assert_eq!(gba.cpu().read_reg(0), 7);
}

#[test]
fn intr_wait_with_flag_clear_ignores_stale_ifbios_bits() {
    let rom = arm_rom(&[
        0xef00_0004, // SWI 0x04
        0xeaff_fffe, // idle loop
    ]);
    let mut gba = Gba::new(Cartridge::new(rom));
    gba.cpu_mut().set_pc(ROM_ENTRY);
    gba.cpu_mut().write_reg(0, 1);
    gba.cpu_mut().write_reg(1, IRQ_VBLANK.into());
    gba.bus_mut().write_16(IE_ADDR, IRQ_VBLANK);
    gba.bus_mut().write_16(IME_ADDR, 1);
    gba.bus_mut().write_16(REG_IFBIOS_ADDR, IRQ_VBLANK);

    gba.step();
    assert_eq!(gba.cpu().mode(), Mode::Supervisor);
    assert_eq!(gba.cpu().pc(), 0x0000_0008);

    gba.step();
    assert_eq!(gba.bus_mut().read_16(REG_IFBIOS_ADDR), 0);
    assert_eq!(gba.cpu().pc(), ROM_ENTRY + 4);

    gba.step();
    assert_eq!(gba.cpu().pc(), ROM_ENTRY + 4);

    gba.bus_mut().io_mut().request_interrupt(IRQ_VBLANK);
    gba.step();
    assert_eq!(gba.cpu().mode(), Mode::Irq);

    gba.step();
    assert_eq!(gba.cpu().mode(), Mode::User);
    assert_eq!(gba.bus_mut().read_16(REG_IFBIOS_ADDR), IRQ_VBLANK);
}
