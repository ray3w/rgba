const BIOS_SIZE: usize = 0x4000;
const EWRAM_SIZE: usize = 0x40000;
const IWRAM_SIZE: usize = 0x8000;

/// On-board system memory owned by the GBA core.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Memory {
    bios: Box<[u8; BIOS_SIZE]>,
    ewram: Box<[u8; EWRAM_SIZE]>,
    iwram: Box<[u8; IWRAM_SIZE]>,
}

impl Default for Memory {
    fn default() -> Self {
        Self::new()
    }
}

impl Memory {
    pub fn new() -> Self {
        Self {
            bios: Box::new([0; BIOS_SIZE]),
            ewram: Box::new([0; EWRAM_SIZE]),
            iwram: Box::new([0; IWRAM_SIZE]),
        }
    }

    pub fn load_bios(&mut self, bios: &[u8]) -> Result<(), BiosLoadError> {
        if bios.len() > BIOS_SIZE {
            return Err(BiosLoadError::TooLarge {
                actual: bios.len(),
                max: BIOS_SIZE,
            });
        }

        self.bios.fill(0);
        self.bios[..bios.len()].copy_from_slice(bios);
        Ok(())
    }

    pub fn read_bios_8(&self, addr: u32) -> u8 {
        self.bios[(addr as usize) & (BIOS_SIZE - 1)]
    }

    pub fn read_bios_16(&self, addr: u32) -> u16 {
        read_u16(&self.bios[..], (addr as usize) & (BIOS_SIZE - 1))
    }

    pub fn read_bios_32(&self, addr: u32) -> u32 {
        read_u32(&self.bios[..], (addr as usize) & (BIOS_SIZE - 1))
    }

    pub fn read_ewram_8(&self, addr: u32) -> u8 {
        self.ewram[(addr as usize) & (EWRAM_SIZE - 1)]
    }

    pub fn read_ewram_16(&self, addr: u32) -> u16 {
        read_u16(&self.ewram[..], (addr as usize) & (EWRAM_SIZE - 1))
    }

    pub fn read_ewram_32(&self, addr: u32) -> u32 {
        read_u32(&self.ewram[..], (addr as usize) & (EWRAM_SIZE - 1))
    }

    pub fn write_ewram_8(&mut self, addr: u32, val: u8) {
        let offset = (addr as usize) & (EWRAM_SIZE - 1);
        self.ewram[offset] = val;
    }

    pub fn write_ewram_16(&mut self, addr: u32, val: u16) {
        write_u16(&mut self.ewram[..], (addr as usize) & (EWRAM_SIZE - 1), val);
    }

    pub fn write_ewram_32(&mut self, addr: u32, val: u32) {
        write_u32(&mut self.ewram[..], (addr as usize) & (EWRAM_SIZE - 1), val);
    }

    pub fn read_iwram_8(&self, addr: u32) -> u8 {
        self.iwram[(addr as usize) & (IWRAM_SIZE - 1)]
    }

    pub fn read_iwram_16(&self, addr: u32) -> u16 {
        read_u16(&self.iwram[..], (addr as usize) & (IWRAM_SIZE - 1))
    }

    pub fn read_iwram_32(&self, addr: u32) -> u32 {
        read_u32(&self.iwram[..], (addr as usize) & (IWRAM_SIZE - 1))
    }

    pub fn write_iwram_8(&mut self, addr: u32, val: u8) {
        let offset = (addr as usize) & (IWRAM_SIZE - 1);
        self.iwram[offset] = val;
    }

    pub fn write_iwram_16(&mut self, addr: u32, val: u16) {
        write_u16(&mut self.iwram[..], (addr as usize) & (IWRAM_SIZE - 1), val);
    }

    pub fn write_iwram_32(&mut self, addr: u32, val: u32) {
        write_u32(&mut self.iwram[..], (addr as usize) & (IWRAM_SIZE - 1), val);
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BiosLoadError {
    TooLarge { actual: usize, max: usize },
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
    use super::{BiosLoadError, Memory};

    #[test]
    fn ewram_mirrors_across_its_whole_window() {
        let mut mem = Memory::new();
        mem.write_ewram_32(0x0200_0000, 0x1234_5678);

        assert_eq!(mem.read_ewram_32(0x0204_0000), 0x1234_5678);
    }

    #[test]
    fn iwram_mirrors_across_its_whole_window() {
        let mut mem = Memory::new();
        mem.write_iwram_16(0x0300_0002, 0xbeef);

        assert_eq!(mem.read_iwram_16(0x0300_8002), 0xbeef);
    }

    #[test]
    fn bios_load_rejects_oversized_images() {
        let mut mem = Memory::new();
        let err = mem.load_bios(&vec![0; 0x4001]).unwrap_err();
        assert_eq!(
            err,
            BiosLoadError::TooLarge {
                actual: 0x4001,
                max: 0x4000
            }
        );
    }
}
