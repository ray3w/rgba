//! GBA hardware integration.
//!
//! Phase 2 introduces the first real machine skeleton around the CPU:
//! cartridge ROM, BIOS/WRAM storage, and a shared address bus that implements
//! `rgba_arm7tdmi::BusInterface`.

pub mod bus;
pub mod cartridge;
pub mod io;
pub mod mem;
pub mod scheduler;

pub use bus::Bus;
pub use cartridge::Cartridge;
pub use io::IoRegs;
pub use mem::{BiosLoadError, Memory};
pub use rgba_arm7tdmi as arm7tdmi;
pub use scheduler::{Event, EventKind, Scheduler};

use rgba_arm7tdmi::Arm7tdmi;

/// Minimal GBA core used to run the CPU against the real `core::Bus`.
#[derive(Debug, Clone)]
pub struct Gba {
    cpu: Arm7tdmi,
    bus: Bus,
    scheduler: Scheduler,
}

impl Gba {
    pub fn new(cartridge: Cartridge) -> Self {
        Self {
            cpu: Arm7tdmi::new(),
            bus: Bus::new(cartridge),
            scheduler: Scheduler::new(),
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

    pub fn scheduler(&self) -> &Scheduler {
        &self.scheduler
    }

    pub fn scheduler_mut(&mut self) -> &mut Scheduler {
        &mut self.scheduler
    }

    pub fn schedule_event(&mut self, fire_at: u64, kind: EventKind) {
        self.scheduler.schedule(fire_at, kind);
    }

    pub fn schedule_event_in(&mut self, delta: u64, kind: EventKind) {
        self.scheduler.schedule_in(delta, kind);
    }

    pub fn step(&mut self) -> u32 {
        let cycles = self.cpu.step(&mut self.bus);
        self.scheduler.advance(cycles);

        while let Some(event) = self.scheduler.pop_pending() {
            self.handle_event(event.kind);
        }

        cycles
    }

    fn handle_event(&mut self, kind: EventKind) {
        match kind {
            EventKind::HBlank => self.bus.io_mut().set_hblank(true),
            EventKind::VBlank => self.bus.io_mut().set_vblank(true),
            EventKind::TimerOverflow(index) => {
                if index < 4 {
                    self.bus.io_mut().request_interrupt(io::IRQ_TIMER0 << index);
                }
            }
            EventKind::DmaComplete(index) => {
                if index < 4 {
                    self.bus.io_mut().request_interrupt(io::IRQ_DMA0 << index);
                }
            }
        }
    }
}
