//! Phase 5 keypad visual integration test.
//!
//! Paints a button dashboard into Mode 3 VRAM, simulates button presses,
//! verifies KEYINPUT register state, renders frames through the PPU, and
//! saves PNG screenshots to `test_output/`.

use std::fs::{self, File};
use std::io::BufWriter;
use std::path::PathBuf;

use rgba_core::arm7tdmi::BusInterface;
use rgba_core::{Button, Gba, FRAME_PIXELS, SCREEN_HEIGHT, SCREEN_WIDTH};

const DISPCNT_ADDR: u32 = 0x0400_0000;
const KEYINPUT_ADDR: u32 = 0x0400_0130;
const VRAM_ADDR: u32 = 0x0600_0000;
const MODE3_BG2: u16 = 0x0403;

// --- Layout constants ---

const BLOCK_W: usize = 40;
const BLOCK_H: usize = 50;
const GAP_X: usize = 8;
const GAP_Y: usize = 20;
const MARGIN_X: usize = 8;
const MARGIN_Y: usize = 20;
const BORDER: usize = 2;

const BG_COLOR: u16 = 0x1084; // very dark grey
const DARK_COLOR: u16 = 0x294A; // dark grey for unpressed buttons

// RGB555 bright colors for each pressed button.
const BUTTON_COLORS: [u16; 10] = [
    0x001F, // A     — red
    0x03E0, // B     — green
    0x7C00, // Sel   — blue
    0x7FFF, // Start — white
    0x03FF, // Right — yellow
    0x7C1F, // Left  — magenta
    0x7FE0, // Up    — cyan
    0x2DEB, // Down  — orange
    0x56B5, // R     — light grey
    0x4E73, // L     — steel
];

const BUTTON_ORDER: [Button; 10] = [
    Button::A,
    Button::B,
    Button::Select,
    Button::Start,
    Button::Right,
    Button::Left,
    Button::Up,
    Button::Down,
    Button::R,
    Button::L,
];

// --- Helpers ---

fn block_origin(col: usize, row: usize) -> (usize, usize) {
    (
        MARGIN_X + col * (BLOCK_W + GAP_X),
        MARGIN_Y + row * (BLOCK_H + GAP_Y),
    )
}

fn idle_rom(words: usize) -> Vec<u8> {
    let mut rom = Vec::with_capacity(words * 4);
    for _ in 0..words {
        rom.extend_from_slice(&0xe3a0_0000u32.to_le_bytes());
    }
    rom
}

fn write_vram_pixel(gba: &mut Gba, x: usize, y: usize, color: u16) {
    let addr = VRAM_ADDR + ((y * SCREEN_WIDTH + x) as u32 * 2);
    gba.bus_mut().write_16(addr, color);
}

/// Paint the entire button dashboard into VRAM.
///
/// `pressed_mask` uses the same bit layout as the inverted KEYINPUT register:
/// bit N = 1 means button N is pressed.
fn paint_dashboard(gba: &mut Gba, pressed_mask: u16) {
    // Fill background
    for y in 0..SCREEN_HEIGHT {
        for x in 0..SCREEN_WIDTH {
            write_vram_pixel(gba, x, y, BG_COLOR);
        }
    }

    // Draw each button as a bordered rectangle
    for (i, &button) in BUTTON_ORDER.iter().enumerate() {
        let col = i % 5;
        let row = i / 5;
        let (bx, by) = block_origin(col, row);
        let is_pressed = (pressed_mask & button.mask()) != 0;
        let fill = if is_pressed {
            BUTTON_COLORS[i]
        } else {
            DARK_COLOR
        };
        let border_color = if is_pressed { 0x7FFF } else { 0x3DEF };

        for dy in 0..BLOCK_H {
            for dx in 0..BLOCK_W {
                let px = bx + dx;
                let py = by + dy;
                if px >= SCREEN_WIDTH || py >= SCREEN_HEIGHT {
                    continue;
                }
                let on_border = dx < BORDER
                    || dx >= BLOCK_W - BORDER
                    || dy < BORDER
                    || dy >= BLOCK_H - BORDER;
                let color = if on_border { border_color } else { fill };
                write_vram_pixel(gba, px, py, color);
            }
        }
    }
}

fn run_one_frame(gba: &mut Gba) {
    for _ in 0..300_000 {
        gba.step();
        if gba.ppu_mut().take_frame_ready() {
            return;
        }
    }
    panic!("frame not produced within step limit");
}

