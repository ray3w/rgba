mod mode3;

use crate::io::IoRegs;

pub const SCREEN_WIDTH: usize = 240;
pub const SCREEN_HEIGHT: usize = 160;
pub const FRAME_PIXELS: usize = SCREEN_WIDTH * SCREEN_HEIGHT;

pub const HDRAW_CYCLES: u32 = 960;
pub const HBLANK_CYCLES: u32 = 272;
pub const SCANLINE_CYCLES: u32 = HDRAW_CYCLES + HBLANK_CYCLES;
pub const TOTAL_SCANLINES: u16 = 228;

/// Minimal LCD renderer for the early PPU phases.
#[derive(Debug, Clone)]
pub struct Ppu {
    vcount: u16,
    line_cycles: u32,
    framebuffer: Box<[u16; FRAME_PIXELS]>,
    frame_ready: bool,
}

impl Default for Ppu {
    fn default() -> Self {
        Self::new()
    }
}

impl Ppu {
    pub fn new() -> Self {
        Self {
            vcount: 0,
            line_cycles: 0,
            framebuffer: Box::new([0; FRAME_PIXELS]),
            frame_ready: false,
        }
    }

    pub fn vcount(&self) -> u16 {
        self.vcount
    }

    pub fn framebuffer(&self) -> &[u16; FRAME_PIXELS] {
        &self.framebuffer
    }

    pub fn frame_ready(&self) -> bool {
        self.frame_ready
    }

    pub fn take_frame_ready(&mut self) -> bool {
        let ready = self.frame_ready;
        self.frame_ready = false;
        ready
    }

    pub fn step(&mut self, cycles: u32, io: &mut IoRegs, vram: &[u8]) {
        let mut remaining = cycles;

        while remaining > 0 {
            let boundary = if self.line_cycles < HDRAW_CYCLES {
                HDRAW_CYCLES
            } else {
                SCANLINE_CYCLES
            };
            let advance = remaining.min(boundary - self.line_cycles);
            self.line_cycles += advance;
            remaining -= advance;

            if self.line_cycles == HDRAW_CYCLES {
                io.set_hblank(true);
                if self.vcount < SCREEN_HEIGHT as u16 {
                    self.render_visible_scanline(io, vram);
                }
            }

            if self.line_cycles == SCANLINE_CYCLES {
                self.finish_scanline(io);
            }
        }
    }

    pub fn write_xrgb8888(&self, out: &mut [u32]) {
        assert!(out.len() >= FRAME_PIXELS);

        for (dst, &src) in out.iter_mut().zip(self.framebuffer.iter()) {
            *dst = rgb555_to_xrgb8888(src);
        }
    }

    fn render_visible_scanline(&mut self, io: &IoRegs, vram: &[u8]) {
        let y = self.vcount as usize;
        if io.display_mode() == 3 && io.bg2_enabled() {
            mode3::render_scanline(&mut self.framebuffer, vram, y);
        } else {
            let start = y * SCREEN_WIDTH;
            self.framebuffer[start..start + SCREEN_WIDTH].fill(0);
        }
    }

    fn finish_scanline(&mut self, io: &mut IoRegs) {
        self.line_cycles = 0;
        io.set_hblank(false);

        self.vcount = self.vcount.wrapping_add(1);
        if self.vcount == SCREEN_HEIGHT as u16 {
            io.set_vblank(true);
            self.frame_ready = true;
        } else if self.vcount == TOTAL_SCANLINES {
            self.vcount = 0;
            io.set_vblank(false);
        }

        io.set_vcount(self.vcount);
    }
}

pub fn rgb555_to_xrgb8888(pixel: u16) -> u32 {
    let r = (u32::from(pixel & 0x001f) * 255) / 31;
    let g = (u32::from((pixel >> 5) & 0x001f) * 255) / 31;
    let b = (u32::from((pixel >> 10) & 0x001f) * 255) / 31;

    (r << 16) | (g << 8) | b
}

#[cfg(test)]
mod tests {
    use super::{
        rgb555_to_xrgb8888, Ppu, FRAME_PIXELS, HDRAW_CYCLES, SCANLINE_CYCLES, SCREEN_HEIGHT,
    };
    use crate::io::IoRegs;

    const MODE3_BG2: u16 = 0x0403;

    fn write_pixel(vram: &mut [u8], x: usize, y: usize, color: u16) {
        let offset = ((y * 240) + x) * 2;
        vram[offset] = color as u8;
        vram[offset + 1] = (color >> 8) as u8;
    }

    #[test]
    fn mode3_hblank_renders_visible_scanline() {
        let mut ppu = Ppu::new();
        let mut io = IoRegs::new();
        let mut vram = vec![0; 0x18000];
        io.write_16(0x0400_0000, MODE3_BG2);
        write_pixel(&mut vram, 0, 0, 0x001f);
        write_pixel(&mut vram, 1, 0, 0x03e0);

        ppu.step(HDRAW_CYCLES, &mut io, &vram);

        assert_eq!(ppu.framebuffer()[0], 0x001f);
        assert_eq!(ppu.framebuffer()[1], 0x03e0);
        assert_ne!(io.dispstat() & 0x0002, 0);
    }

    #[test]
    fn entering_vblank_marks_frame_ready_and_updates_vcount() {
        let mut ppu = Ppu::new();
        let mut io = IoRegs::new();
        let vram = vec![0; 0x18000];

        ppu.step(SCANLINE_CYCLES * SCREEN_HEIGHT as u32, &mut io, &vram);

        assert_eq!(ppu.vcount(), SCREEN_HEIGHT as u16);
        assert_eq!(io.vcount(), SCREEN_HEIGHT as u16);
        assert_ne!(io.dispstat() & 0x0001, 0);
        assert!(ppu.take_frame_ready());
        assert!(!ppu.take_frame_ready());
    }

    #[test]
    fn xrgb_conversion_expands_rgb555_channels() {
        assert_eq!(rgb555_to_xrgb8888(0x001f), 0x00ff_0000);
        assert_eq!(rgb555_to_xrgb8888(0x03e0), 0x0000_ff00);
        assert_eq!(rgb555_to_xrgb8888(0x7c00), 0x0000_00ff);
    }

    #[test]
    fn framebuffer_size_matches_visible_screen() {
        let ppu = Ppu::new();
        assert_eq!(ppu.framebuffer().len(), FRAME_PIXELS);
    }
}
