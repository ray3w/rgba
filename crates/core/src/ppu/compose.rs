use super::effect::{alpha_blend, brighten, darken, BlendMode, EffectConfig};
use super::window::{WindowMask, WindowMaskLine};
use super::{FRAME_PIXELS, SCREEN_WIDTH};

pub const BG_LAYER_COUNT: usize = 4;
pub const OBJ_LAYER_COUNT: usize = 1;
pub const TOTAL_LAYER_COUNT: usize = BG_LAYER_COUNT + OBJ_LAYER_COUNT;
pub const BG_ORDER_BASE: u8 = 0x80;
pub const TARGET_BG0: u16 = 1 << 0;
pub const TARGET_BG1: u16 = 1 << 1;
pub const TARGET_BG2: u16 = 1 << 2;
pub const TARGET_BG3: u16 = 1 << 3;
pub const TARGET_OBJ: u16 = 1 << 4;
pub const TARGET_BACKDROP: u16 = 1 << 5;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LayerPixel {
    pub color: u16,
    pub priority: u8,
    pub order: u8,
    pub transparent: bool,
    pub semi_transparent: bool,
    pub target_bit: u16,
}

impl LayerPixel {
    pub const fn transparent(priority: u8, order: u8, target_bit: u16) -> Self {
        Self {
            color: 0,
            priority,
            order,
            transparent: true,
            semi_transparent: false,
            target_bit,
        }
    }

    pub const fn opaque(color: u16, priority: u8, order: u8, target_bit: u16) -> Self {
        Self {
            color,
            priority,
            order,
            transparent: false,
            semi_transparent: false,
            target_bit,
        }
    }

    pub const fn semi_transparent(color: u16, priority: u8, order: u8, target_bit: u16) -> Self {
        Self {
            color,
            priority,
            order,
            transparent: false,
            semi_transparent: true,
            target_bit,
        }
    }
}

pub fn clear_layer(
    layer: &mut [LayerPixel; SCREEN_WIDTH],
    priority: u8,
    order: u8,
    target_bit: u16,
) {
    layer.fill(LayerPixel::transparent(priority, order, target_bit));
}

pub const fn bg_order(index: usize) -> u8 {
    BG_ORDER_BASE + index as u8
}

pub const fn bg_target(index: usize) -> u16 {
    match index {
        0 => TARGET_BG0,
        1 => TARGET_BG1,
        2 => TARGET_BG2,
        _ => TARGET_BG3,
    }
}

#[cfg_attr(not(test), allow(dead_code))]
pub fn compose_layers_scanline<const N: usize>(
    framebuffer: &mut [u16; FRAME_PIXELS],
    y: usize,
    backdrop: u16,
    layers: &[[LayerPixel; SCREEN_WIDTH]; N],
) {
    compose_layers_scanline_with_effects(
        framebuffer,
        y,
        backdrop,
        layers,
        &[WindowMask::all_visible(); SCREEN_WIDTH],
        EffectConfig::new(BlendMode::Off, 0, 0, 0, 0, 0),
    );
}

pub fn compose_layers_scanline_with_effects<const N: usize>(
    framebuffer: &mut [u16; FRAME_PIXELS],
    y: usize,
    backdrop: u16,
    layers: &[[LayerPixel; SCREEN_WIDTH]; N],
    window_masks: &WindowMaskLine,
    effects: EffectConfig,
) {
    let line_start = y * SCREEN_WIDTH;

    for x in 0..SCREEN_WIDTH {
        let window = window_masks[x];
        let backdrop_pixel = LayerPixel::opaque(backdrop, 4, u8::MAX, TARGET_BACKDROP);
        let (top, second) = select_top_two_layers(layers, x, window, backdrop_pixel);
        framebuffer[line_start + x] = apply_effects(top, second, window, effects);
    }
}

fn select_top_two_layers<const N: usize>(
    layers: &[[LayerPixel; SCREEN_WIDTH]; N],
    x: usize,
    window: WindowMask,
    backdrop: LayerPixel,
) -> (LayerPixel, LayerPixel) {
    let mut best = backdrop;
    let mut second = backdrop;

    for (layer_index, layer) in layers.iter().enumerate() {
        if !window.layer_visible(layer_index) {
            continue;
        }

        let candidate = layer[x];
        if candidate.transparent {
            continue;
        }

        if is_better(candidate, best) {
            second = best;
            best = candidate;
        } else if is_better(candidate, second) {
            second = candidate;
        }
    }

    (best, second)
}

fn is_better(candidate: LayerPixel, incumbent: LayerPixel) -> bool {
    candidate.priority < incumbent.priority
        || (candidate.priority == incumbent.priority && candidate.order < incumbent.order)
}

