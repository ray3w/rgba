use super::{FRAME_PIXELS, SCREEN_WIDTH};

/// Render one visible scanline in Mode 3 directly from VRAM.
pub fn render_scanline(framebuffer: &mut [u16; FRAME_PIXELS], vram: &[u8], y: usize) {
    let line_start = y * SCREEN_WIDTH;
    let vram_start = line_start * 2;

    for x in 0..SCREEN_WIDTH {
        let offset = vram_start + (x * 2);
        let lo = vram.get(offset).copied().unwrap_or(0) as u16;
        let hi = vram.get(offset + 1).copied().unwrap_or(0) as u16;
        framebuffer[line_start + x] = lo | (hi << 8);
    }
}
