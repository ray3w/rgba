use std::env;
use std::error::Error;

use minifb::{Key, Scale, Window, WindowOptions};
use rgba_core::arm7tdmi::BusInterface;
use rgba_core::{Button, Cartridge, Gba, FRAME_PIXELS, SCREEN_HEIGHT, SCREEN_WIDTH};

const DISPCNT_ADDR: u32 = 0x0400_0000;
const VRAM_ADDR: u32 = 0x0600_0000;
const MODE3_BG2: u16 = 0x0403;
const IDLE_INSTRUCTION: u32 = 0xe3a0_0000; // MOV r0, #0

// --- Keypad dashboard layout ---

const BLOCK_W: usize = 40;
const BLOCK_H: usize = 50;
const GAP_X: usize = 8;
const GAP_Y: usize = 20;
const MARGIN_X: usize = 8;
const MARGIN_Y: usize = 20;
const BORDER: usize = 2;

const BG_COLOR: u16 = 0x1084;
const DARK_COLOR: u16 = 0x294A;

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

enum Mode {
    Rom,
    GradientDemo,
    KeypadDemo,
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().collect();
    let keypad_demo = args.iter().any(|a| a == "--keypad-demo");

    let (mut gba, mode) = if keypad_demo {
        (create_idle_gba(), Mode::KeypadDemo)
    } else if let Some(path) = env::args_os().nth(1) {
        let mut gba = Gba::new(Cartridge::from_file(path)?);
        gba.cpu_mut().set_pc(0x0800_0000);
        (gba, Mode::Rom)
    } else {
        (create_gradient_demo(), Mode::GradientDemo)
    };

    let title = match mode {
        Mode::KeypadDemo => "RGBA - Keypad Demo  (Z/X=A/B  Enter=Start  Space=Sel  Arrows=DPad  A/S=L/R)",
        _ => "RGBA - GBA Emulator",
    };

    let mut window = Window::new(
        title,
        SCREEN_WIDTH,
        SCREEN_HEIGHT,
        WindowOptions {
            resize: true,
            scale: Scale::X4,
            ..WindowOptions::default()
        },
    )?;
    window.set_target_fps(60);

    let mut buffer = vec![0; FRAME_PIXELS];

    while window.is_open() && !window.is_key_down(Key::Escape) {
        sync_input(&window, &mut gba);

        if matches!(mode, Mode::KeypadDemo) {
            let keyinput = gba.bus().io().keyinput();
            let pressed_mask = (!keyinput) & 0x03FF;
            paint_dashboard(&mut gba, pressed_mask);
        }

        let mut produced_frame = false;
        for _ in 0..50_000 {
            gba.step();
            if gba.ppu_mut().take_frame_ready() {
                produced_frame = true;
                break;
            }
        }

        if produced_frame {
            gba.ppu().write_xrgb8888(&mut buffer);
            window.update_with_buffer(&buffer, SCREEN_WIDTH, SCREEN_HEIGHT)?;
        } else {
            window.update();
        }
    }

    Ok(())
}

fn sync_input(window: &Window, gba: &mut Gba) {
    let bindings = [
        (Key::Z, Button::A),
        (Key::X, Button::B),
        (Key::Space, Button::Select),
        (Key::Enter, Button::Start),
        (Key::Right, Button::Right),
        (Key::Left, Button::Left),
        (Key::Up, Button::Up),
        (Key::Down, Button::Down),
        (Key::A, Button::L),
        (Key::S, Button::R),
    ];

    for (host, gba_button) in bindings {
        gba.set_button_pressed(gba_button, window.is_key_down(host));
    }
}

fn create_idle_gba() -> Gba {
    let mut rom = Vec::with_capacity(4096);
    for _ in 0..1024 {
        rom.extend_from_slice(&IDLE_INSTRUCTION.to_le_bytes());
    }
    let mut gba = Gba::new(Cartridge::new(rom));
    gba.cpu_mut().set_pc(0x0800_0000);
    gba.bus_mut().write_16(DISPCNT_ADDR, MODE3_BG2);
    gba
}

fn create_gradient_demo() -> Gba {
    let mut gba = create_idle_gba();

    for y in 0..SCREEN_HEIGHT {
        for x in 0..SCREEN_WIDTH {
            let r = (x as u16 * 31) / (SCREEN_WIDTH as u16 - 1);
            let g = (y as u16 * 31) / (SCREEN_HEIGHT as u16 - 1);
            let b = (((x + y) as u16) * 31) / ((SCREEN_WIDTH + SCREEN_HEIGHT - 2) as u16);
            let color = r | (g << 5) | (b << 10);
            let addr = VRAM_ADDR + (((y * SCREEN_WIDTH) + x) as u32 * 2);
            gba.bus_mut().write_16(addr, color);
        }
    }

    gba
}

// --- Keypad dashboard painting ---

fn block_origin(col: usize, row: usize) -> (usize, usize) {
    (
        MARGIN_X + col * (BLOCK_W + GAP_X),
        MARGIN_Y + row * (BLOCK_H + GAP_Y),
    )
}

fn write_vram_pixel(gba: &mut Gba, x: usize, y: usize, color: u16) {
    let addr = VRAM_ADDR + ((y * SCREEN_WIDTH + x) as u32 * 2);
    gba.bus_mut().write_16(addr, color);
}

fn paint_dashboard(gba: &mut Gba, pressed_mask: u16) {
    for y in 0..SCREEN_HEIGHT {
        for x in 0..SCREEN_WIDTH {
            write_vram_pixel(gba, x, y, BG_COLOR);
        }
    }

    for (i, &button) in BUTTON_ORDER.iter().enumerate() {
        let col = i % 5;
        let row = i / 5;
        let (bx, by) = block_origin(col, row);
        let is_pressed = (pressed_mask & button.mask()) != 0;
        let fill = if is_pressed { BUTTON_COLORS[i] } else { DARK_COLOR };
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
