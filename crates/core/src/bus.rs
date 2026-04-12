use rgba_arm7tdmi::BusInterface;

use crate::cartridge::Cartridge;
use crate::mem::{BiosLoadError, Memory};

const IO_SIZE: usize = 0x400;
const PALETTE_SIZE: usize = 0x400;
const VRAM_SIZE: usize = 0x18000;
const OAM_SIZE: usize = 0x400;
const SRAM_SIZE: usize = 0x10000;

const BIOS_BASE: u32 = 0x0000_0000;
const EWRAM_BASE: u32 = 0x0200_0000;
const IWRAM_BASE: u32 = 0x0300_0000;

/// The shared address bus for the whole GBA.
///
/// Phase 2 keeps this intentionally small: enough to route CPU accesses to
/// BIOS, WRAM and Game Pak ROM, while reserving storage for future MMIO/PPU
/// work.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bus {
    memory: Memory,
    cartridge: Cartridge,
    io: Box<[u8; IO_SIZE]>,
    palette: Box<[u8; PALETTE_SIZE]>,
    vram: Box<[u8; VRAM_SIZE]>,
    oam: Box<[u8; OAM_SIZE]>,
    sram: Box<[u8; SRAM_SIZE]>,
}

impl Bus {
    pub fn new(cartridge: Cartridge) -> Self {
        Self {
            memory: Memory::new(),
            cartridge,
            io: Box::new([0; IO_SIZE]),
            palette: Box::new([0; PALETTE_SIZE]),
            vram: Box::new([0; VRAM_SIZE]),
            oam: Box::new([0; OAM_SIZE]),
            sram: Box::new([0; SRAM_SIZE]),
        }
    }

    pub fn memory(&self) -> &Memory {
        &self.memory
    }

    pub fn memory_mut(&mut self) -> &mut Memory {
        &mut self.memory
    }

    pub fn load_bios(&mut self, bios: &[u8]) -> Result<(), BiosLoadError> {
        self.memory.load_bios(bios)
    }

    pub fn cartridge(&self) -> &Cartridge {
        &self.cartridge
    }

    pub fn read_32_debug(&self, addr: u32) -> u32 {
        match (addr >> 24) & 0xff {
            0x00 => self.memory.read_bios_32(addr - BIOS_BASE),
            0x02 => self.memory.read_ewram_32(addr - EWRAM_BASE),
            0x03 => self.memory.read_iwram_32(addr - IWRAM_BASE),
            0x04 => read_u32(&self.io[..], (addr & (IO_SIZE as u32 - 1)) as usize),
            0x05 => read_u32(&self.palette[..], (addr & (PALETTE_SIZE as u32 - 1)) as usize),
            0x06 => read_u32(&self.vram[..], vram_offset(addr)),
            0x07 => read_u32(&self.oam[..], (addr & (OAM_SIZE as u32 - 1)) as usize),
            0x08..=0x0d => self.cartridge.read_32(addr),
            0x0e | 0x0f => read_u32(&self.sram[..], (addr & (SRAM_SIZE as u32 - 1)) as usize),
            _ => 0,
        }
    }

    pub fn write_32_debug(&mut self, addr: u32, val: u32) {
        self.write_32(addr, val);
    }
}

impl BusInterface for Bus {
    fn read_8(&mut self, addr: u32) -> u8 {
        match (addr >> 24) & 0xff {
            0x00 => self.memory.read_bios_8(addr - BIOS_BASE),
            0x02 => self.memory.read_ewram_8(addr - EWRAM_BASE),
            0x03 => self.memory.read_iwram_8(addr - IWRAM_BASE),
            0x04 => self.io[(addr & (IO_SIZE as u32 - 1)) as usize],
            0x05 => self.palette[(addr & (PALETTE_SIZE as u32 - 1)) as usize],
            0x06 => self.vram[vram_offset(addr)],
            0x07 => self.oam[(addr & (OAM_SIZE as u32 - 1)) as usize],
            0x08..=0x0d => self.cartridge.read_8(addr),
            0x0e | 0x0f => self.sram[(addr & (SRAM_SIZE as u32 - 1)) as usize],
            _ => 0,
        }
    }

    fn read_16(&mut self, addr: u32) -> u16 {
        match (addr >> 24) & 0xff {
            0x00 => self.memory.read_bios_16(addr - BIOS_BASE),
            0x02 => self.memory.read_ewram_16(addr - EWRAM_BASE),
            0x03 => self.memory.read_iwram_16(addr - IWRAM_BASE),
            0x04 => read_u16(&self.io[..], (addr & (IO_SIZE as u32 - 1)) as usize),
            0x05 => read_u16(&self.palette[..], (addr & (PALETTE_SIZE as u32 - 1)) as usize),
            0x06 => read_u16(&self.vram[..], vram_offset(addr)),
            0x07 => read_u16(&self.oam[..], (addr & (OAM_SIZE as u32 - 1)) as usize),
            0x08..=0x0d => self.cartridge.read_16(addr),
            0x0e | 0x0f => read_u16(&self.sram[..], (addr & (SRAM_SIZE as u32 - 1)) as usize),
            _ => 0,
        }
    }

