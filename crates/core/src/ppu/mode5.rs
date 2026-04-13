use crate::io::IoRegs;

use super::compose::{bg_order, bg_target, clear_layer, LayerPixel};
use super::SCREEN_WIDTH;
#[cfg(test)]
use super::{compose::TARGET_BACKDROP, FRAME_PIXELS};

const MODE5_WIDTH: usize = 160;
const MODE5_HEIGHT: usize = 128;
const FRAME_STRIDE: usize = 0x0a000;

pub fn render_layer(
    layer: &mut [LayerPixel; SCREEN_WIDTH],
    io: &IoRegs,
    vram: &[u8],
    y: usize,
    frame_select: bool,
) {
    let priority = io.bg_priority(2);
    let order = bg_order(2);
    clear_layer(layer, priority, order, bg_target(2));

    if !io.bg2_enabled() || y >= MODE5_HEIGHT {
        return;
    }

    let frame_base = if frame_select { FRAME_STRIDE } else { 0 };
    let line_start = frame_base + y * MODE5_WIDTH * 2;

    for x in 0..MODE5_WIDTH {
        let offset = line_start + x * 2;
        let lo = vram.get(offset).copied().unwrap_or(0) as u16;
        let hi = vram.get(offset + 1).copied().unwrap_or(0) as u16;
        layer[x] = LayerPixel::opaque(lo | (hi << 8), priority, order, bg_target(2));
    }
}

#[cfg(test)]
#[allow(dead_code)]
pub fn render_scanline(
    framebuffer: &mut [u16; FRAME_PIXELS],
    io: &IoRegs,
    vram: &[u8],
    y: usize,
    frame_select: bool,
    backdrop: u16,
) {
    let mut layer = [LayerPixel::transparent(4, u8::MAX, TARGET_BACKDROP); SCREEN_WIDTH];
    render_layer(&mut layer, io, vram, y, frame_select);

    let line_start = y * SCREEN_WIDTH;
    framebuffer[line_start..line_start + SCREEN_WIDTH].fill(backdrop);
    for x in 0..SCREEN_WIDTH {
        if !layer[x].transparent {
            framebuffer[line_start + x] = layer[x].color;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{render_layer, MODE5_WIDTH};
    use crate::io::IoRegs;
    use crate::ppu::{
        compose::{bg_target, LayerPixel, TARGET_BACKDROP},
        SCREEN_WIDTH,
    };

    #[test]
    fn mode5_renders_160_pixel_visible_area() {
        let mut io = IoRegs::new();
        let mut vram = vec![0; 0x18000];
        let mut layer = [LayerPixel::transparent(4, u8::MAX, TARGET_BACKDROP); SCREEN_WIDTH];

        io.write_16(0x0400_0000, 0x0405);
        vram[0] = 0x1f;
        vram[1] = 0x00;
        vram[(MODE5_WIDTH - 1) * 2] = 0xe0;
        vram[(MODE5_WIDTH - 1) * 2 + 1] = 0x03;

        render_layer(&mut layer, &io, &vram, 0, false);

        assert_eq!(layer[0], LayerPixel::opaque(0x001f, 0, 0x82, bg_target(2)));
        assert_eq!(
            layer[MODE5_WIDTH - 1],
            LayerPixel::opaque(0x03e0, 0, 0x82, bg_target(2))
        );
        assert!(layer[MODE5_WIDTH].transparent);
    }
}
