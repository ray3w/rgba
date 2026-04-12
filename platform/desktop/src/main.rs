use std::env;
use std::error::Error;

use minifb::{Key, Scale, Window, WindowOptions};
use rgba_core::arm7tdmi::BusInterface;
use rgba_core::{Cartridge, Gba, FRAME_PIXELS, SCREEN_HEIGHT, SCREEN_WIDTH};

const DISPCNT_ADDR: u32 = 0x0400_0000;
const VRAM_ADDR: u32 = 0x0600_0000;
const MODE3_BG2: u16 = 0x0403;
const IDLE_INSTRUCTION: u32 = 0xe3a0_0000; // MOV r0, #0

fn main() -> Result<(), Box<dyn Error>> {
    let mut gba = if let Some(path) = env::args_os().nth(1) {
        let mut gba = Gba::new(Cartridge::from_file(path)?);
        gba.cpu_mut().set_pc(0x0800_0000);
        gba
    } else {
        create_mode3_demo()
    };

    let mut window = Window::new(
        "RGBA - Phase 4 Viewer",
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

fn create_mode3_demo() -> Gba {
    let mut rom = Vec::with_capacity(4096);
    for _ in 0..1024 {
        rom.extend_from_slice(&IDLE_INSTRUCTION.to_le_bytes());
    }

    let mut gba = Gba::new(Cartridge::new(rom));
    gba.cpu_mut().set_pc(0x0800_0000);
    gba.bus_mut().write_16(DISPCNT_ADDR, MODE3_BG2);

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
