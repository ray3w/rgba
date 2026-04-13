const DISPCNT_ADDR: u32 = 0x0400_0000;
const DISPSTAT_ADDR: u32 = 0x0400_0004;
const VCOUNT_ADDR: u32 = 0x0400_0006;

const BG0CNT_ADDR: u32 = 0x0400_0008;
const BG0HOFS_ADDR: u32 = 0x0400_0010;
const BG2PA_ADDR: u32 = 0x0400_0020;
const BG2X_ADDR: u32 = 0x0400_0028;

const DMA0SAD_ADDR: u32 = 0x0400_00b0;
const DMA0CNT_H_ADDR: u32 = 0x0400_00ba;

const TM0CNT_L_ADDR: u32 = 0x0400_0100;
const TM0CNT_H_ADDR: u32 = 0x0400_0102;

const KEYINPUT_ADDR: u32 = 0x0400_0130;
const KEYCNT_ADDR: u32 = 0x0400_0132;

const IE_ADDR: u32 = 0x0400_0200;
const IF_ADDR: u32 = 0x0400_0202;
const WAITCNT_ADDR: u32 = 0x0400_0204;
const IME_ADDR: u32 = 0x0400_0208;

const DISPSTAT_VBLANK: u16 = 1 << 0;
const DISPSTAT_HBLANK: u16 = 1 << 1;
const DISPSTAT_VCOUNT_MATCH: u16 = 1 << 2;
const DISPSTAT_VBLANK_IRQ_ENABLE: u16 = 1 << 3;
const DISPSTAT_HBLANK_IRQ_ENABLE: u16 = 1 << 4;
const DISPSTAT_VCOUNT_IRQ_ENABLE: u16 = 1 << 5;

pub const IRQ_VBLANK: u16 = 1 << 0;
pub const IRQ_HBLANK: u16 = 1 << 1;
pub const IRQ_VCOUNT: u16 = 1 << 2;
pub const IRQ_TIMER0: u16 = 1 << 3;
pub const IRQ_TIMER1: u16 = 1 << 4;
pub const IRQ_TIMER2: u16 = 1 << 5;
pub const IRQ_TIMER3: u16 = 1 << 6;
pub const IRQ_DMA0: u16 = 1 << 8;
pub const IRQ_DMA1: u16 = 1 << 9;
pub const IRQ_DMA2: u16 = 1 << 10;
pub const IRQ_DMA3: u16 = 1 << 11;
pub const IRQ_KEYPAD: u16 = 1 << 12;

/// Minimal MMIO register block used by the early core phases.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IoRegs {
    ime: u16,
    ie: u16,
    if_: u16,
    waitcnt: u16,
    dispcnt: u16,
    dispstat: u16,
    vcount: u16,
    bg_control: [u16; 4],
    bg_hofs: [u16; 4],
    bg_vofs: [u16; 4],
    bg_affine_pa: [i16; 2],
    bg_affine_pb: [i16; 2],
    bg_affine_pc: [i16; 2],
    bg_affine_pd: [i16; 2],
    bg_ref_x: [i32; 2],
    bg_ref_y: [i32; 2],
    keyinput: u16,
    keycnt: u16,
    timer_reload: [u16; 4],
    timer_counter: [u16; 4],
    timer_control: [u16; 4],
    dma_source: [u32; 4],
    dma_dest: [u32; 4],
    dma_count: [u16; 4],
    dma_control: [u16; 4],
}

impl Default for IoRegs {
    fn default() -> Self {
        Self::new()
    }
}

impl IoRegs {
    pub fn new() -> Self {
        Self {
            ime: 0,
            ie: 0,
            if_: 0,
            waitcnt: 0,
            dispcnt: 0,
            dispstat: 0,
            vcount: 0,
            bg_control: [0; 4],
            bg_hofs: [0; 4],
            bg_vofs: [0; 4],
            bg_affine_pa: [0; 2],
            bg_affine_pb: [0; 2],
            bg_affine_pc: [0; 2],
            bg_affine_pd: [0; 2],
            bg_ref_x: [0; 2],
            bg_ref_y: [0; 2],
            keyinput: 0x03ff,
            keycnt: 0,
            timer_reload: [0; 4],
            timer_counter: [0; 4],
            timer_control: [0; 4],
            dma_source: [0; 4],
            dma_dest: [0; 4],
            dma_count: [0; 4],
            dma_control: [0; 4],
        }
    }