    fn read_32(&mut self, addr: u32) -> u32 {
        match (addr >> 24) & 0xff {
            0x00 => self.memory.read_bios_32(addr - BIOS_BASE),
            0x02 => self.memory.read_ewram_32(addr - EWRAM_BASE),
            0x03 => self.memory.read_iwram_32(addr - IWRAM_BASE),
            0x04 => read_u32(&self.io[..], (addr & (IO_SIZE as u32 - 1)) as usize),
            0x05 => read_u32(&self.palette[..], (addr & (PALETTE_SIZE as u32 - 1)) as usize),
            0x06 => read_u32(&self.vram[..], vram_offset(addr)),
            0x07 => read_u32(&self.oam[..], (addr & (OAM_SIZE as u32 - 1)) as usize),
            0x08..=0x0d => self.cartridge.read_32(addr),
            0x0e | 0x0f => read_u32(&self.sram[..], (addr & (SRAM_SIZE as u32 - 1)) as usize),
            _ => 0,
        }
    }

    fn write_8(&mut self, addr: u32, val: u8) {
        match (addr >> 24) & 0xff {
            0x00 => {}
            0x02 => self.memory.write_ewram_8(addr - EWRAM_BASE, val),
            0x03 => self.memory.write_iwram_8(addr - IWRAM_BASE, val),
            0x04 => self.io[(addr & (IO_SIZE as u32 - 1)) as usize] = val,
            0x05 => self.palette[(addr & (PALETTE_SIZE as u32 - 1)) as usize] = val,
            0x06 => {
                let offset = vram_offset(addr);
                self.vram[offset] = val;
            }
            0x07 => self.oam[(addr & (OAM_SIZE as u32 - 1)) as usize] = val,
            0x08..=0x0d => {}
            0x0e | 0x0f => self.sram[(addr & (SRAM_SIZE as u32 - 1)) as usize] = val,
            _ => {}
        }
    }

    fn write_16(&mut self, addr: u32, val: u16) {
        match (addr >> 24) & 0xff {
            0x00 => {}
            0x02 => self.memory.write_ewram_16(addr - EWRAM_BASE, val),
            0x03 => self.memory.write_iwram_16(addr - IWRAM_BASE, val),
            0x04 => write_u16(&mut self.io[..], (addr & (IO_SIZE as u32 - 1)) as usize, val),
            0x05 => write_u16(
                &mut self.palette[..],
                (addr & (PALETTE_SIZE as u32 - 1)) as usize,
                val,
            ),
            0x06 => write_u16(&mut self.vram[..], vram_offset(addr), val),
            0x07 => write_u16(&mut self.oam[..], (addr & (OAM_SIZE as u32 - 1)) as usize, val),
            0x08..=0x0d => {}
            0x0e | 0x0f => write_u16(&mut self.sram[..], (addr & (SRAM_SIZE as u32 - 1)) as usize, val),
            _ => {}
        }
    }

    fn write_32(&mut self, addr: u32, val: u32) {
        match (addr >> 24) & 0xff {
            0x00 => {}
            0x02 => self.memory.write_ewram_32(addr - EWRAM_BASE, val),
            0x03 => self.memory.write_iwram_32(addr - IWRAM_BASE, val),
            0x04 => write_u32(&mut self.io[..], (addr & (IO_SIZE as u32 - 1)) as usize, val),
            0x05 => write_u32(
                &mut self.palette[..],
                (addr & (PALETTE_SIZE as u32 - 1)) as usize,
                val,
            ),
            0x06 => write_u32(&mut self.vram[..], vram_offset(addr), val),
            0x07 => write_u32(&mut self.oam[..], (addr & (OAM_SIZE as u32 - 1)) as usize, val),
            0x08..=0x0d => {}
            0x0e | 0x0f => write_u32(&mut self.sram[..], (addr & (SRAM_SIZE as u32 - 1)) as usize, val),
            _ => {}
        }
    }
}

fn vram_offset(addr: u32) -> usize {
    let offset = (addr & 0x0001_ffff) as usize;
    if offset >= VRAM_SIZE {
        offset - 0x8000
    } else {
        offset
    }
}

fn read_u16(slice: &[u8], offset: usize) -> u16 {
    let lo = slice[offset] as u16;
    let hi = slice[offset.wrapping_add(1) % slice.len()] as u16;
    lo | (hi << 8)
}

fn read_u32(slice: &[u8], offset: usize) -> u32 {
    u32::from(slice[offset])
        | (u32::from(slice[offset.wrapping_add(1) % slice.len()]) << 8)
        | (u32::from(slice[offset.wrapping_add(2) % slice.len()]) << 16)
        | (u32::from(slice[offset.wrapping_add(3) % slice.len()]) << 24)
}

fn write_u16(slice: &mut [u8], offset: usize, val: u16) {
    slice[offset] = val as u8;
    slice[offset.wrapping_add(1) % slice.len()] = (val >> 8) as u8;
}

fn write_u32(slice: &mut [u8], offset: usize, val: u32) {
    slice[offset] = val as u8;
    slice[offset.wrapping_add(1) % slice.len()] = (val >> 8) as u8;
    slice[offset.wrapping_add(2) % slice.len()] = (val >> 16) as u8;
    slice[offset.wrapping_add(3) % slice.len()] = (val >> 24) as u8;
}

#[cfg(test)]
mod tests {
    use super::Bus;
    use crate::cartridge::Cartridge;
    use rgba_arm7tdmi::BusInterface;

    #[test]
    fn bus_routes_rom_and_wram_regions() {
        let mut bus = Bus::new(Cartridge::new(vec![0x78, 0x56, 0x34, 0x12]));

        bus.write_32(0x0200_0000, 0xdead_beef);
        bus.write_32(0x0300_0000, 0xaabb_ccdd);

        assert_eq!(bus.read_32(0x0800_0000), 0x1234_5678);
        assert_eq!(bus.read_32(0x0200_0000), 0xdead_beef);
        assert_eq!(bus.read_32(0x0300_0000), 0xaabb_ccdd);
    }
}
