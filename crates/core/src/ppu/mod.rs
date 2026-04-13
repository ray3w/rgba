mod bg;
mod compose;
mod effect;
mod mode3;
mod mode4;
mod mode5;
mod obj;
mod window;

use crate::io::IoRegs;
use compose::{
    bg_target, compose_layers_scanline_with_effects, LayerPixel, BG_LAYER_COUNT, TARGET_BACKDROP,
    TOTAL_LAYER_COUNT,
};
use effect::EffectConfig;
use window::build_window_scanline;

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

    pub fn step(&mut self, cycles: u32, io: &mut IoRegs, vram: &[u8], palette: &[u8], oam: &[u8]) {
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
                    self.render_visible_scanline(io, vram, palette, oam);
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

    fn render_visible_scanline(&mut self, io: &IoRegs, vram: &[u8], palette: &[u8], oam: &[u8]) {
        let y = self.vcount as usize;
        match io.display_mode() {
            0 => {
                let layers = bg::render_mode0_layers(io, vram, palette, y);
                self.compose_with_obj(io, palette, vram, oam, y, layers);
            }
            1 => {
                let layers = bg::render_mode1_layers(io, vram, palette, y);
                self.compose_with_obj(io, palette, vram, oam, y, layers);
            }
            2 => {
                let layers = bg::render_mode2_layers(io, vram, palette, y);
                self.compose_with_obj(io, palette, vram, oam, y, layers);
            }
            3 if io.bg2_enabled() => {
                let mut layers = blank_bg_layers();
                mode3::render_layer(&mut layers[2], io, vram, y);
                self.compose_with_obj(io, palette, vram, oam, y, layers);
            }
            4 if io.bg2_enabled() => {
                let mut layers = blank_bg_layers();
                mode4::render_layer(
                    &mut layers[2],
                    io,
                    vram,
                    palette,
                    y,
                    io.display_frame_select(),
                );
                self.compose_with_obj(io, palette, vram, oam, y, layers);
            }
            5 if io.bg2_enabled() => {
                let mut layers = blank_bg_layers();
                mode5::render_layer(&mut layers[2], io, vram, y, io.display_frame_select());
                self.compose_with_obj(io, palette, vram, oam, y, layers);
            }
            _ => fill_scanline(&mut self.framebuffer, y, backdrop_color(palette)),
        }
    }

    fn compose_with_obj(
        &mut self,
        io: &IoRegs,
        palette: &[u8],
        vram: &[u8],
        oam: &[u8],
        y: usize,
        bg_layers: [[LayerPixel; SCREEN_WIDTH]; BG_LAYER_COUNT],
    ) {
        let mut layers = [[LayerPixel::transparent(4, u8::MAX, TARGET_BACKDROP); SCREEN_WIDTH];
            TOTAL_LAYER_COUNT];
        layers[..BG_LAYER_COUNT].copy_from_slice(&bg_layers);
        obj::render_obj_layer(&mut layers[BG_LAYER_COUNT], io, vram, palette, oam, y);
        let window_masks = build_window_scanline(io, vram, oam, y);
        let effects = EffectConfig::from_io(io);
        compose_layers_scanline_with_effects(
            &mut self.framebuffer,
            y,
            backdrop_color(palette),
            &layers,
            &window_masks,
            effects,
        );
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

pub(crate) fn fill_scanline(framebuffer: &mut [u16; FRAME_PIXELS], y: usize, color: u16) {
    let start = y * SCREEN_WIDTH;
    framebuffer[start..start + SCREEN_WIDTH].fill(color);
}

pub(crate) fn read_palette_color(palette: &[u8], index: usize) -> u16 {
    let offset = (index * 2) % palette.len().max(1);
    let lo = palette.get(offset).copied().unwrap_or(0) as u16;
    let hi = palette.get(offset + 1).copied().unwrap_or(0) as u16;
    lo | (hi << 8)
}

pub(crate) fn backdrop_color(palette: &[u8]) -> u16 {
    read_palette_color(palette, 0)
}

fn blank_bg_layers() -> [[LayerPixel; SCREEN_WIDTH]; BG_LAYER_COUNT] {
    [
        [LayerPixel::transparent(4, u8::MAX, bg_target(0)); SCREEN_WIDTH],
        [LayerPixel::transparent(4, u8::MAX, bg_target(1)); SCREEN_WIDTH],
        [LayerPixel::transparent(4, u8::MAX, bg_target(2)); SCREEN_WIDTH],
        [LayerPixel::transparent(4, u8::MAX, bg_target(3)); SCREEN_WIDTH],
    ]
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

    const BG0_MODE0: u16 = 0x0100;
    const BG0_BG1_MODE0: u16 = 0x0300;
    const BG2_MODE1: u16 = 0x0401;
    const MODE3_BG2: u16 = 0x0403;
    const MODE4_BG2: u16 = 0x0404;
    const BG0CNT_BLOCK1: u16 = 0x0104;
    const BG1CNT_BLOCK1_PRIORITY0: u16 = 0x0104;
    const BG0CNT_PRIORITY1: u16 = 0x0001;
    const BG2CNT_BLOCK1: u16 = 0x0104;

    fn write_u16(slice: &mut [u8], offset: usize, value: u16) {
        slice[offset] = value as u8;
        slice[offset + 1] = (value >> 8) as u8;
    }

    fn write_pixel(vram: &mut [u8], x: usize, y: usize, color: u16) {
        let offset = ((y * 240) + x) * 2;
        write_u16(vram, offset, color);
    }

    #[test]
    fn mode3_hblank_renders_visible_scanline() {
        let mut ppu = Ppu::new();
        let mut io = IoRegs::new();
        let mut vram = vec![0; 0x18000];
        let palette = vec![0; 0x400];
        io.write_16(0x0400_0000, MODE3_BG2);
        write_pixel(&mut vram, 0, 0, 0x001f);
        write_pixel(&mut vram, 1, 0, 0x03e0);

        let oam = vec![0; 0x400];
        ppu.step(HDRAW_CYCLES, &mut io, &vram, &palette, &oam);

        assert_eq!(ppu.framebuffer()[0], 0x001f);
        assert_eq!(ppu.framebuffer()[1], 0x03e0);
        assert_ne!(io.dispstat() & 0x0002, 0);
    }

    #[test]
    fn mode4_hblank_renders_palette_bitmap_scanline() {
        let mut ppu = Ppu::new();
        let mut io = IoRegs::new();
        let mut vram = vec![0; 0x18000];
        let mut palette = vec![0; 0x400];
        io.write_16(0x0400_0000, MODE4_BG2);
        write_u16(&mut palette, 2, 0x001f);
        write_u16(&mut palette, 4, 0x03e0);
        vram[0] = 1;
        vram[1] = 2;

        let oam = vec![0; 0x400];
        ppu.step(HDRAW_CYCLES, &mut io, &vram, &palette, &oam);

        assert_eq!(ppu.framebuffer()[0], 0x001f);
        assert_eq!(ppu.framebuffer()[1], 0x03e0);
    }

    #[test]
    fn mode0_bg0_renders_text_tiles_from_vram() {
        let mut ppu = Ppu::new();
        let mut io = IoRegs::new();
        let mut vram = vec![0; 0x18000];
        let mut palette = vec![0; 0x400];
        io.write_16(0x0400_0000, BG0_MODE0);
        io.write_16(0x0400_0008, BG0CNT_BLOCK1);
        write_u16(&mut palette, 0, 0x0000);
        write_u16(&mut palette, 2, 0x001f);
        write_u16(&mut palette, 4, 0x03e0);

        // Tile 0 in char block 1: alternating red/green on the first row.
        vram[0x4000] = 0x21;
        vram[0x4001] = 0x21;
        vram[0x4002] = 0x21;
        vram[0x4003] = 0x21;
        // Screen block 1 entry 0 -> tile 0.
        write_u16(&mut vram, 0x0800, 0x0000);

        let oam = vec![0; 0x400];
        ppu.step(HDRAW_CYCLES, &mut io, &vram, &palette, &oam);

        assert_eq!(ppu.framebuffer()[0], 0x001f);
        assert_eq!(ppu.framebuffer()[1], 0x03e0);
        assert_eq!(ppu.framebuffer()[2], 0x001f);
        assert_eq!(ppu.framebuffer()[3], 0x03e0);
    }

    #[test]
    fn mode0_composes_multiple_backgrounds_by_priority() {
        let mut ppu = Ppu::new();
        let mut io = IoRegs::new();
        let mut vram = vec![0; 0x18000];
        let mut palette = vec![0; 0x400];
        io.write_16(0x0400_0000, BG0_BG1_MODE0);
        io.write_16(0x0400_0008, BG0CNT_PRIORITY1);
        io.write_16(0x0400_000a, BG1CNT_BLOCK1_PRIORITY0);
        write_u16(&mut palette, 0, 0x0000);
        write_u16(&mut palette, 2, 0x001f);
        write_u16(&mut palette, 4, 0x03e0);

        vram[0x0000] = 0x11;
        vram[0x0001] = 0x11;
        vram[0x0002] = 0x11;
        vram[0x0003] = 0x11;
        write_u16(&mut vram, 0x0000, 0x0000);

        vram[0x4000] = 0x22;
        vram[0x4001] = 0x22;
        vram[0x4002] = 0x22;
        vram[0x4003] = 0x22;
        write_u16(&mut vram, 0x0800, 0x0000);

        let oam = vec![0; 0x400];
        ppu.step(HDRAW_CYCLES, &mut io, &vram, &palette, &oam);

        assert_eq!(ppu.framebuffer()[0], 0x03e0);
        assert_eq!(ppu.framebuffer()[1], 0x03e0);
    }

    #[test]
    fn mode1_renders_affine_bg2_with_identity_transform() {
        let mut ppu = Ppu::new();
        let mut io = IoRegs::new();
        let mut vram = vec![0; 0x18000];
        let mut palette = vec![0; 0x400];
        io.write_16(0x0400_0000, BG2_MODE1);
        io.write_16(0x0400_000c, BG2CNT_BLOCK1);
        io.write_16(0x0400_0020, 0x0100);
        io.write_16(0x0400_0022, 0x0000);
        io.write_16(0x0400_0024, 0x0000);
        io.write_16(0x0400_0026, 0x0100);

        write_u16(&mut palette, 2, 0x001f);
        write_u16(&mut palette, 4, 0x03e0);
        vram[0x4000] = 1;
        vram[0x4001] = 2;
        vram[0x0800] = 0;

        let oam = vec![0; 0x400];
        ppu.step(HDRAW_CYCLES, &mut io, &vram, &palette, &oam);

        assert_eq!(ppu.framebuffer()[0], 0x001f);
        assert_eq!(ppu.framebuffer()[1], 0x03e0);
    }

    #[test]
    fn entering_vblank_marks_frame_ready_and_updates_vcount() {
        let mut ppu = Ppu::new();
        let mut io = IoRegs::new();
        let vram = vec![0; 0x18000];
        let palette = vec![0; 0x400];

        let oam = vec![0; 0x400];
        ppu.step(
            SCANLINE_CYCLES * SCREEN_HEIGHT as u32,
            &mut io,
            &vram,
            &palette,
            &oam,
        );

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

    #[test]
    fn mode0_obj_pixels_render_on_top_of_backdrop() {
        let mut ppu = Ppu::new();
        let mut io = IoRegs::new();
        let mut vram = vec![0; 0x18000];
        let mut palette = vec![0; 0x400];
        let mut oam = vec![0; 0x400];

        io.write_16(0x0400_0000, 0x1040);
        write_u16(&mut palette, 0x200 + 2, 0x001f);
        write_u16(&mut oam, 0x0000, 0x0000);
        write_u16(&mut oam, 0x0002, 0x0000);
        write_u16(&mut oam, 0x0004, 0x0000);
        vram[0x1_0000] = 0x11;

        ppu.step(HDRAW_CYCLES, &mut io, &vram, &palette, &oam);

        assert_eq!(ppu.framebuffer()[0], 0x001f);
    }
}
