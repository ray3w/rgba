use rgba_arm7tdmi::BusInterface;

const BIOS_BASE: u32 = 0x0000_0000;
const BIOS_SIZE: usize = 0x4000;
const EWRAM_BASE: u32 = 0x0200_0000;
const EWRAM_SIZE: usize = 0x40000;
const IWRAM_BASE: u32 = 0x0300_0000;
const IWRAM_SIZE: usize = 0x8000;
const IO_BASE: u32 = 0x0400_0000;
const IO_SIZE: usize = 0x400;
const PALETTE_BASE: u32 = 0x0500_0000;
const PALETTE_SIZE: usize = 0x400;
const VRAM_BASE: u32 = 0x0600_0000;
const VRAM_SIZE: usize = 0x18000;
const OAM_BASE: u32 = 0x0700_0000;
const OAM_SIZE: usize = 0x400;
const GAMEPAK0_BASE: u32 = 0x0800_0000;
const GAMEPAK1_BASE: u32 = 0x0a00_0000;
const GAMEPAK2_BASE: u32 = 0x0c00_0000;
const SRAM_BASE: u32 = 0x0e00_0000;
const SRAM_SIZE: usize = 0x10000;
const DISPSTAT_ADDR: u32 = IO_BASE + 0x0004;

#[derive(Debug, Clone)]
pub struct TestBus {
    bios: Box<[u8; BIOS_SIZE]>,
    ewram: Box<[u8; EWRAM_SIZE]>,
    iwram: Box<[u8; IWRAM_SIZE]>,
    io: Box<[u8; IO_SIZE]>,
    palette: Box<[u8; PALETTE_SIZE]>,
    vram: Box<[u8; VRAM_SIZE]>,
    oam: Box<[u8; OAM_SIZE]>,
    sram: Box<[u8; SRAM_SIZE]>,
    rom: Vec<u8>,
    next_vblank: bool,
}

impl TestBus {
    pub fn new(rom: Vec<u8>) -> Self {
        Self {
            bios: Box::new([0; BIOS_SIZE]),
            ewram: Box::new([0; EWRAM_SIZE]),
            iwram: Box::new([0; IWRAM_SIZE]),
            io: Box::new([0; IO_SIZE]),
            palette: Box::new([0; PALETTE_SIZE]),
            vram: Box::new([0; VRAM_SIZE]),
            oam: Box::new([0; OAM_SIZE]),
            sram: Box::new([0; SRAM_SIZE]),
            rom,
            next_vblank: false,
        }
    }

    pub fn read_rom_word(&self, addr: u32) -> u32 {
        self.read32_fallible(addr).unwrap_or(0)
    }

    fn read32_fallible(&self, addr: u32) -> Option<u32> {
        Some(
            u32::from(self.peek_u8(addr)?)
                | (u32::from(self.peek_u8(addr.wrapping_add(1))?) << 8)
                | (u32::from(self.peek_u8(addr.wrapping_add(2))?) << 16)
                | (u32::from(self.peek_u8(addr.wrapping_add(3))?) << 24),
        )
    }

    fn read16_fallible(&self, addr: u32) -> Option<u16> {
        Some(u16::from(self.peek_u8(addr)?) | (u16::from(self.peek_u8(addr.wrapping_add(1))?) << 8))
    }

    fn peek_u8(&self, addr: u32) -> Option<u8> {
        let (slice, offset) = self.region(addr)?;
        Some(slice[offset])
    }

    /// Map an address to a (base, size, slice) triple, applying GBA mirror rules.
    fn mirror_offset(addr: u32) -> Option<(u32, usize)> {
        let region = (addr >> 24) & 0xff;
        match region {
            0x00 => Some((BIOS_BASE, (addr & (BIOS_SIZE as u32 - 1)) as usize)),
            0x02 => Some((EWRAM_BASE, (addr & (EWRAM_SIZE as u32 - 1)) as usize)),
            0x03 => Some((IWRAM_BASE, (addr & (IWRAM_SIZE as u32 - 1)) as usize)),
            0x04 => Some((IO_BASE, (addr & (IO_SIZE as u32 - 1)) as usize)),
            0x05 => Some((PALETTE_BASE, (addr & (PALETTE_SIZE as u32 - 1)) as usize)),
            0x06 => {
                // VRAM: 96KB total. 0x00000-0x17FFF maps directly.
                // 0x18000-0x1FFFF mirrors 0x10000-0x17FFF (last 32KB).
                // The whole 128KB pattern then repeats.
                let offset = (addr & 0x0001_ffff) as usize;
                let mirrored = if offset >= VRAM_SIZE {
                    offset - 0x8000
                } else {
                    offset
                };
                Some((VRAM_BASE, mirrored))
            }
            0x07 => Some((OAM_BASE, (addr & (OAM_SIZE as u32 - 1)) as usize)),
            0x0e | 0x0f => Some((SRAM_BASE, (addr & (SRAM_SIZE as u32 - 1)) as usize)),
            _ => None,
        }
    }

