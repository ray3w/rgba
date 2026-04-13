use crate::io::IoRegs;

use super::obj;
use super::SCREEN_WIDTH;

pub type WindowMaskLine = [WindowMask; SCREEN_WIDTH];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WindowMask {
    pub bg: [bool; 4],
    pub obj: bool,
    pub color_effect: bool,
}

impl WindowMask {
    pub const fn all_visible() -> Self {
        Self {
            bg: [true; 4],
            obj: true,
            color_effect: true,
        }
    }

    pub const fn from_bits(bits: u8) -> Self {
        Self {
            bg: [
                (bits & 0x01) != 0,
                (bits & 0x02) != 0,
                (bits & 0x04) != 0,
                (bits & 0x08) != 0,
            ],
            obj: (bits & 0x10) != 0,
            color_effect: (bits & 0x20) != 0,
        }
    }

    pub fn layer_visible(&self, layer_index: usize) -> bool {
        match layer_index {
            0..=3 => self.bg[layer_index],
            4 => self.obj,
            _ => true,
        }
    }
}

pub fn build_window_scanline(io: &IoRegs, vram: &[u8], oam: &[u8], y: usize) -> WindowMaskLine {
    let all_visible = WindowMask::all_visible();
    if !io.win0_enabled() && !io.win1_enabled() && !io.obj_window_enabled() {
        return [all_visible; SCREEN_WIDTH];
    }

    let outside = WindowMask::from_bits((io.win_out() & 0x00ff) as u8);
    let mut masks = [outside; SCREEN_WIDTH];

    if io.obj_window_enabled() {
        let mut obj_mask = [false; SCREEN_WIDTH];
        obj::render_obj_window_mask(&mut obj_mask, io, vram, oam, y);
        let obj_window = WindowMask::from_bits((io.win_out() >> 8) as u8);
        for x in 0..SCREEN_WIDTH {
            if obj_mask[x] {
                masks[x] = obj_window;
            }
        }
    }

    if io.win1_enabled() {
        apply_rect_window(
            &mut masks,
            io.window_h(1),
            io.window_v(1),
            y,
            WindowMask::from_bits((io.win_in() >> 8) as u8),
        );
    }

    if io.win0_enabled() {
        apply_rect_window(
            &mut masks,
            io.window_h(0),
            io.window_v(0),
            y,
            WindowMask::from_bits((io.win_in() & 0x00ff) as u8),
        );
    }

    masks
}

fn apply_rect_window(
    masks: &mut [WindowMask; SCREEN_WIDTH],
    horizontal: u16,
    vertical: u16,
    y: usize,
    replacement: WindowMask,
) {
    let left = ((horizontal >> 8) & 0x00ff) as usize;
    let right = (horizontal & 0x00ff) as usize;
    let top = ((vertical >> 8) & 0x00ff) as usize;
    let bottom = (vertical & 0x00ff) as usize;

    if !contains_coordinate(y, top, bottom, 160) {
        return;
    }

    for (x, mask) in masks.iter_mut().enumerate() {
        if contains_coordinate(x, left, right, SCREEN_WIDTH) {
            *mask = replacement;
        }
    }
}

fn contains_coordinate(value: usize, start: usize, end: usize, max: usize) -> bool {
    if start == end {
        return false;
    }

    if start < end {
        (start..end).contains(&value)
    } else {
        value >= start || value < end.min(max)
    }
}

#[cfg(test)]
mod tests {
    use super::{build_window_scanline, contains_coordinate, WindowMask};
    use crate::io::IoRegs;

    #[test]
    fn coordinate_ranges_support_wraparound() {
        assert!(contains_coordinate(250, 240, 16, 256));
        assert!(contains_coordinate(8, 240, 16, 256));
        assert!(!contains_coordinate(32, 240, 16, 256));
    }

    #[test]
    fn no_window_bits_means_everything_is_visible() {
        let io = IoRegs::new();
        let masks = build_window_scanline(&io, &[], &[], 0);
        assert_eq!(masks[0], WindowMask::all_visible());
        assert_eq!(masks[239], WindowMask::all_visible());
    }

    #[test]
    fn win0_replaces_outside_mask_inside_rect() {
        let mut io = IoRegs::new();
        io.write_16(0x0400_0000, 1 << 13);
        io.write_16(0x0400_0040, 0x1020);
        io.write_16(0x0400_0044, 0x0818);
        io.write_16(0x0400_0048, 0x0011);
        io.write_16(0x0400_004a, 0x0002);

        let masks = build_window_scanline(&io, &[], &[], 16);
        assert!(masks[0].bg[1]);
        assert!(!masks[0].bg[0]);
        assert!(masks[16].bg[0]);
        assert!(masks[16].obj);
        assert!(!masks[16].color_effect);
    }

    #[test]
    fn obj_window_uses_winout_upper_byte_mask() {
        let mut io = IoRegs::new();
        let mut vram = vec![0; 0x18000];
        let mut oam = vec![0; 0x400];

        io.write_16(0x0400_0000, (1 << 12) | (1 << 15) | (1 << 6));
        io.write_16(0x0400_004a, 0x2001);
        oam[0] = 0x00;
        oam[1] = 0x08;
        vram[0x1_0000] = 0x11;

        let masks = build_window_scanline(&io, &vram, &oam, 0);
        assert!(masks[0].color_effect);
        assert!(!masks[0].obj);
        assert!(!masks[0].bg[0]);
    }
}
