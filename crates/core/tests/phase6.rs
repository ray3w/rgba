use rgba_arm7tdmi::BusInterface;
use rgba_core::arm7tdmi::{Mode, SP};
use rgba_core::{Cartridge, Gba};

const BG0CNT_ADDR: u32 = 0x0400_0008;
const BG1CNT_ADDR: u32 = 0x0400_000a;
const DISPCNT_ADDR: u32 = 0x0400_0000;
const WIN0H_ADDR: u32 = 0x0400_0040;
const WIN0V_ADDR: u32 = 0x0400_0044;
const WININ_ADDR: u32 = 0x0400_0048;
const WINOUT_ADDR: u32 = 0x0400_004a;
const BLDCNT_ADDR: u32 = 0x0400_0050;
const BLDALPHA_ADDR: u32 = 0x0400_0052;
const PALETTE_ADDR: u32 = 0x0500_0000;
const VRAM_ADDR: u32 = 0x0600_0000;
const OAM_ADDR: u32 = 0x0700_0000;

const MODE4_BG2: u16 = 0x0404;
const MODE0_BG0: u16 = 0x0100;
const MODE0_BG0_BG1_WIN0: u16 = 0x2300;
const MODE0_OBJ_1D: u16 = 0x1040;
const MODE0_BG0_OBJ_1D: u16 = 0x1140;
const MODE0_BG0_OBJ_1D_OBJWIN: u16 = 0x9140;
const MODE5_BG2: u16 = 0x0405;
const BG0CNT_SCREEN1_CHAR1: u16 = 0x0104;
const BG0CNT_SCREEN1_CHAR0: u16 = 0x0100;
const BG1CNT_SCREEN2_CHAR1: u16 = 0x0204;

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

fn step_to_hblank(gba: &mut Gba) {
    for _ in 0..960 {
        gba.step();
    }
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

    step_to_hblank(&mut gba);

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

    step_to_hblank(&mut gba);

    assert_eq!(gba.ppu().framebuffer()[0], 0x001f);
    assert_eq!(gba.ppu().framebuffer()[1], 0x03e0);
    assert_eq!(gba.ppu().framebuffer()[2], 0x001f);
    assert_eq!(gba.ppu().framebuffer()[3], 0x03e0);
}

#[test]
fn gba_step_renders_mode0_obj_scanline_from_oam_and_vram() {
    let mut gba = Gba::new(Cartridge::new(idle_rom(1024)));
    gba.cpu_mut().set_pc(0x0800_0000);
    gba.bus_mut().write_16(DISPCNT_ADDR, MODE0_OBJ_1D);
    gba.bus_mut().write_16(PALETTE_ADDR + 0x0200 + 2, 0x001f);
    gba.bus_mut().write_16(PALETTE_ADDR + 0x0200 + 4, 0x03e0);

    gba.bus_mut().write_16(OAM_ADDR, 0x0000);
    gba.bus_mut().write_16(OAM_ADDR + 2, 0x0000);
    gba.bus_mut().write_16(OAM_ADDR + 4, 0x0000);

    gba.bus_mut().write_8(VRAM_ADDR + 0x1_0000, 0x21);
    gba.bus_mut().write_8(VRAM_ADDR + 0x1_0000 + 1, 0x21);
    gba.bus_mut().write_8(VRAM_ADDR + 0x1_0000 + 2, 0x21);
    gba.bus_mut().write_8(VRAM_ADDR + 0x1_0000 + 3, 0x21);

    step_to_hblank(&mut gba);

    assert_eq!(gba.ppu().framebuffer()[0], 0x001f);
    assert_eq!(gba.ppu().framebuffer()[1], 0x03e0);
}

#[test]
fn gba_step_renders_mode5_scanline_from_bitmap() {
    let mut gba = Gba::new(Cartridge::new(idle_rom(1024)));
    gba.cpu_mut().set_pc(0x0800_0000);
    gba.bus_mut().write_16(DISPCNT_ADDR, MODE5_BG2);
    gba.bus_mut().write_16(VRAM_ADDR, 0x001f);
    gba.bus_mut().write_16(VRAM_ADDR + 159 * 2, 0x03e0);

    step_to_hblank(&mut gba);

    assert_eq!(gba.ppu().framebuffer()[0], 0x001f);
    assert_eq!(gba.ppu().framebuffer()[159], 0x03e0);
    assert_eq!(gba.ppu().framebuffer()[160], 0x0000);
}

