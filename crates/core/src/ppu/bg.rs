use crate::io::IoRegs;

use super::{backdrop_color, read_palette_color, FRAME_PIXELS, SCREEN_WIDTH};

const SCREEN_BLOCK_SIZE: usize = 0x0800;
const CHAR_BLOCK_SIZE: usize = 0x4000;

/// Render the first text background layer used by the early Mode 0 path.
pub fn render_mode0_bg0_scanline(
    framebuffer: &mut [u16; FRAME_PIXELS],
    io: &IoRegs,
    vram: &[u8],
    palette: &[u8],
    y: usize,
) {
    let line_start = y * SCREEN_WIDTH;
    let backdrop = backdrop_color(palette);
    framebuffer[line_start..line_start + SCREEN_WIDTH].fill(backdrop);

    if !io.bg_enabled(0) {
        return;
    }

    let hofs = usize::from(io.bg_hofs(0));
    let vofs = usize::from(io.bg_vofs(0));
    let world_y = y.wrapping_add(vofs);
    let (bg_width, bg_height) = text_bg_dimensions(io.bg_size(0));
    let wrapped_y = if bg_height == 0 {
        0
    } else {
        world_y % bg_height
    };
    let palette_256 = io.bg_palette_256(0);
    let char_base = io.bg_char_base_block(0) * CHAR_BLOCK_SIZE;
    let screen_base = io.bg_screen_base_block(0) * SCREEN_BLOCK_SIZE;

    for x in 0..SCREEN_WIDTH {
        let world_x = x.wrapping_add(hofs);
        let wrapped_x = if bg_width == 0 { 0 } else { world_x % bg_width };

        let screen_entry =
            read_screen_entry(vram, screen_base, io.bg_size(0), wrapped_x, wrapped_y);
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
            fetch_8bpp_tile_color(
                vram, palette, char_base, tile_index, tile_x, tile_y, backdrop,
            )
        } else {
            fetch_4bpp_tile_color(
                vram,
                palette,
                char_base,
                tile_index,
                tile_x,
                tile_y,
                palette_bank,
                backdrop,
            )
        };

        framebuffer[line_start + x] = color;
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

fn fetch_4bpp_tile_color(
    vram: &[u8],
    palette: &[u8],
    char_base: usize,
    tile_index: usize,
    tile_x: usize,
    tile_y: usize,
    palette_bank: usize,
    backdrop: u16,
) -> u16 {
    let tile_offset = char_base + tile_index * 32 + tile_y * 4 + (tile_x / 2);
    let packed = vram.get(tile_offset).copied().unwrap_or(0);
    let palette_index = if (tile_x & 1) == 0 {
        packed & 0x0f
    } else {
        packed >> 4
    };

    if palette_index == 0 {
        backdrop
    } else {
        read_palette_color(palette, palette_bank * 16 + usize::from(palette_index))
    }
}

fn fetch_8bpp_tile_color(
    vram: &[u8],
    palette: &[u8],
    char_base: usize,
    tile_index: usize,
    tile_x: usize,
    tile_y: usize,
    backdrop: u16,
) -> u16 {
    let tile_offset = char_base + tile_index * 64 + tile_y * 8 + tile_x;
    let palette_index = vram.get(tile_offset).copied().unwrap_or(0);

    if palette_index == 0 {
        backdrop
    } else {
        read_palette_color(palette, usize::from(palette_index))
    }
}

fn read_u16(slice: &[u8], offset: usize) -> u16 {
    let lo = slice.get(offset).copied().unwrap_or(0) as u16;
    let hi = slice.get(offset + 1).copied().unwrap_or(0) as u16;
    lo | (hi << 8)
}
