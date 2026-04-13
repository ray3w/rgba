use crate::io::IoRegs;

use super::compose::{clear_layer, LayerPixel};
use super::{read_palette_color, SCREEN_HEIGHT, SCREEN_WIDTH};

const OBJ_COUNT: usize = 128;
const OAM_ENTRY_SIZE: usize = 8;
const OBJ_TILE_BASE: usize = 0x1_0000;
const OBJ_PALETTE_BASE: usize = 256;

const ATTR0_AFFINE: u16 = 1 << 8;
const ATTR0_DISABLED: u16 = 1 << 9;
const ATTR0_8BPP: u16 = 1 << 13;
const ATTR0_SHAPE_MASK: u16 = 0xc000;
const ATTR0_WINDOW: u16 = 1 << 11;

const ATTR1_X_MASK: u16 = 0x01ff;
const ATTR1_HFLIP: u16 = 1 << 12;
const ATTR1_VFLIP: u16 = 1 << 13;
const ATTR1_SIZE_MASK: u16 = 0xc000;

const ATTR2_TILE_MASK: u16 = 0x03ff;
const ATTR2_PRIORITY_MASK: u16 = 0x0c00;
const ATTR2_PALETTE_MASK: u16 = 0xf000;

const OBJ_SIZE_TABLE: [[[usize; 2]; 4]; 3] = [
    [[8, 8], [16, 16], [32, 32], [64, 64]],
    [[16, 8], [32, 8], [32, 16], [64, 32]],
    [[8, 16], [8, 32], [16, 32], [32, 64]],
];

pub fn render_obj_layer(
    layer: &mut [LayerPixel; SCREEN_WIDTH],
    io: &IoRegs,
    vram: &[u8],
    palette: &[u8],
    oam: &[u8],
    y: usize,
) {
    clear_layer(layer, 4, u8::MAX);

    if !io.obj_enabled() {
        return;
    }

    let mapping_1d = io.obj_mapping_1d();
    let scanline_y = y as i32;

    for obj_index in 0..OBJ_COUNT {
        let oam_offset = obj_index * OAM_ENTRY_SIZE;
        let attr0 = read_u16(oam, oam_offset);
        let attr1 = read_u16(oam, oam_offset + 2);
        let attr2 = read_u16(oam, oam_offset + 4);

        if !regular_obj_visible(attr0) {
            continue;
        }

        let Some((width, height)) = obj_dimensions(attr0, attr1) else {
            continue;
        };

        let obj_x = obj_screen_x(attr1);
        let obj_y = obj_screen_y(attr0);
        if scanline_y < obj_y || scanline_y >= obj_y + height as i32 {
            continue;
        }

        let bpp8 = (attr0 & ATTR0_8BPP) != 0;
        let hflip = (attr1 & ATTR1_HFLIP) != 0;
        let vflip = (attr1 & ATTR1_VFLIP) != 0;
        let priority = ((attr2 & ATTR2_PRIORITY_MASK) >> 10) as u8;
        let palette_bank = ((attr2 & ATTR2_PALETTE_MASK) >> 12) as usize;
        let obj_order = obj_index as u8;
        let tile_id = usize::from(attr2 & ATTR2_TILE_MASK);

        let row_in_obj = (scanline_y - obj_y) as usize;
        let source_y = if vflip { height - 1 - row_in_obj } else { row_in_obj };

        for local_x in 0..width {
            let screen_x = obj_x + local_x as i32;
            if !(0..SCREEN_WIDTH as i32).contains(&screen_x) {
                continue;
            }

            let source_x = if hflip { width - 1 - local_x } else { local_x };
            let color = fetch_obj_pixel(
                vram,
                palette,
                tile_id,
                source_x,
                source_y,
                width,
                bpp8,
                mapping_1d,
                palette_bank,
            );

            let Some(color) = color else {
                continue;
            };

            let pixel = &mut layer[screen_x as usize];
            if pixel.transparent
                || priority < pixel.priority
                || (priority == pixel.priority && obj_order < pixel.order)
            {
                *pixel = LayerPixel::opaque(color, priority, obj_order);
            }
        }
    }
}

fn regular_obj_visible(attr0: u16) -> bool {
    if (attr0 & ATTR0_WINDOW) != 0 {
        return false;
    }

    match attr0 & (ATTR0_AFFINE | ATTR0_DISABLED) {
        0 => true,
        ATTR0_DISABLED => false,
        _ => false,
    }
}

fn obj_dimensions(attr0: u16, attr1: u16) -> Option<(usize, usize)> {
    let shape = ((attr0 & ATTR0_SHAPE_MASK) >> 14) as usize;
    let size = ((attr1 & ATTR1_SIZE_MASK) >> 14) as usize;
    let dims = OBJ_SIZE_TABLE.get(shape)?.get(size)?;
    Some((dims[0], dims[1]))
}