    fn region(&self, addr: u32) -> Option<(&[u8], usize)> {
        if let Some((base, offset)) = Self::mirror_offset(addr) {
            let slice: &[u8] = match base {
                BIOS_BASE => &self.bios[..],
                EWRAM_BASE => &self.ewram[..],
                IWRAM_BASE => &self.iwram[..],
                IO_BASE => &self.io[..],
                PALETTE_BASE => &self.palette[..],
                VRAM_BASE => &self.vram[..],
                OAM_BASE => &self.oam[..],
                SRAM_BASE => &self.sram[..],
                _ => return None,
            };
            Some((slice, offset))
        } else if let Some((_, offset)) = self.rom_offset(addr) {
            if offset < self.rom.len() {
                return Some((&self.rom[..], offset));
            }
            None
        } else {
            None
        }
    }

    fn region_mut(&mut self, addr: u32) -> Option<(&mut [u8], usize)> {
        let (base, offset) = Self::mirror_offset(addr)?;
        let slice: &mut [u8] = match base {
            BIOS_BASE => &mut self.bios[..],
            EWRAM_BASE => &mut self.ewram[..],
            IWRAM_BASE => &mut self.iwram[..],
            IO_BASE => &mut self.io[..],
            PALETTE_BASE => &mut self.palette[..],
            VRAM_BASE => &mut self.vram[..],
            OAM_BASE => &mut self.oam[..],
            SRAM_BASE => &mut self.sram[..],
            _ => return None,
        };
        Some((slice, offset))
    }

    fn rom_offset(&self, addr: u32) -> Option<(u32, usize)> {
        if self.rom.is_empty() {
            return None;
        }

        let base = if addr >= GAMEPAK2_BASE {
            GAMEPAK2_BASE
        } else if addr >= GAMEPAK1_BASE {
            GAMEPAK1_BASE
        } else if addr >= GAMEPAK0_BASE {
            GAMEPAK0_BASE
        } else {
            return None;
        };

        Some((base, (addr - base) as usize))
    }

    fn display_mode(&self) -> u8 {
        (self.io[0] & 0x07) as u8
    }

    fn is_vram_obj_area(&self, vram_offset: usize) -> bool {
        let obj_boundary = if self.display_mode() >= 3 {
            0x14000
        } else {
            0x10000
        };
        vram_offset >= obj_boundary
    }

    /// Raw byte write — no special STRB semantics.
    fn raw_write_8(&mut self, addr: u32, val: u8) {
        if let Some((slice, offset)) = self.region_mut(addr) {
            slice[offset] = val;
        }
    }

    fn read_dispstat_word(&mut self) -> u32 {
        let low = if self.next_vblank { 1u8 } else { 0u8 };
        self.next_vblank = !self.next_vblank;
        self.io[0x04] = low;
        u32::from(low)
    }

    fn read_dispstat_halfword(&mut self) -> u16 {
        self.read_dispstat_word() as u16
    }
}

impl BusInterface for TestBus {
    fn read_8(&mut self, addr: u32) -> u8 {
        if addr == DISPSTAT_ADDR {
            self.read_dispstat_halfword() as u8
        } else {
            self.peek_u8(addr).unwrap_or(0)
        }
    }

    fn read_16(&mut self, addr: u32) -> u16 {
        if addr == DISPSTAT_ADDR {
            self.read_dispstat_halfword()
        } else {
            self.read16_fallible(addr).unwrap_or(0)
        }
    }

    fn read_32(&mut self, addr: u32) -> u32 {
        if addr == DISPSTAT_ADDR {
            self.read_dispstat_word()
        } else {
            self.read32_fallible(addr).unwrap_or(0)
        }
    }

    fn write_8(&mut self, addr: u32, val: u8) {
        let region = (addr >> 24) & 0xff;
        match region {
            // OAM: byte writes are completely ignored
            0x07 => {}
            // Palette: byte is duplicated into a halfword
            0x05 => {
                let aligned = addr & !1;
                let hword = u16::from(val) | (u16::from(val) << 8);
                self.raw_write_8(aligned, hword as u8);
                self.raw_write_8(aligned.wrapping_add(1), (hword >> 8) as u8);
            }
            // VRAM: byte is duplicated into a halfword, except OBJ area is ignored
            0x06 => {
                let vram_offset = (addr & 0x0001_ffff) as usize;
                let mirrored = if vram_offset >= VRAM_SIZE {
                    vram_offset - 0x8000
                } else {
                    vram_offset
                };
                if self.is_vram_obj_area(mirrored) {
                    return;
                }
                let aligned = addr & !1;
                let hword = u16::from(val) | (u16::from(val) << 8);
                self.raw_write_8(aligned, hword as u8);
                self.raw_write_8(aligned.wrapping_add(1), (hword >> 8) as u8);
            }
            _ => self.raw_write_8(addr, val),
        }
    }

    fn write_16(&mut self, addr: u32, val: u16) {
        self.raw_write_8(addr, val as u8);
        self.raw_write_8(addr.wrapping_add(1), (val >> 8) as u8);
    }

    fn write_32(&mut self, addr: u32, val: u32) {
        self.raw_write_8(addr, val as u8);
        self.raw_write_8(addr.wrapping_add(1), (val >> 8) as u8);
        self.raw_write_8(addr.wrapping_add(2), (val >> 16) as u8);
        self.raw_write_8(addr.wrapping_add(3), (val >> 24) as u8);
    }
}