#[test]
fn gba_step_window_masks_background_layers_before_compose() {
    let mut gba = Gba::new(Cartridge::new(idle_rom(1024)));
    gba.cpu_mut().set_pc(0x0800_0000);
    gba.bus_mut().write_16(DISPCNT_ADDR, MODE0_BG0_BG1_WIN0);
    gba.bus_mut().write_16(BG0CNT_ADDR, BG0CNT_SCREEN1_CHAR0);
    gba.bus_mut().write_16(BG1CNT_ADDR, BG1CNT_SCREEN2_CHAR1);
    gba.bus_mut().write_16(WIN0H_ADDR, 0x0008);
    gba.bus_mut().write_16(WIN0V_ADDR, 0x0001);
    gba.bus_mut().write_16(WININ_ADDR, 0x0002);
    gba.bus_mut().write_16(WINOUT_ADDR, 0x0001);

    gba.bus_mut().write_16(PALETTE_ADDR, 0x0000);
    gba.bus_mut().write_16(PALETTE_ADDR + 2, 0x001f);
    gba.bus_mut().write_16(PALETTE_ADDR + 4, 0x03e0);

    gba.bus_mut().write_8(VRAM_ADDR + 0x0000, 0x11);
    gba.bus_mut().write_8(VRAM_ADDR + 0x0001, 0x11);
    gba.bus_mut().write_8(VRAM_ADDR + 0x0002, 0x11);
    gba.bus_mut().write_8(VRAM_ADDR + 0x0003, 0x11);
    gba.bus_mut().write_16(VRAM_ADDR + 0x0800, 0x0000);

    gba.bus_mut().write_8(VRAM_ADDR + 0x4000, 0x22);
    gba.bus_mut().write_8(VRAM_ADDR + 0x4001, 0x22);
    gba.bus_mut().write_8(VRAM_ADDR + 0x4002, 0x22);
    gba.bus_mut().write_8(VRAM_ADDR + 0x4003, 0x22);
    gba.bus_mut().write_16(VRAM_ADDR + 0x1000, 0x0000);

    step_to_hblank(&mut gba);

    assert_eq!(gba.ppu().framebuffer()[0], 0x03e0);
    assert_eq!(gba.ppu().framebuffer()[8], 0x001f);
}

#[test]
fn gba_step_blends_semi_transparent_obj_with_background() {
    let mut gba = Gba::new(Cartridge::new(idle_rom(1024)));
    gba.cpu_mut().set_pc(0x0800_0000);
    gba.bus_mut().write_16(DISPCNT_ADDR, MODE0_BG0_OBJ_1D);
    gba.bus_mut().write_16(BG0CNT_ADDR, BG0CNT_SCREEN1_CHAR1);
    gba.bus_mut().write_16(BLDCNT_ADDR, 0x0150);
    gba.bus_mut().write_16(BLDALPHA_ADDR, 0x0808);

    gba.bus_mut().write_16(PALETTE_ADDR, 0x0000);
    gba.bus_mut().write_16(PALETTE_ADDR + 2, 0x001f);
    gba.bus_mut().write_16(PALETTE_ADDR + 0x0200 + 2, 0x03e0);

    gba.bus_mut().write_8(VRAM_ADDR + 0x4000, 0x11);
    gba.bus_mut().write_8(VRAM_ADDR + 0x4001, 0x11);
    gba.bus_mut().write_8(VRAM_ADDR + 0x4002, 0x11);
    gba.bus_mut().write_8(VRAM_ADDR + 0x4003, 0x11);
    gba.bus_mut().write_16(VRAM_ADDR + 0x0800, 0x0000);

    gba.bus_mut().write_16(OAM_ADDR, 0x0400);
    gba.bus_mut().write_16(OAM_ADDR + 2, 0x0000);
    gba.bus_mut().write_16(OAM_ADDR + 4, 0x0000);
    gba.bus_mut().write_8(VRAM_ADDR + 0x1_0000, 0x11);

    step_to_hblank(&mut gba);

    assert_eq!(gba.ppu().framebuffer()[0], 0x01ef);
}

#[test]
fn gba_step_obj_window_masks_obj_layer() {
    let mut gba = Gba::new(Cartridge::new(idle_rom(1024)));
    gba.cpu_mut().set_pc(0x0800_0000);
    gba.bus_mut()
        .write_16(DISPCNT_ADDR, MODE0_BG0_OBJ_1D_OBJWIN);
    gba.bus_mut().write_16(BG0CNT_ADDR, BG0CNT_SCREEN1_CHAR1);
    gba.bus_mut().write_16(WINOUT_ADDR, 0x0111);

    gba.bus_mut().write_16(PALETTE_ADDR, 0x0000);
    gba.bus_mut().write_16(PALETTE_ADDR + 2, 0x001f);
    gba.bus_mut().write_16(PALETTE_ADDR + 0x0200 + 2, 0x03e0);

    gba.bus_mut().write_8(VRAM_ADDR + 0x4000, 0x11);
    gba.bus_mut().write_8(VRAM_ADDR + 0x4001, 0x11);
    gba.bus_mut().write_8(VRAM_ADDR + 0x4002, 0x11);
    gba.bus_mut().write_8(VRAM_ADDR + 0x4003, 0x11);
    gba.bus_mut().write_16(VRAM_ADDR + 0x0800, 0x0000);

    gba.bus_mut().write_16(OAM_ADDR, 0x0000);
    gba.bus_mut().write_16(OAM_ADDR + 2, 0x0000);
    gba.bus_mut().write_16(OAM_ADDR + 4, 0x0000);
    gba.bus_mut().write_16(OAM_ADDR + 8, 0x0800);
    gba.bus_mut().write_16(OAM_ADDR + 10, 0x0000);
    gba.bus_mut().write_16(OAM_ADDR + 12, 0x0000);
    gba.bus_mut().write_8(VRAM_ADDR + 0x1_0000, 0x11);

    step_to_hblank(&mut gba);

    assert_eq!(gba.ppu().framebuffer()[0], 0x001f);
}