    pub fn ime(&self) -> u16 {
        self.ime
    }

    pub fn ie(&self) -> u16 {
        self.ie
    }

    pub fn if_(&self) -> u16 {
        self.if_
    }

    pub fn irq_pending_mask(&self) -> u16 {
        self.ie & self.if_
    }

    pub fn waitcnt(&self) -> u16 {
        self.waitcnt
    }

    pub fn dispcnt(&self) -> u16 {
        self.dispcnt
    }

    pub fn dispstat(&self) -> u16 {
        self.dispstat
    }

    pub fn vcount(&self) -> u16 {
        self.vcount
    }

    pub fn keyinput(&self) -> u16 {
        self.keyinput
    }

    pub fn keycnt(&self) -> u16 {
        self.keycnt
    }

    pub fn display_mode(&self) -> u16 {
        self.dispcnt & 0x0007
    }

    pub fn display_frame_select(&self) -> bool {
        (self.dispcnt & (1 << 4)) != 0
    }

    pub fn bg_enabled(&self, index: usize) -> bool {
        index < 4 && (self.dispcnt & (1 << (8 + index))) != 0
    }

    pub fn bg2_enabled(&self) -> bool {
        self.bg_enabled(2)
    }

    pub fn bg_control(&self, index: usize) -> u16 {
        self.bg_control[index]
    }

    pub fn bg_hofs(&self, index: usize) -> u16 {
        self.bg_hofs[index]
    }

    pub fn bg_vofs(&self, index: usize) -> u16 {
        self.bg_vofs[index]
    }

    pub fn bg_priority(&self, index: usize) -> u8 {
        (self.bg_control[index] & 0x0003) as u8
    }

    pub fn bg_char_base_block(&self, index: usize) -> usize {
        ((self.bg_control[index] >> 2) & 0x0003) as usize
    }

    pub fn bg_palette_256(&self, index: usize) -> bool {
        (self.bg_control[index] & (1 << 7)) != 0
    }

    pub fn bg_screen_base_block(&self, index: usize) -> usize {
        ((self.bg_control[index] >> 8) & 0x001f) as usize
    }

    pub fn bg_wrap(&self, index: usize) -> bool {
        (self.bg_control[index] & (1 << 13)) != 0
    }

    pub fn bg_size(&self, index: usize) -> u8 {
        ((self.bg_control[index] >> 14) & 0x0003) as u8
    }

    pub fn bg_affine_matrix(&self, index: usize) -> Option<(i16, i16, i16, i16)> {
        let affine = affine_index(index)?;
        Some((
            self.bg_affine_pa[affine],
            self.bg_affine_pb[affine],
            self.bg_affine_pc[affine],
            self.bg_affine_pd[affine],
        ))
    }

    pub fn bg_affine_ref_point(&self, index: usize) -> Option<(i32, i32)> {
        let affine = affine_index(index)?;
        Some((self.bg_ref_x[affine], self.bg_ref_y[affine]))
    }

    pub fn ime_enabled(&self) -> bool {
        (self.ime & 1) != 0
    }

    pub fn set_keyinput(&mut self, value: u16) {
        self.keyinput = value & 0x03ff;
    }

    pub fn timer_reload(&self, index: usize) -> u16 {
        self.timer_reload[index]
    }

    pub fn timer_counter(&self, index: usize) -> u16 {
        self.timer_counter[index]
    }

    pub fn timer_control(&self, index: usize) -> u16 {
        self.timer_control[index]
    }

    pub fn set_timer_counter(&mut self, index: usize, value: u16) {
        self.timer_counter[index] = value;
    }

