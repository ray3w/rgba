use crate::io::IoRegs;

#[cfg(test)]
use super::compose::compose_layers_scanline;
use super::compose::{bg_order, bg_target, clear_layer, LayerPixel, BG_LAYER_COUNT};
#[cfg(test)]
use super::{backdrop_color, FRAME_PIXELS};
use super::{read_palette_color, SCREEN_WIDTH};

const SCREEN_BLOCK_SIZE: usize = 0x0800;
const CHAR_BLOCK_SIZE: usize = 0x4000;

#[cfg(test)]
pub fn render_mode0_scanline(
    framebuffer: &mut [u16; FRAME_PIXELS],
    io: &IoRegs,
    vram: &[u8],
    palette: &[u8],
    y: usize,
) {
    let layers = render_mode0_layers(io, vram, palette, y);
    compose_layers_scanline(framebuffer, y, backdrop_color(palette), &layers);
}

#[cfg(test)]
pub fn render_mode1_scanline(
    framebuffer: &mut [u16; FRAME_PIXELS],
    io: &IoRegs,
    vram: &[u8],
    palette: &[u8],
    y: usize,
) {
    let layers = render_mode1_layers(io, vram, palette, y);
    compose_layers_scanline(framebuffer, y, backdrop_color(palette), &layers);
}

#[cfg(test)]
#[allow(dead_code)]
pub fn render_mode2_scanline(
    framebuffer: &mut [u16; FRAME_PIXELS],
    io: &IoRegs,
    vram: &[u8],
    palette: &[u8],
    y: usize,
) {
    let layers = render_mode2_layers(io, vram, palette, y);
    compose_layers_scanline(framebuffer, y, backdrop_color(palette), &layers);
}

pub fn render_mode0_layers(
    io: &IoRegs,
    vram: &[u8],
    palette: &[u8],
    y: usize,
) -> [[LayerPixel; SCREEN_WIDTH]; BG_LAYER_COUNT] {
    let mut layers = blank_layers(io);

    for (bg_index, layer) in layers.iter_mut().enumerate() {
        render_text_bg_layer(layer, io, vram, palette, bg_index, y);
    }

    layers
}

pub fn render_mode1_layers(
    io: &IoRegs,
    vram: &[u8],
    palette: &[u8],
    y: usize,
) -> [[LayerPixel; SCREEN_WIDTH]; BG_LAYER_COUNT] {
    let mut layers = blank_layers(io);

    render_text_bg_layer(&mut layers[0], io, vram, palette, 0, y);
    render_text_bg_layer(&mut layers[1], io, vram, palette, 1, y);
    render_affine_bg_layer(&mut layers[2], io, vram, palette, 2, y);

    layers
}

pub fn render_mode2_layers(
    io: &IoRegs,
    vram: &[u8],
    palette: &[u8],
    y: usize,
) -> [[LayerPixel; SCREEN_WIDTH]; BG_LAYER_COUNT] {
    let mut layers = blank_layers(io);

    render_affine_bg_layer(&mut layers[2], io, vram, palette, 2, y);
    render_affine_bg_layer(&mut layers[3], io, vram, palette, 3, y);

    layers
}

fn blank_layers(io: &IoRegs) -> [[LayerPixel; SCREEN_WIDTH]; BG_LAYER_COUNT] {
    let mut layers = [[LayerPixel::transparent(3, 3, super::compose::TARGET_BACKDROP);
        SCREEN_WIDTH]; BG_LAYER_COUNT];
    for (bg_index, layer) in layers.iter_mut().enumerate() {
        clear_layer(
            layer,
            io.bg_priority(bg_index),
            bg_order(bg_index),
            bg_target(bg_index),
        );
    }
    layers
}