fn obj_screen_x(attr1: u16) -> i32 {
    let raw = i32::from(attr1 & ATTR1_X_MASK);
    if raw >= SCREEN_WIDTH as i32 {
        raw - 512
    } else {
        raw
    }
}

fn obj_screen_y(attr0: u16) -> i32 {
    let raw = i32::from(attr0 & 0x00ff);
    if raw >= SCREEN_HEIGHT as i32 {
        raw - 256
    } else {
        raw
    }
}

fn fetch_obj_pixel(
    vram: &[u8],
    palette: &[u8],
    tile_id: usize,
    x: usize,
    y: usize,
    obj_width: usize,
    bpp8: bool,
    mapping_1d: bool,
    palette_bank: usize,
) -> Option<u16> {
    let tile_x = x / 8;
    let tile_y = y / 8;
    let pixel_x = x & 7;
    let pixel_y = y & 7;
    let tile_width = obj_width / 8;
    let slot_step = if bpp8 { 2 } else { 1 };
    let row_stride = if mapping_1d {
        tile_width * slot_step
    } else {
        32
    };
    let base_tile = if bpp8 { tile_id & !1 } else { tile_id };
    let slot = base_tile + tile_y * row_stride + tile_x * slot_step;
    let tile_offset = OBJ_TILE_BASE + slot * 32;

    if bpp8 {
        let palette_index = vram
            .get(tile_offset + pixel_y * 8 + pixel_x)
            .copied()
            .unwrap_or(0);
        if palette_index == 0 {
            None
        } else {
            Some(read_palette_color(
                palette,
                OBJ_PALETTE_BASE + usize::from(palette_index),
            ))
        }
    } else {
        let packed = vram
            .get(tile_offset + pixel_y * 4 + (pixel_x / 2))
            .copied()
            .unwrap_or(0);
        let palette_index = if (pixel_x & 1) == 0 {
            packed & 0x0f
        } else {
            packed >> 4
        };
        if palette_index == 0 {
            None
        } else {
            Some(read_palette_color(
                palette,
                OBJ_PALETTE_BASE + palette_bank * 16 + usize::from(palette_index),
            ))
        }
    }
}

fn read_u16(slice: &[u8], offset: usize) -> u16 {
    let lo = slice.get(offset).copied().unwrap_or(0) as u16;
    let hi = slice.get(offset + 1).copied().unwrap_or(0) as u16;
    lo | (hi << 8)
}

#[cfg(test)]
mod tests {
    use super::render_obj_layer;
    use crate::io::IoRegs;
    use crate::ppu::{compose::LayerPixel, SCREEN_WIDTH};

    const DISPCNT_ADDR: u32 = 0x0400_0000;
    const OBJ_ENABLED: u16 = 1 << 12;
    const OBJ_1D_MAP: u16 = 1 << 6;
    const OAM_BASE: usize = 0;
    const OBJ_TILE_BASE: usize = 0x1_0000;

    fn write_u16(slice: &mut [u8], offset: usize, value: u16) {
        slice[offset] = value as u8;
        slice[offset + 1] = (value >> 8) as u8;
    }

    #[test]
    fn regular_obj_fetches_expected_pixels_from_oam_and_vram() {
        let mut io = IoRegs::new();
        let mut vram = vec![0; 0x18000];
        let mut palette = vec![0; 0x400];
        let mut oam = vec![0; 0x400];
        let mut layer = [LayerPixel::transparent(4, u8::MAX); SCREEN_WIDTH];

        io.write_16(DISPCNT_ADDR, OBJ_ENABLED | OBJ_1D_MAP);
        write_u16(&mut palette, 0x200 + 2, 0x001f);
        write_u16(&mut palette, 0x200 + 4, 0x03e0);

        write_u16(&mut oam, OAM_BASE, 0x0000);
        write_u16(&mut oam, OAM_BASE + 2, 0x0000);
        write_u16(&mut oam, OAM_BASE + 4, 0x0000);

        vram[OBJ_TILE_BASE] = 0x21;
        vram[OBJ_TILE_BASE + 1] = 0x21;
        vram[OBJ_TILE_BASE + 2] = 0x21;
        vram[OBJ_TILE_BASE + 3] = 0x21;

        render_obj_layer(&mut layer, &io, &vram, &palette, &oam, 0);

        assert_eq!(layer[0], LayerPixel::opaque(0x001f, 0, 0));
        assert_eq!(layer[1], LayerPixel::opaque(0x03e0, 0, 0));
    }

