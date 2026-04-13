use crate::io::IoRegs;

use super::compose::{bg_order, bg_target, clear_layer, LayerPixel};
#[cfg(test)]
use super::FRAME_PIXELS;
use super::SCREEN_WIDTH;

pub fn render_layer(layer: &mut [LayerPixel; SCREEN_WIDTH], io: &IoRegs, vram: &[u8], y: usize) {
    let priority = io.bg_priority(2);
    let order = bg_order(2);
    clear_layer(layer, priority, order, bg_target(2));

    if !io.bg2_enabled() {
        return;
    }

    let line_start = y * SCREEN_WIDTH;
    let vram_start = line_start * 2;

    for (x, pixel) in layer.iter_mut().enumerate() {
        let offset = vram_start + (x * 2);
        let lo = vram.get(offset).copied().unwrap_or(0) as u16;
        let hi = vram.get(offset + 1).copied().unwrap_or(0) as u16;
        *pixel = LayerPixel::opaque(lo | (hi << 8), priority, order, bg_target(2));
    }
}

#[cfg(test)]
#[allow(dead_code)]
/// Render one visible scanline in Mode 3 directly from VRAM.
pub fn render_scanline(framebuffer: &mut [u16; FRAME_PIXELS], vram: &[u8], y: usize) {
    let mut io = IoRegs::new();
    let mut layer =
        [LayerPixel::transparent(4, u8::MAX, super::compose::TARGET_BACKDROP); SCREEN_WIDTH];
    io.write_16(0x0400_0000, 0x0403);
    render_layer(&mut layer, &io, vram, y);
    let line_start = y * SCREEN_WIDTH;
    for x in 0..SCREEN_WIDTH {
        framebuffer[line_start + x] = layer[x].color;
    }
}
