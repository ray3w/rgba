use std::fs;
use std::io;
use std::path::Path;

const GAMEPAK0_BASE: u32 = 0x0800_0000;
const GAMEPAK1_BASE: u32 = 0x0a00_0000;
const GAMEPAK2_BASE: u32 = 0x0c00_0000;

/// Loaded Game Pak ROM image.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cartridge {
    rom: Vec<u8>,
}

impl Cartridge {
    pub fn new(rom: Vec<u8>) -> Self {
        Self { rom }
    }

    pub fn from_file(path: impl AsRef<Path>) -> io::Result<Self> {
        Ok(Self::new(fs::read(path)?))
    }

    pub fn len(&self) -> usize {
        self.rom.len()
    }

    pub fn is_empty(&self) -> bool {
        self.rom.is_empty()
    }

    pub fn read_8(&self, addr: u32) -> u8 {
        self.rom_offset(addr)
            .and_then(|offset| self.rom.get(offset).copied())
            .unwrap_or(0)
    }

    pub fn read_16(&self, addr: u32) -> u16 {
        u16::from(self.read_8(addr)) | (u16::from(self.read_8(addr.wrapping_add(1))) << 8)
    }

    pub fn read_32(&self, addr: u32) -> u32 {
        u32::from(self.read_8(addr))
            | (u32::from(self.read_8(addr.wrapping_add(1))) << 8)
            | (u32::from(self.read_8(addr.wrapping_add(2))) << 16)
            | (u32::from(self.read_8(addr.wrapping_add(3))) << 24)
    }

    fn rom_offset(&self, addr: u32) -> Option<usize> {
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

        Some((addr - base) as usize)
    }
}

#[cfg(test)]
mod tests {
    use super::Cartridge;

    #[test]
    fn gamepak_windows_all_map_to_the_same_rom() {
        let cart = Cartridge::new(vec![0x78, 0x56, 0x34, 0x12]);

        assert_eq!(cart.read_32(0x0800_0000), 0x1234_5678);
        assert_eq!(cart.read_32(0x0a00_0000), 0x1234_5678);
        assert_eq!(cart.read_32(0x0c00_0000), 0x1234_5678);
    }
}