    #[test]
    fn obj_palette_zero_is_transparent() {
        let mut io = IoRegs::new();
        let mut vram = vec![0; 0x18000];
        let palette = vec![0; 0x400];
        let mut oam = vec![0; 0x400];
        let mut layer = [LayerPixel::transparent(4, u8::MAX); SCREEN_WIDTH];

        io.write_16(DISPCNT_ADDR, OBJ_ENABLED | OBJ_1D_MAP);
        write_u16(&mut oam, OAM_BASE, 0x0000);
        write_u16(&mut oam, OAM_BASE + 2, 0x0000);
        write_u16(&mut oam, OAM_BASE + 4, 0x0000);
        vram[OBJ_TILE_BASE] = 0x00;

        render_obj_layer(&mut layer, &io, &vram, &palette, &oam, 0);

        assert!(layer[0].transparent);
    }

    #[test]
    fn obj_8bpp_uses_obj_palette_base() {
        let mut io = IoRegs::new();
        let mut vram = vec![0; 0x18000];
        let mut palette = vec![0; 0x400];
        let mut oam = vec![0; 0x400];
        let mut layer = [LayerPixel::transparent(4, u8::MAX); SCREEN_WIDTH];

        io.write_16(DISPCNT_ADDR, OBJ_ENABLED | OBJ_1D_MAP);
        write_u16(&mut palette, 0x200 + 2, 0x001f);
        write_u16(&mut oam, OAM_BASE, 0x2000);
        write_u16(&mut oam, OAM_BASE + 2, 0x0000);
        write_u16(&mut oam, OAM_BASE + 4, 0x0000);

        vram[OBJ_TILE_BASE] = 1;

        render_obj_layer(&mut layer, &io, &vram, &palette, &oam, 0);

        assert_eq!(layer[0], LayerPixel::opaque(0x001f, 0, 0));
    }

    #[test]
    fn obj_hflip_and_vflip_change_sampling() {
        let mut io = IoRegs::new();
        let mut vram = vec![0; 0x18000];
        let mut palette = vec![0; 0x400];
        let mut oam = vec![0; 0x400];
        let mut layer = [LayerPixel::transparent(4, u8::MAX); SCREEN_WIDTH];

        io.write_16(DISPCNT_ADDR, OBJ_ENABLED | OBJ_1D_MAP);
        write_u16(&mut palette, 0x200 + 2, 0x001f);
        write_u16(&mut palette, 0x200 + 4, 0x03e0);
        write_u16(&mut palette, 0x200 + 6, 0x7c00);
        write_u16(&mut palette, 0x200 + 8, 0x7fe0);

        write_u16(&mut oam, OAM_BASE, 0x0000);
        write_u16(&mut oam, OAM_BASE + 2, 0x3000);
        write_u16(&mut oam, OAM_BASE + 4, 0x0000);

        vram[OBJ_TILE_BASE + 28] = 0x03;
        vram[OBJ_TILE_BASE + 31] = 0x40;

        render_obj_layer(&mut layer, &io, &vram, &palette, &oam, 0);

        assert_eq!(layer[0].color, 0x7fe0);
        assert_eq!(layer[7].color, 0x7c00);
    }

    #[test]
    fn obj_1d_and_2d_mapping_select_different_tiles() {
        let mut io = IoRegs::new();
        let mut vram = vec![0; 0x18000];
        let mut palette = vec![0; 0x400];
        let mut oam = vec![0; 0x400];
        let mut layer = [LayerPixel::transparent(4, u8::MAX); SCREEN_WIDTH];

        write_u16(&mut palette, 0x200 + 2, 0x001f);
        write_u16(&mut palette, 0x200 + 4, 0x03e0);
        write_u16(&mut oam, OAM_BASE, 0x0000);
        write_u16(&mut oam, OAM_BASE + 2, 0x8000);
        write_u16(&mut oam, OAM_BASE + 4, 0x0000);

        vram[OBJ_TILE_BASE] = 0x11;
        vram[OBJ_TILE_BASE + 0x80] = 0x22;
        vram[OBJ_TILE_BASE + 0x400] = 0x11;

        io.write_16(DISPCNT_ADDR, OBJ_ENABLED | OBJ_1D_MAP);
        render_obj_layer(&mut layer, &io, &vram, &palette, &oam, 8);
        assert_eq!(layer[0].color, 0x03e0);

        layer.fill(LayerPixel::transparent(4, u8::MAX));
        io.write_16(DISPCNT_ADDR, OBJ_ENABLED);
        render_obj_layer(&mut layer, &io, &vram, &palette, &oam, 8);
        assert_eq!(layer[0].color, 0x001f);
    }
}