fn save_png(framebuffer: &[u16; FRAME_PIXELS], path: &std::path::Path) {
    let file = File::create(path).unwrap();
    let w = BufWriter::new(file);
    let mut encoder = png::Encoder::new(w, SCREEN_WIDTH as u32, SCREEN_HEIGHT as u32);
    encoder.set_color(png::ColorType::Rgb);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().unwrap();

    let mut data = Vec::with_capacity(FRAME_PIXELS * 3);
    for &pixel in framebuffer.iter() {
        let r = (u32::from(pixel & 0x001f) * 255 / 31) as u8;
        let g = (u32::from((pixel >> 5) & 0x001f) * 255 / 31) as u8;
        let b = (u32::from((pixel >> 10) & 0x001f) * 255 / 31) as u8;
        data.push(r);
        data.push(g);
        data.push(b);
    }
    writer.write_image_data(&data).unwrap();
}

fn read_fb(gba: &Gba, x: usize, y: usize) -> u16 {
    gba.ppu().framebuffer()[y * SCREEN_WIDTH + x]
}

/// Center pixel of the given button block.
fn button_center(index: usize) -> (usize, usize) {
    let col = index % 5;
    let row = index / 5;
    let (bx, by) = block_origin(col, row);
    (bx + BLOCK_W / 2, by + BLOCK_H / 2)
}

fn output_dir() -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("test_output");
    fs::create_dir_all(&dir).unwrap();
    dir
}

// --- Tests ---

#[test]
fn keypad_visual_idle() {
    let mut gba = Gba::with_rom(idle_rom(200_000));
    gba.cpu_mut().set_pc(0x0800_0000);
    gba.bus_mut().write_16(DISPCNT_ADDR, MODE3_BG2);
    let out = output_dir();

    // No buttons pressed.
    paint_dashboard(&mut gba, 0);
    run_one_frame(&mut gba);
    save_png(gba.ppu().framebuffer(), &out.join("keypad_idle.png"));

    // KEYINPUT should read 0x03FF (all released, low-active).
    let keyinput = gba.bus().io().read_16(KEYINPUT_ADDR);
    assert_eq!(keyinput, 0x03FF);

    // Every button block center should be DARK_COLOR.
    for i in 0..10 {
        let (cx, cy) = button_center(i);
        assert_eq!(
            read_fb(&gba, cx, cy),
            DARK_COLOR,
            "button {} should be dark when idle",
            i
        );
    }
}

#[test]
fn keypad_visual_a_b_start() {
    let mut gba = Gba::with_rom(idle_rom(200_000));
    gba.cpu_mut().set_pc(0x0800_0000);
    gba.bus_mut().write_16(DISPCNT_ADDR, MODE3_BG2);
    let out = output_dir();

    // Press A, B, Start.
    gba.set_button_pressed(Button::A, true);
    gba.set_button_pressed(Button::B, true);
    gba.set_button_pressed(Button::Start, true);

    // Verify low-active KEYINPUT.
    let keyinput = gba.bus().io().read_16(KEYINPUT_ADDR);
    assert_eq!(keyinput & Button::A.mask(), 0, "A pressed → bit 0 = 0");
    assert_eq!(keyinput & Button::B.mask(), 0, "B pressed → bit 1 = 0");
    assert_eq!(keyinput & Button::Start.mask(), 0, "Start pressed → bit 3 = 0");
    assert_ne!(keyinput & Button::Up.mask(), 0, "Up released → bit 6 = 1");
    assert_ne!(keyinput & Button::L.mask(), 0, "L released → bit 9 = 1");

    let pressed_mask = (!keyinput) & 0x03FF;
    paint_dashboard(&mut gba, pressed_mask);
    run_one_frame(&mut gba);
    save_png(gba.ppu().framebuffer(), &out.join("keypad_a_b_start.png"));

    // A (index 0) should be bright red.
    let (cx, cy) = button_center(0);
    assert_eq!(read_fb(&gba, cx, cy), BUTTON_COLORS[0]);

    // B (index 1) should be bright green.
    let (cx, cy) = button_center(1);
    assert_eq!(read_fb(&gba, cx, cy), BUTTON_COLORS[1]);

    // Start (index 3) should be bright white.
    let (cx, cy) = button_center(3);
    assert_eq!(read_fb(&gba, cx, cy), BUTTON_COLORS[3]);

    // Select (index 2) should still be dark.
    let (cx, cy) = button_center(2);
    assert_eq!(read_fb(&gba, cx, cy), DARK_COLOR);

    // Up (index 6) should still be dark.
    let (cx, cy) = button_center(6);
    assert_eq!(read_fb(&gba, cx, cy), DARK_COLOR);
}