fn apply_effects(
    top: LayerPixel,
    second: LayerPixel,
    window: WindowMask,
    effects: EffectConfig,
) -> u16 {
    if !window.color_effect {
        return top.color;
    }

    if top.semi_transparent {
        if effects.second_target_enabled(second.target_bit) {
            return alpha_blend(top.color, second.color, effects.eva, effects.evb);
        }
        return top.color;
    }

    match effects.mode {
        BlendMode::Off => top.color,
        BlendMode::Alpha => {
            if effects.first_target_enabled(top.target_bit)
                && effects.second_target_enabled(second.target_bit)
            {
                alpha_blend(top.color, second.color, effects.eva, effects.evb)
            } else {
                top.color
            }
        }
        BlendMode::Brighten => {
            if effects.first_target_enabled(top.target_bit) {
                brighten(top.color, effects.evy)
            } else {
                top.color
            }
        }
        BlendMode::Darken => {
            if effects.first_target_enabled(top.target_bit) {
                darken(top.color, effects.evy)
            } else {
                top.color
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        bg_order, bg_target, clear_layer, compose_layers_scanline,
        compose_layers_scanline_with_effects, BlendMode, EffectConfig, LayerPixel, BG_LAYER_COUNT,
        TARGET_BACKDROP, TARGET_OBJ, TOTAL_LAYER_COUNT,
    };
    use crate::ppu::window::WindowMask;
    use crate::ppu::{FRAME_PIXELS, SCREEN_WIDTH};

    #[test]
    fn compose_prefers_lower_priority_number() {
        let mut framebuffer = Box::new([0; FRAME_PIXELS]);
        let mut layers =
            [[LayerPixel::transparent(3, 3, TARGET_BACKDROP); SCREEN_WIDTH]; BG_LAYER_COUNT];

        clear_layer(&mut layers[1], 1, bg_order(1), bg_target(1));
        clear_layer(&mut layers[2], 0, bg_order(2), bg_target(2));
        layers[1][0] = LayerPixel::opaque(0x001f, 1, bg_order(1), bg_target(1));
        layers[2][0] = LayerPixel::opaque(0x03e0, 0, bg_order(2), bg_target(2));

        compose_layers_scanline(&mut framebuffer, 0, 0x7c00, &layers);

        assert_eq!(framebuffer[0], 0x03e0);
    }

    #[test]
    fn compose_uses_lower_bg_index_as_tie_breaker() {
        let mut framebuffer = Box::new([0; FRAME_PIXELS]);
        let mut layers =
            [[LayerPixel::transparent(3, 3, TARGET_BACKDROP); SCREEN_WIDTH]; BG_LAYER_COUNT];

        clear_layer(&mut layers[0], 2, bg_order(0), bg_target(0));
        clear_layer(&mut layers[1], 2, bg_order(1), bg_target(1));
        layers[0][0] = LayerPixel::opaque(0x001f, 2, bg_order(0), bg_target(0));
        layers[1][0] = LayerPixel::opaque(0x03e0, 2, bg_order(1), bg_target(1));

        compose_layers_scanline(&mut framebuffer, 0, 0x7c00, &layers);

        assert_eq!(framebuffer[0], 0x001f);
    }

    #[test]
    fn compose_falls_back_to_backdrop_when_all_layers_are_transparent() {
        let mut framebuffer = Box::new([0; FRAME_PIXELS]);
        let layers =
            [[LayerPixel::transparent(3, 3, TARGET_BACKDROP); SCREEN_WIDTH]; BG_LAYER_COUNT];

        compose_layers_scanline(&mut framebuffer, 0, 0x56b5, &layers);

        assert_eq!(framebuffer[0], 0x56b5);
    }

    #[test]
    fn obj_order_beats_background_when_priorities_tie() {
        let mut framebuffer = Box::new([0; FRAME_PIXELS]);
        let mut layers =
            [[LayerPixel::transparent(3, 3, TARGET_BACKDROP); SCREEN_WIDTH]; TOTAL_LAYER_COUNT];

        clear_layer(&mut layers[0], 1, bg_order(0), bg_target(0));
        clear_layer(&mut layers[BG_LAYER_COUNT], 1, 7, TARGET_OBJ);
        layers[0][0] = LayerPixel::opaque(0x001f, 1, bg_order(0), bg_target(0));
        layers[BG_LAYER_COUNT][0] = LayerPixel::opaque(0x03e0, 1, 7, TARGET_OBJ);

        compose_layers_scanline(&mut framebuffer, 0, 0x7c00, &layers);

        assert_eq!(framebuffer[0], 0x03e0);
    }

    #[test]
    fn window_mask_can_hide_obj_layer() {
        let mut framebuffer = Box::new([0; FRAME_PIXELS]);
        let mut layers =
            [[LayerPixel::transparent(3, 3, TARGET_BACKDROP); SCREEN_WIDTH]; TOTAL_LAYER_COUNT];
        let mut windows = [WindowMask::all_visible(); SCREEN_WIDTH];

        clear_layer(&mut layers[0], 1, bg_order(0), bg_target(0));
        clear_layer(&mut layers[BG_LAYER_COUNT], 1, 7, TARGET_OBJ);
        layers[0][0] = LayerPixel::opaque(0x001f, 1, bg_order(0), bg_target(0));
        layers[BG_LAYER_COUNT][0] = LayerPixel::opaque(0x03e0, 1, 7, TARGET_OBJ);
        windows[0].obj = false;

        compose_layers_scanline_with_effects(
            &mut framebuffer,
            0,
            0x7c00,
            &layers,
            &windows,
            EffectConfig::new(BlendMode::Off, 0, 0, 0, 0, 0),
        );

        assert_eq!(framebuffer[0], 0x001f);
    }

    #[test]
    fn alpha_blend_uses_backdrop_as_second_target() {
        let mut framebuffer = Box::new([0; FRAME_PIXELS]);
        let mut layers =
            [[LayerPixel::transparent(3, 3, TARGET_BACKDROP); SCREEN_WIDTH]; TOTAL_LAYER_COUNT];

        clear_layer(&mut layers[BG_LAYER_COUNT], 1, 7, TARGET_OBJ);
        layers[BG_LAYER_COUNT][0] = LayerPixel::semi_transparent(0x001f, 1, 7, TARGET_OBJ);

        compose_layers_scanline_with_effects(
            &mut framebuffer,
            0,
            0x03e0,
            &layers,
            &[WindowMask::all_visible(); SCREEN_WIDTH],
            EffectConfig::new(BlendMode::Alpha, TARGET_OBJ, TARGET_BACKDROP, 8, 8, 0),
        );

        assert_eq!(framebuffer[0], 0x01ef);
    }
}
