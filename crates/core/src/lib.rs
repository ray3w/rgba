//! GBA hardware integration.
//!
//! Phase 2 introduces the first real machine skeleton around the CPU:
//! cartridge ROM, BIOS/WRAM storage, and a shared address bus that implements
//! `rgba_arm7tdmi::BusInterface`.

pub mod bus;
pub mod cartridge;
pub mod mem;

pub use bus::Bus;
pub use cartridge::Cartridge;
pub use mem::{BiosLoadError, Memory};
pub use rgba_arm7tdmi as arm7tdmi;

use rgba_arm7tdmi::Arm7tdmi;

/// Minimal GBA core used to run the CPU against the real `core::Bus`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Gba {
    cpu: Arm7tdmi,
    bus: Bus,
}

impl Gba {
    pub fn new(cartridge: Cartridge) -> Self {
        Self {
            cpu: Arm7tdmi::new(),
            bus: Bus::new(cartridge),
        }
    }

    pub fn with_rom(rom: Vec<u8>) -> Self {
        Self::new(Cartridge::new(rom))
    }

    pub fn cpu(&self) -> &Arm7tdmi {
        &self.cpu
    }

    pub fn cpu_mut(&mut self) -> &mut Arm7tdmi {
        &mut self.cpu
    }

    pub fn load_bios(&mut self, bios: &[u8]) -> Result<(), BiosLoadError> {
        self.bus.load_bios(bios)
    }

    pub fn bus(&self) -> &Bus {
        &self.bus
    }

    pub fn bus_mut(&mut self) -> &mut Bus {
        &mut self.bus
    }

    pub fn step(&mut self) -> u32 {
        self.cpu.step(&mut self.bus)
    }
}