#[test]
fn keypad_visual_dpad() {
    let mut gba = Gba::with_rom(idle_rom(200_000));
    gba.cpu_mut().set_pc(0x0800_0000);
    gba.bus_mut().write_16(DISPCNT_ADDR, MODE3_BG2);
    let out = output_dir();

    // Press all four D-pad directions.
    gba.set_button_pressed(Button::Up, true);
    gba.set_button_pressed(Button::Down, true);
    gba.set_button_pressed(Button::Left, true);
    gba.set_button_pressed(Button::Right, true);

    let keyinput = gba.bus().io().read_16(KEYINPUT_ADDR);
    let pressed_mask = (!keyinput) & 0x03FF;
    assert_eq!(pressed_mask & Button::Up.mask(), Button::Up.mask());
    assert_eq!(pressed_mask & Button::Down.mask(), Button::Down.mask());
    assert_eq!(pressed_mask & Button::Left.mask(), Button::Left.mask());
    assert_eq!(pressed_mask & Button::Right.mask(), Button::Right.mask());
    assert_eq!(pressed_mask & Button::A.mask(), 0, "A should not be in pressed mask");

    paint_dashboard(&mut gba, pressed_mask);
    run_one_frame(&mut gba);
    save_png(gba.ppu().framebuffer(), &out.join("keypad_dpad.png"));

    // D-pad buttons (indices 4,5,6,7) should be bright.
    for &i in &[4usize, 5, 6, 7] {
        let (cx, cy) = button_center(i);
        assert_eq!(
            read_fb(&gba, cx, cy),
            BUTTON_COLORS[i],
            "D-pad button {} should be bright",
            i
        );
    }

    // Non-D-pad buttons should be dark.
    for &i in &[0usize, 1, 2, 3, 8, 9] {
        let (cx, cy) = button_center(i);
        assert_eq!(
            read_fb(&gba, cx, cy),
            DARK_COLOR,
            "button {} should be dark",
            i
        );
    }
}

#[test]
fn keypad_visual_all_pressed() {
    let mut gba = Gba::with_rom(idle_rom(200_000));
    gba.cpu_mut().set_pc(0x0800_0000);
    gba.bus_mut().write_16(DISPCNT_ADDR, MODE3_BG2);
    let out = output_dir();

    // Press every button.
    for &button in &BUTTON_ORDER {
        gba.set_button_pressed(button, true);
    }

    let keyinput = gba.bus().io().read_16(KEYINPUT_ADDR);
    assert_eq!(keyinput, 0x0000, "all 10 bits should be 0 when all pressed");

    let pressed_mask = (!keyinput) & 0x03FF;
    paint_dashboard(&mut gba, pressed_mask);
    run_one_frame(&mut gba);
    save_png(gba.ppu().framebuffer(), &out.join("keypad_all.png"));

    // Every button should show its bright color.
    for i in 0..10 {
        let (cx, cy) = button_center(i);
        assert_eq!(
            read_fb(&gba, cx, cy),
            BUTTON_COLORS[i],
            "button {} should be bright when all pressed",
            i
        );
    }
}

#[test]
fn keypad_visual_release_updates_display() {
    let mut gba = Gba::with_rom(idle_rom(200_000));
    gba.cpu_mut().set_pc(0x0800_0000);
    gba.bus_mut().write_16(DISPCNT_ADDR, MODE3_BG2);
    let out = output_dir();

    // Press A, render frame 1.
    gba.set_button_pressed(Button::A, true);
    let keyinput = gba.bus().io().read_16(KEYINPUT_ADDR);
    paint_dashboard(&mut gba, (!keyinput) & 0x03FF);
    run_one_frame(&mut gba);

    let (ax, ay) = button_center(0);
    assert_eq!(read_fb(&gba, ax, ay), BUTTON_COLORS[0], "A should be bright");

    // Release A, render frame 2.
    gba.set_button_pressed(Button::A, false);
    let keyinput = gba.bus().io().read_16(KEYINPUT_ADDR);
    assert_ne!(keyinput & Button::A.mask(), 0, "A should be released in KEYINPUT");

    paint_dashboard(&mut gba, (!keyinput) & 0x03FF);
    run_one_frame(&mut gba);
    save_png(gba.ppu().framebuffer(), &out.join("keypad_release.png"));

    assert_eq!(
        read_fb(&gba, ax, ay),
        DARK_COLOR,
        "A should be dark after release"
    );
}