fn render_text_bg_layer(
    layer: &mut [LayerPixel; SCREEN_WIDTH],
    io: &IoRegs,
    vram: &[u8],
    palette: &[u8],
    bg_index: usize,
    y: usize,
) {
    let priority = io.bg_priority(bg_index);
    let order = bg_order(bg_index);
    clear_layer(layer, priority, order, bg_target(bg_index));

    if !io.bg_enabled(bg_index) {
        return;
    }

    let hofs = usize::from(io.bg_hofs(bg_index));
    let vofs = usize::from(io.bg_vofs(bg_index));
    let world_y = y.wrapping_add(vofs);
    let (bg_width, bg_height) = text_bg_dimensions(io.bg_size(bg_index));
    let wrapped_y = if bg_height == 0 {
        0
    } else {
        world_y % bg_height
    };
    let palette_256 = io.bg_palette_256(bg_index);
    let char_base = io.bg_char_base_block(bg_index) * CHAR_BLOCK_SIZE;
    let screen_base = io.bg_screen_base_block(bg_index) * SCREEN_BLOCK_SIZE;

    for (x, pixel) in layer.iter_mut().enumerate() {
        let world_x = x.wrapping_add(hofs);
        let wrapped_x = if bg_width == 0 { 0 } else { world_x % bg_width };

        let screen_entry = read_screen_entry(
            vram,
            screen_base,
            io.bg_size(bg_index),
            wrapped_x,
            wrapped_y,
        );
        let tile_index = (screen_entry & 0x03ff) as usize;
        let hflip = (screen_entry & (1 << 10)) != 0;
        let vflip = (screen_entry & (1 << 11)) != 0;
        let palette_bank = ((screen_entry >> 12) & 0x000f) as usize;

        let mut tile_x = wrapped_x & 7;
        let mut tile_y = wrapped_y & 7;
        if hflip {
            tile_x = 7 - tile_x;
        }
        if vflip {
            tile_y = 7 - tile_y;
        }

        let color = if palette_256 {
            fetch_8bpp_tile_color(vram, palette, char_base, tile_index, tile_x, tile_y)
        } else {
            fetch_4bpp_tile_color(
                vram,
                palette,
                char_base,
                tile_index,
                tile_x,
                tile_y,
                palette_bank,
            )
        };

        if let Some(color) = color {
            *pixel = LayerPixel::opaque(color, priority, order, bg_target(bg_index));
        }
    }
}

fn render_affine_bg_layer(
    layer: &mut [LayerPixel; SCREEN_WIDTH],
    io: &IoRegs,
    vram: &[u8],
    palette: &[u8],
    bg_index: usize,
    y: usize,
) {
    let priority = io.bg_priority(bg_index);
    let order = bg_order(bg_index);
    clear_layer(layer, priority, order, bg_target(bg_index));

    if !io.bg_enabled(bg_index) {
        return;
    }

    let Some((pa, pb, pc, pd)) = io.bg_affine_matrix(bg_index) else {
        return;
    };
    let Some((ref_x, ref_y)) = io.bg_affine_ref_point(bg_index) else {
        return;
    };

    let dimension = affine_bg_dimension(io.bg_size(bg_index));
    let wrap = io.bg_wrap(bg_index);
    let char_base = io.bg_char_base_block(bg_index) * CHAR_BLOCK_SIZE;
    let screen_base = io.bg_screen_base_block(bg_index) * SCREEN_BLOCK_SIZE;
    let y = y as i32;

    for (x, pixel) in layer.iter_mut().enumerate() {
        let x = x as i32;
        let tex_x = (ref_x + i32::from(pa) * x + i32::from(pb) * y) >> 8;
        let tex_y = (ref_y + i32::from(pc) * x + i32::from(pd) * y) >> 8;

        let Some((tex_x, tex_y)) = normalize_affine_coords(tex_x, tex_y, dimension, wrap) else {
            continue;
        };

        let tile_index = read_affine_screen_entry(vram, screen_base, dimension, tex_x, tex_y);
        let tile_x = tex_x & 7;
        let tile_y = tex_y & 7;

        if let Some(color) =
            fetch_8bpp_tile_color(vram, palette, char_base, tile_index, tile_x, tile_y)
        {
            *pixel = LayerPixel::opaque(color, priority, order, bg_target(bg_index));
        }
    }
}

fn text_bg_dimensions(size: u8) -> (usize, usize) {
    match size & 0x03 {
        0 => (256, 256),
        1 => (512, 256),
        2 => (256, 512),
        _ => (512, 512),
    }
}

