use rgba_arm7tdmi::BusInterface;

use crate::bus::Bus;
use crate::io::{IRQ_DMA0, IRQ_DMA1, IRQ_DMA2, IRQ_DMA3};

const DMA_ENABLE: u16 = 1 << 15;
const DMA_IRQ_ENABLE: u16 = 1 << 14;
const DMA_START_TIMING_SHIFT: u16 = 12;
const DMA_TRANSFER_32: u16 = 1 << 10;
const DMA_REPEAT: u16 = 1 << 9;

const DMA_IRQS: [u16; 4] = [IRQ_DMA0, IRQ_DMA1, IRQ_DMA2, IRQ_DMA3];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DmaTiming {
    Immediate,
    VBlank,
    HBlank,
    Special,
}

/// Minimal DMA controller used for Phase 5.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DmaController {
    previous_enable: [bool; 4],
}

impl DmaController {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn service(&mut self, bus: &mut Bus, entered_hblank: bool, entered_vblank: bool) {
        for index in 0..4 {
            let control = bus.io().dma_control(index);
            let enabled = (control & DMA_ENABLE) != 0;
            let timing = timing(control);

            let should_run = enabled
                && match timing {
                    DmaTiming::Immediate => !self.previous_enable[index],
                    DmaTiming::VBlank => entered_vblank,
                    DmaTiming::HBlank => entered_hblank,
                    DmaTiming::Special => false,
                };

            if should_run {
                self.run_channel(bus, index);
            }

            self.previous_enable[index] = (bus.io().dma_control(index) & DMA_ENABLE) != 0;
        }
    }

    fn run_channel(&mut self, bus: &mut Bus, index: usize) {
        let source = bus.io().dma_source(index);
        let dest = bus.io().dma_dest(index);
        let count = bus.io().dma_count(index);
        let control = bus.io().dma_control(index);

        let unit = if (control & DMA_TRANSFER_32) != 0 {
            4
        } else {
            2
        };
        let mut src = align_addr(source, unit);
        let mut dst = align_addr(dest, unit);
        let units = transfer_units(index, count);
        let src_step = src_step(control, unit as i32);
        let dst_step = dst_step(control, unit as i32);

        for _ in 0..units {
            if unit == 4 {
                let value = BusInterface::read_32(bus, src);
                BusInterface::write_32(bus, dst, value);
            } else {
                let value = BusInterface::read_16(bus, src);
                BusInterface::write_16(bus, dst, value);
            }

            src = src.wrapping_add_signed(src_step);
            dst = dst.wrapping_add_signed(dst_step);
        }

        bus.io_mut().set_dma_source(index, src);
        let original_dest = bus.io().dma_dest(index);
        if ((control >> 5) & 0x3) == 3 {
            bus.io_mut().set_dma_dest(index, original_dest);
        } else {
            bus.io_mut().set_dma_dest(index, dst);
        }

        let repeat = (control & DMA_REPEAT) != 0;
        let timing = timing(control);
        if !repeat || matches!(timing, DmaTiming::Immediate) {
            bus.io_mut().set_dma_control(index, control & !DMA_ENABLE);
        }

        if (control & DMA_IRQ_ENABLE) != 0 {
            bus.io_mut().request_interrupt(DMA_IRQS[index]);
        }
    }
}

fn timing(control: u16) -> DmaTiming {
    match (control >> DMA_START_TIMING_SHIFT) & 0x3 {
        0 => DmaTiming::Immediate,
        1 => DmaTiming::VBlank,
        2 => DmaTiming::HBlank,
        _ => DmaTiming::Special,
    }
}

fn transfer_units(index: usize, count: u16) -> u32 {
    if count != 0 {
        u32::from(count)
    } else if index == 3 {
        0x1_0000
    } else {
        0x4000
    }
}

fn align_addr(addr: u32, unit: u32) -> u32 {
    if unit == 4 {
        addr & !3
    } else {
        addr & !1
    }
}

fn src_step(control: u16, unit: i32) -> i32 {
    match (control >> 7) & 0x3 {
        0 => unit,
        1 => -unit,
        2 => 0,
        _ => 0,
    }
}

fn dst_step(control: u16, unit: i32) -> i32 {
    match (control >> 5) & 0x3 {
        0 | 3 => unit,
        1 => -unit,
        2 => 0,
        _ => unit,
    }
}

#[cfg(test)]
mod tests {
    use super::DmaController;
    use crate::bus::Bus;
    use crate::cartridge::Cartridge;
    use rgba_arm7tdmi::BusInterface;

    #[test]
    fn immediate_dma_copies_words_and_disables_channel() {
        let mut dma = DmaController::new();
        let mut bus = Bus::new(Cartridge::new(vec![0; 4]));
        BusInterface::write_32(&mut bus, 0x0200_0000, 0x1234_5678);
        BusInterface::write_32(&mut bus, 0x0200_0004, 0x89ab_cdef);

        bus.io_mut().write_32(0x0400_00b0, 0x0200_0000);
        bus.io_mut().write_32(0x0400_00b4, 0x0300_0000);
        bus.io_mut().write_16(0x0400_00b8, 2);
        bus.io_mut().write_16(0x0400_00ba, 0x8400);

        dma.service(&mut bus, false, false);

        assert_eq!(BusInterface::read_32(&mut bus, 0x0300_0000), 0x1234_5678);
        assert_eq!(BusInterface::read_32(&mut bus, 0x0300_0004), 0x89ab_cdef);
        assert_eq!(bus.io().dma_control(0) & 0x8000, 0);
    }

    #[test]
    fn vblank_dma_waits_for_vblank_edge() {
        let mut dma = DmaController::new();
        let mut bus = Bus::new(Cartridge::new(vec![0; 4]));
        BusInterface::write_16(&mut bus, 0x0200_0000, 0x1357);

        bus.io_mut().write_32(0x0400_00bc, 0x0200_0000);
        bus.io_mut().write_32(0x0400_00c0, 0x0300_0000);
        bus.io_mut().write_16(0x0400_00c4, 1);
        bus.io_mut().write_16(0x0400_00c6, 0x9000);

        dma.service(&mut bus, false, false);
        assert_eq!(BusInterface::read_16(&mut bus, 0x0300_0000), 0);

        dma.service(&mut bus, false, true);
        assert_eq!(BusInterface::read_16(&mut bus, 0x0300_0000), 0x1357);
    }
}