    pub fn set_vcount(&mut self, value: u16) {
        let compare = (self.dispstat >> 8) & 0x00ff;
        let had_match = (self.dispstat & DISPSTAT_VCOUNT_MATCH) != 0;

        self.vcount = value & 0x00ff;

        if self.vcount == compare {
            self.dispstat |= DISPSTAT_VCOUNT_MATCH;
            if !had_match && (self.dispstat & DISPSTAT_VCOUNT_IRQ_ENABLE) != 0 {
                self.request_interrupt(IRQ_VCOUNT);
            }
        } else {
            self.dispstat &= !DISPSTAT_VCOUNT_MATCH;
        }
    }

    pub fn request_interrupt(&mut self, mask: u16) {
        self.if_ |= mask;
    }

    pub fn set_vblank(&mut self, active: bool) {
        if active {
            self.dispstat |= DISPSTAT_VBLANK;
            if (self.dispstat & DISPSTAT_VBLANK_IRQ_ENABLE) != 0 {
                self.request_interrupt(IRQ_VBLANK);
            }
        } else {
            self.dispstat &= !DISPSTAT_VBLANK;
        }
    }

    pub fn set_hblank(&mut self, active: bool) {
        if active {
            self.dispstat |= DISPSTAT_HBLANK;
            if (self.dispstat & DISPSTAT_HBLANK_IRQ_ENABLE) != 0 {
                self.request_interrupt(IRQ_HBLANK);
            }
        } else {
            self.dispstat &= !DISPSTAT_HBLANK;
        }
    }

    pub fn dma_source(&self, index: usize) -> u32 {
        self.dma_source[index]
    }

    pub fn dma_dest(&self, index: usize) -> u32 {
        self.dma_dest[index]
    }

    pub fn dma_count(&self, index: usize) -> u16 {
        self.dma_count[index]
    }

    pub fn dma_control(&self, index: usize) -> u16 {
        self.dma_control[index]
    }

    pub fn set_dma_source(&mut self, index: usize, value: u32) {
        self.dma_source[index] = value;
    }

    pub fn set_dma_dest(&mut self, index: usize, value: u32) {
        self.dma_dest[index] = value;
    }

    pub fn set_dma_count(&mut self, index: usize, value: u16) {
        self.dma_count[index] = value;
    }

    pub fn set_dma_control(&mut self, index: usize, value: u16) {
        self.dma_control[index] = value & 0xf7e0;
    }

    pub fn read_8(&self, addr: u32) -> u8 {
        let half = self.read_16(addr & !1);
        if (addr & 1) == 0 {
            half as u8
        } else {
            (half >> 8) as u8
        }
    }

    pub fn read_16(&self, addr: u32) -> u16 {
        let aligned = addr & !1;
        match aligned {
            DISPCNT_ADDR => self.dispcnt,
            DISPSTAT_ADDR => self.dispstat,
            VCOUNT_ADDR => self.vcount,
            KEYINPUT_ADDR => self.keyinput,
            KEYCNT_ADDR => self.keycnt,
            IE_ADDR => self.ie,
            IF_ADDR => self.if_,
            WAITCNT_ADDR => self.waitcnt,
            IME_ADDR => self.ime,
            _ => {
                if let Some(index) = bg_control_reg(aligned) {
                    self.bg_control[index]
                } else if let Some((index, vertical)) = bg_scroll_reg(aligned) {
                    if vertical {
                        self.bg_vofs[index]
                    } else {
                        self.bg_hofs[index]
                    }
                } else if let Some((index, param)) = affine_param_reg(aligned) {
                    match param {
                        AffineParam::Pa => self.bg_affine_pa[index] as u16,
                        AffineParam::Pb => self.bg_affine_pb[index] as u16,
                        AffineParam::Pc => self.bg_affine_pc[index] as u16,
                        AffineParam::Pd => self.bg_affine_pd[index] as u16,
                    }
                } else if let Some((index, axis, high)) = affine_ref_reg(aligned) {
                    let raw = if axis == AffineAxis::X {
                        self.bg_ref_x[index] as u32
                    } else {
                        self.bg_ref_y[index] as u32
                    };
                    if high {
                        (raw >> 16) as u16
                    } else {
                        raw as u16
                    }
                } else if let Some((index, high)) = timer_reg(aligned) {
                    if high {
                        self.timer_control[index]
                    } else {
                        self.timer_counter[index]
                    }
                } else if let Some((index, part)) = dma_reg(aligned) {
                    match part {
                        DmaRegPart::SourceLo => self.dma_source[index] as u16,
                        DmaRegPart::SourceHi => (self.dma_source[index] >> 16) as u16,
                        DmaRegPart::DestLo => self.dma_dest[index] as u16,
                        DmaRegPart::DestHi => (self.dma_dest[index] >> 16) as u16,
                        DmaRegPart::Count => self.dma_count[index],
                        DmaRegPart::Control => self.dma_control[index],
                    }
                } else {
                    0
                }
            }
        }
    }