fn affine_bg_dimension(size: u8) -> usize {
    match size & 0x03 {
        0 => 128,
        1 => 256,
        2 => 512,
        _ => 1024,
    }
}

fn read_screen_entry(vram: &[u8], screen_base: usize, size: u8, x: usize, y: usize) -> u16 {
    let tile_x = x / 8;
    let tile_y = y / 8;

    let (blocks_wide, block_x, block_y) = match size & 0x03 {
        0 => (1usize, 0usize, 0usize),
        1 => (2usize, tile_x / 32, 0usize),
        2 => (1usize, 0usize, tile_y / 32),
        _ => (2usize, tile_x / 32, tile_y / 32),
    };

    let local_x = tile_x % 32;
    let local_y = tile_y % 32;
    let screen_block = block_y * blocks_wide + block_x;
    let entry_offset =
        screen_base + screen_block * SCREEN_BLOCK_SIZE + (local_y * 32 + local_x) * 2;

    read_u16(vram, entry_offset)
}

fn read_affine_screen_entry(
    vram: &[u8],
    screen_base: usize,
    dimension: usize,
    x: usize,
    y: usize,
) -> usize {
    let map_width = dimension / 8;
    let tile_x = x / 8;
    let tile_y = y / 8;
    let entry_offset = screen_base + tile_y * map_width + tile_x;
    usize::from(vram.get(entry_offset).copied().unwrap_or(0))
}

fn normalize_affine_coords(x: i32, y: i32, dimension: usize, wrap: bool) -> Option<(usize, usize)> {
    let dimension = i32::try_from(dimension).ok()?;
    if wrap {
        Some((
            x.rem_euclid(dimension) as usize,
            y.rem_euclid(dimension) as usize,
        ))
    } else if (0..dimension).contains(&x) && (0..dimension).contains(&y) {
        Some((x as usize, y as usize))
    } else {
        None
    }
}

fn fetch_4bpp_tile_color(
    vram: &[u8],
    palette: &[u8],
    char_base: usize,
    tile_index: usize,
    tile_x: usize,
    tile_y: usize,
    palette_bank: usize,
) -> Option<u16> {
    let tile_offset = char_base + tile_index * 32 + tile_y * 4 + (tile_x / 2);
    let packed = vram.get(tile_offset).copied().unwrap_or(0);
    let palette_index = if (tile_x & 1) == 0 {
        packed & 0x0f
    } else {
        packed >> 4
    };

    if palette_index == 0 {
        None
    } else {
        Some(read_palette_color(
            palette,
            palette_bank * 16 + usize::from(palette_index),
        ))
    }
}

fn fetch_8bpp_tile_color(
    vram: &[u8],
    palette: &[u8],
    char_base: usize,
    tile_index: usize,
    tile_x: usize,
    tile_y: usize,
) -> Option<u16> {
    let tile_offset = char_base + tile_index * 64 + tile_y * 8 + tile_x;
    let palette_index = vram.get(tile_offset).copied().unwrap_or(0);

    if palette_index == 0 {
        None
    } else {
        Some(read_palette_color(palette, usize::from(palette_index)))
    }
}

fn read_u16(slice: &[u8], offset: usize) -> u16 {
    let lo = slice.get(offset).copied().unwrap_or(0) as u16;
    let hi = slice.get(offset + 1).copied().unwrap_or(0) as u16;
    lo | (hi << 8)
}

#[cfg(test)]
mod tests {
    use super::{render_mode0_scanline, render_mode1_scanline, FRAME_PIXELS};
    use crate::io::IoRegs;

    const DISPCNT_ADDR: u32 = 0x0400_0000;
    const BG0CNT_ADDR: u32 = 0x0400_0008;
    const BG1CNT_ADDR: u32 = 0x0400_000a;
    const BG2CNT_ADDR: u32 = 0x0400_000c;
    const BG0HOFS_ADDR: u32 = 0x0400_0010;
    const BG2PA_ADDR: u32 = 0x0400_0020;
    const BG2PB_ADDR: u32 = 0x0400_0022;
    const BG2PC_ADDR: u32 = 0x0400_0024;
    const BG2PD_ADDR: u32 = 0x0400_0026;

