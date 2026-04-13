use super::{read_palette_color, FRAME_PIXELS, SCREEN_WIDTH};

const FRAME_STRIDE: usize = 0x0a000;

/// Render one visible scanline in Mode 4 using the BG palette.
pub fn render_scanline(
    framebuffer: &mut [u16; FRAME_PIXELS],
    vram: &[u8],
    palette: &[u8],
    y: usize,
    frame_select: bool,
) {
    let line_start = y * SCREEN_WIDTH;
    let frame_base = if frame_select { FRAME_STRIDE } else { 0 };
    let vram_start = frame_base + line_start;

    for x in 0..SCREEN_WIDTH {
        let palette_index = vram.get(vram_start + x).copied().unwrap_or(0) as usize;
        framebuffer[line_start + x] = read_palette_color(palette, palette_index);
    }
}