    pub fn read_32(&self, addr: u32) -> u32 {
        let lo = u32::from(self.read_16(addr & !3));
        let hi = u32::from(self.read_16((addr & !3).wrapping_add(2)));
        lo | (hi << 16)
    }

    pub fn write_8(&mut self, addr: u32, val: u8) {
        let aligned = addr & !1;
        let old = self.read_16(aligned);
        let value = if (addr & 1) == 0 {
            (old & 0xff00) | u16::from(val)
        } else {
            (old & 0x00ff) | (u16::from(val) << 8)
        };
        self.write_16(aligned, value);
    }

    pub fn write_16(&mut self, addr: u32, val: u16) {
        let aligned = addr & !1;
        match aligned {
            DISPCNT_ADDR => self.dispcnt = val,
            DISPSTAT_ADDR => {
                let status = self.dispstat & 0x0007;
                self.dispstat = status | (val & !0x0007);
                self.set_vcount(self.vcount);
            }
            VCOUNT_ADDR => {}
            KEYINPUT_ADDR => {}
            KEYCNT_ADDR => self.keycnt = val & 0xc3ff,
            IE_ADDR => self.ie = val,
            IF_ADDR => self.if_ &= !val,
            WAITCNT_ADDR => self.waitcnt = val,
            IME_ADDR => self.ime = val & 1,
            _ => {
                if let Some(index) = bg_control_reg(aligned) {
                    self.bg_control[index] = val;
                } else if let Some((index, vertical)) = bg_scroll_reg(aligned) {
                    if vertical {
                        self.bg_vofs[index] = val & 0x01ff;
                    } else {
                        self.bg_hofs[index] = val & 0x01ff;
                    }
                } else if let Some((index, param)) = affine_param_reg(aligned) {
                    let value = val as i16;
                    match param {
                        AffineParam::Pa => self.bg_affine_pa[index] = value,
                        AffineParam::Pb => self.bg_affine_pb[index] = value,
                        AffineParam::Pc => self.bg_affine_pc[index] = value,
                        AffineParam::Pd => self.bg_affine_pd[index] = value,
                    }
                } else if let Some((index, axis, high)) = affine_ref_reg(aligned) {
                    let current = if axis == AffineAxis::X {
                        self.bg_ref_x[index] as u32
                    } else {
                        self.bg_ref_y[index] as u32
                    };
                    let combined = if high {
                        (current & 0x0000_ffff) | (u32::from(val) << 16)
                    } else {
                        (current & 0xffff_0000) | u32::from(val)
                    };
                    let normalized = normalize_bg_ref(combined);
                    if axis == AffineAxis::X {
                        self.bg_ref_x[index] = normalized;
                    } else {
                        self.bg_ref_y[index] = normalized;
                    }
                } else if let Some((index, high)) = timer_reg(aligned) {
                    if high {
                        self.timer_control[index] = val & 0x00c7;
                    } else {
                        self.timer_reload[index] = val;
                    }
                } else if let Some((index, part)) = dma_reg(aligned) {
                    match part {
                        DmaRegPart::SourceLo => {
                            self.dma_source[index] =
                                (self.dma_source[index] & 0xffff_0000) | u32::from(val)
                        }
                        DmaRegPart::SourceHi => {
                            self.dma_source[index] =
                                (self.dma_source[index] & 0x0000_ffff) | (u32::from(val) << 16)
                        }
                        DmaRegPart::DestLo => {
                            self.dma_dest[index] =
                                (self.dma_dest[index] & 0xffff_0000) | u32::from(val)
                        }
                        DmaRegPart::DestHi => {
                            self.dma_dest[index] =
                                (self.dma_dest[index] & 0x0000_ffff) | (u32::from(val) << 16)
                        }
                        DmaRegPart::Count => self.dma_count[index] = val,
                        DmaRegPart::Control => self.set_dma_control(index, val),
                    }
                }
            }
        }
    }