    fn write_u16(slice: &mut [u8], offset: usize, value: u16) {
        slice[offset] = value as u8;
        slice[offset + 1] = (value >> 8) as u8;
    }

    #[test]
    fn mode0_multiple_backgrounds_compose_by_priority() {
        let mut framebuffer = Box::new([0; FRAME_PIXELS]);
        let mut io = IoRegs::new();
        let mut vram = vec![0; 0x18000];
        let mut palette = vec![0; 0x400];

        io.write_16(DISPCNT_ADDR, 0x0300);
        io.write_16(BG0CNT_ADDR, 0x0000);
        io.write_16(BG1CNT_ADDR, 0x0105);

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

        render_mode0_scanline(&mut framebuffer, &io, &vram, &palette, 0);

        assert_eq!(framebuffer[0], 0x03e0);
        assert_eq!(framebuffer[1], 0x03e0);
    }

    #[test]
    fn mode0_scroll_changes_visible_tile_sample() {
        let mut framebuffer = Box::new([0; FRAME_PIXELS]);
        let mut io = IoRegs::new();
        let mut vram = vec![0; 0x18000];
        let mut palette = vec![0; 0x400];

        io.write_16(DISPCNT_ADDR, 0x0100);
        io.write_16(BG0CNT_ADDR, 0x0000);
        io.write_16(BG0HOFS_ADDR, 8);

        write_u16(&mut palette, 2, 0x001f);
        write_u16(&mut palette, 4, 0x03e0);

        vram[0x0000] = 0x11;
        vram[0x0001] = 0x11;
        vram[0x0002] = 0x11;
        vram[0x0003] = 0x11;
        vram[0x0020] = 0x22;
        vram[0x0021] = 0x22;
        vram[0x0022] = 0x22;
        vram[0x0023] = 0x22;
        write_u16(&mut vram, 0x0000, 0x0000);
        write_u16(&mut vram, 0x0002, 0x0001);

        render_mode0_scanline(&mut framebuffer, &io, &vram, &palette, 0);

        assert_eq!(framebuffer[0], 0x03e0);
    }

    #[test]
    fn mode1_affine_bg2_identity_transform_fetches_expected_pixels() {
        let mut framebuffer = Box::new([0; FRAME_PIXELS]);
        let mut io = IoRegs::new();
        let mut vram = vec![0; 0x18000];
        let mut palette = vec![0; 0x400];

        io.write_16(DISPCNT_ADDR, 0x0401);
        io.write_16(BG2CNT_ADDR, 0x0104);
        io.write_16(BG2PA_ADDR, 0x0100);
        io.write_16(BG2PB_ADDR, 0x0000);
        io.write_16(BG2PC_ADDR, 0x0000);
        io.write_16(BG2PD_ADDR, 0x0100);

        write_u16(&mut palette, 2, 0x001f);
        write_u16(&mut palette, 4, 0x03e0);
        vram[0x4000] = 1;
        vram[0x4001] = 2;
        vram[0x0800] = 0;

        render_mode1_scanline(&mut framebuffer, &io, &vram, &palette, 0);

        assert_eq!(framebuffer[0], 0x001f);
        assert_eq!(framebuffer[1], 0x03e0);
    }

    #[test]
    fn mode1_affine_layer_out_of_range_falls_back_to_backdrop_without_wrap() {
        let mut framebuffer = Box::new([0; FRAME_PIXELS]);
        let mut io = IoRegs::new();
        let mut vram = vec![0; 0x18000];
        let mut palette = vec![0; 0x400];

        io.write_16(DISPCNT_ADDR, 0x0401);
        io.write_16(BG2CNT_ADDR, 0x0104);
        io.write_16(BG2PA_ADDR, 0x0100);
        io.write_16(BG2PD_ADDR, 0x0100);
        io.write_32(0x0400_0028, 128 << 8);

        write_u16(&mut palette, 0, 0x7c00);
        vram[0x4000] = 1;
        write_u16(&mut palette, 2, 0x001f);
        vram[0x0800] = 0;

        render_mode1_scanline(&mut framebuffer, &io, &vram, &palette, 0);

        assert_eq!(framebuffer[0], 0x7c00);
    }
}
