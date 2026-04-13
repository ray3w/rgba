use rgba_arm7tdmi::BusInterface;
use rgba_core::arm7tdmi::{Mode, SP};
use rgba_core::{Cartridge, Gba};

const BG0CNT_ADDR: u32 = 0x0400_0008;
const DISPCNT_ADDR: u32 = 0x0400_0000;
const PALETTE_ADDR: u32 = 0x0500_0000;
const VRAM_ADDR: u32 = 0x0600_0000;

const MODE4_BG2: u16 = 0x0404;
const MODE0_BG0: u16 = 0x0100;
const BG0CNT_SCREEN1_CHAR1: u16 = 0x0104;

fn push_word(rom: &mut Vec<u8>, value: u32) {
    rom.extend_from_slice(&value.to_le_bytes());
}

fn idle_rom(words: usize) -> Vec<u8> {
    let mut rom = Vec::with_capacity(words * 4);
    for _ in 0..words {
        push_word(&mut rom, 0xe3a0_0000); // MOV r0, #0
    }
    rom
}

#[test]
fn gba_initializes_post_bios_stack_pointers() {
    let gba = Gba::new(Cartridge::new(idle_rom(4)));

    assert_eq!(gba.cpu().read_reg_for_mode(Mode::User, SP), 0x0300_7f00);
    assert_eq!(gba.cpu().read_reg_for_mode(Mode::Irq, SP), 0x0300_7fa0);
    assert_eq!(
        gba.cpu().read_reg_for_mode(Mode::Supervisor, SP),
        0x0300_7fe0
    );
}

#[test]
fn gba_step_renders_mode4_scanline_from_palette_bitmap() {
    let mut gba = Gba::new(Cartridge::new(idle_rom(1024)));
    gba.cpu_mut().set_pc(0x0800_0000);
    gba.bus_mut().write_16(DISPCNT_ADDR, MODE4_BG2);
    gba.bus_mut().write_16(PALETTE_ADDR + 2, 0x001f);
    gba.bus_mut().write_16(PALETTE_ADDR + 4, 0x03e0);
    gba.bus_mut().write_8(VRAM_ADDR, 1);
    gba.bus_mut().write_8(VRAM_ADDR + 1, 2);

    for _ in 0..960 {
        gba.step();
    }

    assert_eq!(gba.ppu().framebuffer()[0], 0x001f);
    assert_eq!(gba.ppu().framebuffer()[1], 0x03e0);
}

#[test]
fn gba_step_renders_mode0_bg0_scanline_from_text_tiles() {
    let mut gba = Gba::new(Cartridge::new(idle_rom(1024)));
    gba.cpu_mut().set_pc(0x0800_0000);
    gba.bus_mut().write_16(DISPCNT_ADDR, MODE0_BG0);
    gba.bus_mut().write_16(BG0CNT_ADDR, BG0CNT_SCREEN1_CHAR1);
    gba.bus_mut().write_16(PALETTE_ADDR, 0x0000);
    gba.bus_mut().write_16(PALETTE_ADDR + 2, 0x001f);
    gba.bus_mut().write_16(PALETTE_ADDR + 4, 0x03e0);

    // Tile 0 row 0: 1,2,1,2,1,2,1,2 in 4bpp.
    gba.bus_mut().write_8(VRAM_ADDR + 0x4000, 0x21);
    gba.bus_mut().write_8(VRAM_ADDR + 0x4001, 0x21);
    gba.bus_mut().write_8(VRAM_ADDR + 0x4002, 0x21);
    gba.bus_mut().write_8(VRAM_ADDR + 0x4003, 0x21);
    // Screen block 1 entry 0 -> tile 0.
    gba.bus_mut().write_16(VRAM_ADDR + 0x0800, 0x0000);

    for _ in 0..960 {
        gba.step();
    }

    assert_eq!(gba.ppu().framebuffer()[0], 0x001f);
    assert_eq!(gba.ppu().framebuffer()[1], 0x03e0);
    assert_eq!(gba.ppu().framebuffer()[2], 0x001f);
    assert_eq!(gba.ppu().framebuffer()[3], 0x03e0);
}