    pub fn write_32(&mut self, addr: u32, val: u32) {
        self.write_16(addr & !3, val as u16);
        self.write_16((addr & !3).wrapping_add(2), (val >> 16) as u16);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DmaRegPart {
    SourceLo,
    SourceHi,
    DestLo,
    DestHi,
    Count,
    Control,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AffineParam {
    Pa,
    Pb,
    Pc,
    Pd,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AffineAxis {
    X,
    Y,
}

fn affine_index(bg_index: usize) -> Option<usize> {
    bg_index.checked_sub(2).filter(|&index| index < 2)
}

fn bg_control_reg(addr: u32) -> Option<usize> {
    if !(BG0CNT_ADDR..=BG0CNT_ADDR + 6).contains(&addr) {
        return None;
    }
    let offset = addr - BG0CNT_ADDR;
    if (offset & 1) != 0 {
        return None;
    }
    Some((offset / 2) as usize)
}

fn bg_scroll_reg(addr: u32) -> Option<(usize, bool)> {
    if !(BG0HOFS_ADDR..=BG0HOFS_ADDR + 0x0e).contains(&addr) {
        return None;
    }

    let offset = addr - BG0HOFS_ADDR;
    match offset & 0x0003 {
        0 => Some(((offset / 4) as usize, false)),
        2 => Some(((offset / 4) as usize, true)),
        _ => None,
    }
}

fn affine_param_reg(addr: u32) -> Option<(usize, AffineParam)> {
    if (BG2PA_ADDR..=BG2PA_ADDR + 0x16).contains(&addr) {
        let offset = addr - BG2PA_ADDR;
        let index = (offset / 0x10) as usize;
        let param = match offset & 0x000e {
            0x0000 => AffineParam::Pa,
            0x0002 => AffineParam::Pb,
            0x0004 => AffineParam::Pc,
            0x0006 => AffineParam::Pd,
            _ => return None,
        };
        Some((index, param))
    } else {
        None
    }
}

fn affine_ref_reg(addr: u32) -> Option<(usize, AffineAxis, bool)> {
    if (BG2X_ADDR..=BG2X_ADDR + 0x16).contains(&addr) {
        let offset = addr - BG2X_ADDR;
        let index = (offset / 0x10) as usize;
        let high = (offset & 0x0002) != 0;
        let axis = match offset & 0x000c {
            0x0000 => AffineAxis::X,
            0x0004 => AffineAxis::Y,
            _ => return None,
        };
        Some((index, axis, high))
    } else {
        None
    }
}

fn normalize_bg_ref(raw: u32) -> i32 {
    ((raw << 4) as i32) >> 4
}

fn timer_reg(addr: u32) -> Option<(usize, bool)> {
    if !(TM0CNT_L_ADDR..=TM0CNT_H_ADDR + 12).contains(&addr) {
        return None;
    }

    let offset = addr - TM0CNT_L_ADDR;
    if offset % 4 > 2 {
        return None;
    }

    Some(((offset / 4) as usize, (offset % 4) == 2))
}

fn dma_reg(addr: u32) -> Option<(usize, DmaRegPart)> {
    const DMA_BASES: [u32; 4] = [
        DMA0SAD_ADDR,
        DMA0SAD_ADDR + 0x0c,
        DMA0SAD_ADDR + 0x18,
        DMA0SAD_ADDR + 0x24,
    ];

    for (index, base) in DMA_BASES.into_iter().enumerate() {
        let range_end = base + (DMA0CNT_H_ADDR - DMA0SAD_ADDR);
        if !(base..=range_end).contains(&addr) {
            continue;
        }

        let offset = addr - base;
        let part = match offset {
            0x00 => DmaRegPart::SourceLo,
            0x02 => DmaRegPart::SourceHi,
            0x04 => DmaRegPart::DestLo,
            0x06 => DmaRegPart::DestHi,
            0x08 => DmaRegPart::Count,
            0x0a => DmaRegPart::Control,
            _ => continue,
        };
        return Some((index, part));
    }

    None
}

#[cfg(test)]
mod tests {
    use super::{
        IoRegs, DISPSTAT_ADDR, DMA0CNT_H_ADDR, DMA0SAD_ADDR, IE_ADDR, IF_ADDR, IME_ADDR,
        IRQ_KEYPAD, IRQ_VBLANK, IRQ_VCOUNT, KEYCNT_ADDR, KEYINPUT_ADDR, TM0CNT_H_ADDR,
        TM0CNT_L_ADDR, VCOUNT_ADDR, WAITCNT_ADDR,
    };

    const BG0CNT_ADDR: u32 = 0x0400_0008;
    const BG0HOFS_ADDR: u32 = 0x0400_0010;
    const BG0VOFS_ADDR: u32 = 0x0400_0012;
    const BG2PA_ADDR: u32 = 0x0400_0020;
    const BG2X_ADDR: u32 = 0x0400_0028;
    const BG2Y_ADDR: u32 = 0x0400_002c;
    const BG3CNT_ADDR: u32 = 0x0400_000e;
    const DISPCNT_ADDR: u32 = 0x0400_0000;
    const DMA0DAD_ADDR: u32 = 0x0400_00b4;
    const DMA0CNT_L_ADDR: u32 = 0x0400_00b8;

    #[test]
    fn ordinary_registers_round_trip() {
        let mut io = IoRegs::new();
        io.write_16(IME_ADDR, 1);
        io.write_16(IE_ADDR, 0x1234);
        io.write_16(WAITCNT_ADDR, 0x4321);

        assert_eq!(io.read_16(IME_ADDR), 1);
        assert_eq!(io.read_16(IE_ADDR), 0x1234);
        assert_eq!(io.read_16(WAITCNT_ADDR), 0x4321);
    }

    #[test]
    fn display_register_helpers_decode_mode_frame_and_background_bits() {
        let mut io = IoRegs::new();
        io.write_16(DISPCNT_ADDR, 0x0511);

        assert_eq!(io.display_mode(), 1);
        assert!(io.display_frame_select());
        assert!(io.bg_enabled(0));
        assert!(!io.bg_enabled(1));
        assert!(io.bg_enabled(2));
    }

    #[test]
    fn background_registers_round_trip_and_apply_masks() {
        let mut io = IoRegs::new();
        io.write_16(BG0CNT_ADDR, 0x4104);
        io.write_16(BG0HOFS_ADDR, 0x03ab);
        io.write_16(BG0VOFS_ADDR, 0x02cd);
        io.write_16(BG3CNT_ADDR, 0x6110);

        assert_eq!(io.read_16(BG0CNT_ADDR), 0x4104);
        assert_eq!(io.bg_char_base_block(0), 1);
        assert_eq!(io.bg_screen_base_block(0), 1);
        assert_eq!(io.bg_size(0), 1);
        assert_eq!(io.read_16(BG0HOFS_ADDR), 0x01ab);
        assert_eq!(io.read_16(BG0VOFS_ADDR), 0x00cd);
        assert_eq!(io.bg_screen_base_block(3), 1);
        assert_eq!(io.bg_size(3), 1);
        assert!(io.bg_wrap(3));
    }

    #[test]
    fn affine_background_registers_round_trip() {
        let mut io = IoRegs::new();
        io.write_16(BG2PA_ADDR, 0x0100);
        io.write_16(BG2PA_ADDR + 2, 0xff80);
        io.write_16(BG2PA_ADDR + 4, 0x0020);
        io.write_16(BG2PA_ADDR + 6, 0x00c0);
        io.write_32(BG2X_ADDR, 0x0001_0080);
        io.write_32(BG2Y_ADDR, 0xffff_ff00);

        assert_eq!(io.read_16(BG2PA_ADDR), 0x0100);
        assert_eq!(io.read_16(BG2PA_ADDR + 2), 0xff80);
        assert_eq!(io.bg_affine_matrix(2), Some((0x0100, -128, 0x0020, 0x00c0)));
        assert_eq!(io.bg_affine_ref_point(2), Some((0x0001_0080, -0x100)));
    }

    #[test]
    fn interrupt_flags_are_write_one_to_clear() {
        let mut io = IoRegs::new();
        io.request_interrupt(0b111);
        io.write_16(IF_ADDR, 0b010);

        assert_eq!(io.read_16(IF_ADDR), 0b101);
    }

    #[test]
    fn key_registers_round_trip_with_masks() {
        let mut io = IoRegs::new();

        assert_eq!(io.read_16(KEYINPUT_ADDR), 0x03ff);
        io.write_16(KEYINPUT_ADDR, 0);
        assert_eq!(io.read_16(KEYINPUT_ADDR), 0x03ff);

        io.write_16(KEYCNT_ADDR, 0xffff);
        assert_eq!(io.read_16(KEYCNT_ADDR), 0xc3ff);

        io.request_interrupt(IRQ_KEYPAD);
        assert_ne!(io.read_16(IF_ADDR) & IRQ_KEYPAD, 0);
    }

    #[test]
    fn vblank_event_sets_status_and_requests_irq_when_enabled() {
        let mut io = IoRegs::new();
        io.write_16(DISPSTAT_ADDR, 1 << 3);
        io.set_vblank(true);

        assert_ne!(io.dispstat() & 1, 0);
        assert_ne!(io.if_() & IRQ_VBLANK, 0);
    }

    #[test]
    fn vcount_register_tracks_current_scanline() {
        let mut io = IoRegs::new();
        io.set_vcount(37);

        assert_eq!(io.read_16(VCOUNT_ADDR), 37);
    }

    #[test]
    fn vcount_match_sets_status_and_requests_irq_once() {
        let mut io = IoRegs::new();
        io.write_16(DISPSTAT_ADDR, (1 << 5) | (12 << 8));

        io.set_vcount(11);
        assert_eq!(io.dispstat() & (1 << 2), 0);
        assert_eq!(io.if_() & IRQ_VCOUNT, 0);

        io.set_vcount(12);
        assert_ne!(io.dispstat() & (1 << 2), 0);
        assert_ne!(io.if_() & IRQ_VCOUNT, 0);

        let flags = io.if_();
        io.set_vcount(12);
        assert_eq!(io.if_(), flags);
    }

    #[test]
    fn timer_registers_keep_reload_counter_and_control_separate() {
        let mut io = IoRegs::new();
        io.write_16(TM0CNT_L_ADDR, 0xff80);
        io.write_16(TM0CNT_H_ADDR, 0x00c1);
        io.set_timer_counter(0, 0xff82);

        assert_eq!(io.timer_reload(0), 0xff80);
        assert_eq!(io.read_16(TM0CNT_L_ADDR), 0xff82);
        assert_eq!(io.read_16(TM0CNT_H_ADDR), 0x00c1);
    }

    #[test]
    fn dma_registers_round_trip_through_mmio_layout() {
        let mut io = IoRegs::new();
        io.write_32(DMA0SAD_ADDR, 0x0200_0010);
        io.write_32(DMA0DAD_ADDR, 0x0300_0020);
        io.write_16(DMA0CNT_L_ADDR, 8);
        io.write_16(DMA0CNT_H_ADDR, 0x8400);

        assert_eq!(io.read_32(DMA0SAD_ADDR), 0x0200_0010);
        assert_eq!(io.read_32(DMA0DAD_ADDR), 0x0300_0020);
        assert_eq!(io.read_16(DMA0CNT_L_ADDR), 8);
        assert_eq!(io.read_16(DMA0CNT_H_ADDR), 0x8400);
    }
}
