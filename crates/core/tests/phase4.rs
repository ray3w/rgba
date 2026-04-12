use rgba_arm7tdmi::BusInterface;
use rgba_core::{Cartridge, Gba, SCREEN_WIDTH};

const DISPCNT_ADDR: u32 = 0x0400_0000;
const VCOUNT_ADDR: u32 = 0x0400_0006;
const VRAM_ADDR: u32 = 0x0600_0000;
const MODE3_BG2: u16 = 0x0403;

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
fn gba_step_renders_mode3_scanline_into_framebuffer() {
    let mut gba = Gba::new(Cartridge::new(idle_rom(1024)));
    gba.cpu_mut().set_pc(0x0800_0000);
    gba.bus_mut().write_16(DISPCNT_ADDR, MODE3_BG2);
    gba.bus_mut().write_16(VRAM_ADDR, 0x001f);
    gba.bus_mut().write_16(VRAM_ADDR + 2, 0x03e0);

    for _ in 0..960 {
        gba.step();
    }

    assert_eq!(gba.ppu().framebuffer()[0], 0x001f);
    assert_eq!(gba.ppu().framebuffer()[1], 0x03e0);
}

#[test]
fn gba_step_advances_vcount_through_ppu() {
    let mut gba = Gba::new(Cartridge::new(idle_rom(1400)));
    gba.cpu_mut().set_pc(0x0800_0000);
    gba.bus_mut().write_16(DISPCNT_ADDR, MODE3_BG2);

    for _ in 0..1232 {
        gba.step();
    }

    assert_eq!(gba.bus().io().read_16(VCOUNT_ADDR), 1);
}

#[test]
fn ppu_marks_frame_ready_after_visible_frame() {
    let mut gba = Gba::new(Cartridge::new(idle_rom(160 * 1232 + 16)));
    gba.cpu_mut().set_pc(0x0800_0000);
    gba.bus_mut().write_16(DISPCNT_ADDR, MODE3_BG2);

    for _ in 0..(160 * 1232) {
        gba.step();
    }

    assert!(gba.ppu().frame_ready());
    assert!(gba.ppu_mut().take_frame_ready());
    assert!(!gba.ppu_mut().take_frame_ready());
    assert_eq!(gba.ppu().framebuffer().len(), SCREEN_WIDTH * 160);
}
