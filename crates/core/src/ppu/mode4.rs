use crate::io::IoRegs;

use super::compose::{bg_order, bg_target, clear_layer, LayerPixel};
#[cfg(test)]
use super::FRAME_PIXELS;
use super::{read_palette_color, SCREEN_WIDTH};

const FRAME_STRIDE: usize = 0x0a000;

pub fn render_layer(
    layer: &mut [LayerPixel; SCREEN_WIDTH],
    io: &IoRegs,
    vram: &[u8],
    palette: &[u8],
    y: usize,
    frame_select: bool,
) {
    let priority = io.bg_priority(2);
    let order = bg_order(2);
    clear_layer(layer, priority, order, bg_target(2));

    if !io.bg2_enabled() {
        return;
    }

    let frame_base = if frame_select { FRAME_STRIDE } else { 0 };
    let vram_start = frame_base + y * SCREEN_WIDTH;

    for (x, pixel) in layer.iter_mut().enumerate() {
        let palette_index = vram.get(vram_start + x).copied().unwrap_or(0) as usize;
        *pixel = LayerPixel::opaque(
            read_palette_color(palette, palette_index),
            priority,
            order,
            bg_target(2),
        );
    }
}

#[cfg(test)]
#[allow(dead_code)]
/// Render one visible scanline in Mode 4 using the BG palette.
pub fn render_scanline(
    framebuffer: &mut [u16; FRAME_PIXELS],
    vram: &[u8],
    palette: &[u8],
    y: usize,
    frame_select: bool,
) {
    let mut io = IoRegs::new();
    let mut layer =
        [LayerPixel::transparent(4, u8::MAX, super::compose::TARGET_BACKDROP); SCREEN_WIDTH];
    io.write_16(0x0400_0000, 0x0404);
    render_layer(&mut layer, &io, vram, palette, y, frame_select);
    let line_start = y * SCREEN_WIDTH;
    for x in 0..SCREEN_WIDTH {
        framebuffer[line_start + x] = layer[x].color;
    }
}
