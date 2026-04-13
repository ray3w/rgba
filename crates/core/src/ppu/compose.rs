use super::{FRAME_PIXELS, SCREEN_WIDTH};

pub const BG_LAYER_COUNT: usize = 4;
pub const OBJ_LAYER_COUNT: usize = 1;
pub const TOTAL_LAYER_COUNT: usize = BG_LAYER_COUNT + OBJ_LAYER_COUNT;
pub const BG_ORDER_BASE: u8 = 0x80;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LayerPixel {
    pub color: u16,
    pub priority: u8,
    pub order: u8,
    pub transparent: bool,
}

impl LayerPixel {
    pub const fn transparent(priority: u8, order: u8) -> Self {
        Self {
            color: 0,
            priority,
            order,
            transparent: true,
        }
    }

    pub const fn opaque(color: u16, priority: u8, order: u8) -> Self {
        Self {
            color,
            priority,
            order,
            transparent: false,
        }
    }
}

pub fn clear_layer(layer: &mut [LayerPixel; SCREEN_WIDTH], priority: u8, order: u8) {
    layer.fill(LayerPixel::transparent(priority, order));
}

pub const fn bg_order(index: usize) -> u8 {
    BG_ORDER_BASE + index as u8
}

pub fn compose_layers_scanline<const N: usize>(
    framebuffer: &mut [u16; FRAME_PIXELS],
    y: usize,
    backdrop: u16,
    layers: &[[LayerPixel; SCREEN_WIDTH]; N],
) {
    let line_start = y * SCREEN_WIDTH;

    for x in 0..SCREEN_WIDTH {
        let mut best = LayerPixel::opaque(backdrop, 4, 4);

        for layer in layers.iter() {
            let candidate = layer[x];
            if candidate.transparent {
                continue;
            }

            if candidate.priority < best.priority
                || (candidate.priority == best.priority && candidate.order < best.order)
            {
                best = candidate;
            }
        }

        framebuffer[line_start + x] = best.color;
    }
}

#[cfg(test)]
mod tests {
    use super::{
        bg_order, clear_layer, compose_layers_scanline, LayerPixel, BG_LAYER_COUNT,
        TOTAL_LAYER_COUNT,
    };
    use crate::ppu::{FRAME_PIXELS, SCREEN_WIDTH};

    #[test]
    fn compose_prefers_lower_priority_number() {
        let mut framebuffer = Box::new([0; FRAME_PIXELS]);
        let mut layers = [[LayerPixel::transparent(3, 3); SCREEN_WIDTH]; BG_LAYER_COUNT];

        clear_layer(&mut layers[1], 1, bg_order(1));
        clear_layer(&mut layers[2], 0, bg_order(2));
        layers[1][0] = LayerPixel::opaque(0x001f, 1, bg_order(1));
        layers[2][0] = LayerPixel::opaque(0x03e0, 0, bg_order(2));

        compose_layers_scanline(&mut framebuffer, 0, 0x7c00, &layers);

        assert_eq!(framebuffer[0], 0x03e0);
    }

    #[test]
    fn compose_uses_lower_bg_index_as_tie_breaker() {
        let mut framebuffer = Box::new([0; FRAME_PIXELS]);
        let mut layers = [[LayerPixel::transparent(3, 3); SCREEN_WIDTH]; BG_LAYER_COUNT];

        clear_layer(&mut layers[0], 2, bg_order(0));
        clear_layer(&mut layers[1], 2, bg_order(1));
        layers[0][0] = LayerPixel::opaque(0x001f, 2, bg_order(0));
        layers[1][0] = LayerPixel::opaque(0x03e0, 2, bg_order(1));

        compose_layers_scanline(&mut framebuffer, 0, 0x7c00, &layers);

        assert_eq!(framebuffer[0], 0x001f);
    }

    #[test]
    fn compose_falls_back_to_backdrop_when_all_layers_are_transparent() {
        let mut framebuffer = Box::new([0; FRAME_PIXELS]);
        let layers = [[LayerPixel::transparent(3, 3); SCREEN_WIDTH]; BG_LAYER_COUNT];

        compose_layers_scanline(&mut framebuffer, 0, 0x56b5, &layers);

        assert_eq!(framebuffer[0], 0x56b5);
    }

    #[test]
    fn obj_order_beats_background_when_priorities_tie() {
        let mut framebuffer = Box::new([0; FRAME_PIXELS]);
        let mut layers = [[LayerPixel::transparent(3, 3); SCREEN_WIDTH]; TOTAL_LAYER_COUNT];

        clear_layer(&mut layers[0], 1, bg_order(0));
        clear_layer(&mut layers[BG_LAYER_COUNT], 1, 7);
        layers[0][0] = LayerPixel::opaque(0x001f, 1, bg_order(0));
        layers[BG_LAYER_COUNT][0] = LayerPixel::opaque(0x03e0, 1, 7);

        compose_layers_scanline(&mut framebuffer, 0, 0x7c00, &layers);

        assert_eq!(framebuffer[0], 0x03e0);
    }
}
