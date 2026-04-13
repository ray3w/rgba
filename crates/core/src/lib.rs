//! GBA hardware integration.
//!
//! The `rgba-core` crate assembles the CPU, bus, scheduler, MMIO block, and
//! early PPU into the first real machine-level execution path.

pub mod bus;
pub mod cartridge;
pub mod dma;
pub mod interrupt;
pub mod io;
pub mod keypad;
pub mod mem;
pub mod ppu;
pub mod scheduler;
pub mod timer;

pub use bus::Bus;
pub use cartridge::Cartridge;
pub use dma::DmaController;
pub use io::IoRegs;
pub use keypad::{Button, Keypad};
pub use mem::{BiosLoadError, Memory};
pub use ppu::{rgb555_to_xrgb8888, Ppu, FRAME_PIXELS, SCREEN_HEIGHT, SCREEN_WIDTH};
pub use rgba_arm7tdmi as arm7tdmi;
pub use scheduler::{Event, EventKind, Scheduler};
pub use timer::Timers;

use rgba_arm7tdmi::{Arm7tdmi, Mode, SP};

const USER_STACK_TOP: u32 = 0x0300_7f00;
const IRQ_STACK_TOP: u32 = 0x0300_7fa0;
const SUPERVISOR_STACK_TOP: u32 = 0x0300_7fe0;

/// Minimal GBA core used to run the CPU against the real `core::Bus`.
#[derive(Debug, Clone)]
pub struct Gba {
    cpu: Arm7tdmi,
    bus: Bus,
    ppu: Ppu,
    scheduler: Scheduler,
    timers: Timers,
    dma: DmaController,
    keypad: Keypad,
}

impl Gba {
    pub fn new(cartridge: Cartridge) -> Self {
        let mut gba = Self {
            cpu: Arm7tdmi::new(),
            bus: Bus::new(cartridge),
            ppu: Ppu::new(),
            scheduler: Scheduler::new(),
            timers: Timers::new(),
            dma: DmaController::new(),
            keypad: Keypad::new(),
        };
        gba.cpu.write_reg_for_mode(Mode::User, SP, USER_STACK_TOP);
        gba.cpu.write_reg_for_mode(Mode::Irq, SP, IRQ_STACK_TOP);
        gba.cpu
            .write_reg_for_mode(Mode::Supervisor, SP, SUPERVISOR_STACK_TOP);
        gba.bus.io_mut().set_vcount(0);
        gba.keypad.sync_to_io(gba.bus.io_mut());
        gba
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

    pub fn ppu(&self) -> &Ppu {
        &self.ppu
    }

    pub fn ppu_mut(&mut self) -> &mut Ppu {
        &mut self.ppu
    }

    pub fn timers(&self) -> &Timers {
        &self.timers
    }

    pub fn dma(&self) -> &DmaController {
        &self.dma
    }

    pub fn keypad(&self) -> &Keypad {
        &self.keypad
    }

    pub fn set_button_pressed(&mut self, button: Button, pressed: bool) {
        self.keypad.set_pressed(button, pressed);
        self.keypad.sync_to_io(self.bus.io_mut());
    }

    pub fn step(&mut self) -> u32 {
        let cycles = if interrupt::service_irq(&mut self.cpu, self.bus.io()) {
            2
        } else {
            self.cpu.step(&mut self.bus)
        };

        let previous_dispstat = self.bus.io().dispstat();
        self.scheduler.advance(cycles);
        let ppu = &mut self.ppu;
        self.bus
            .with_ppu_state(|io, vram, palette, _oam| ppu.step(cycles, io, vram, palette));
        let current_dispstat = self.bus.io().dispstat();
        let entered_hblank = (previous_dispstat & 0x0002) == 0 && (current_dispstat & 0x0002) != 0;
        let entered_vblank = (previous_dispstat & 0x0001) == 0 && (current_dispstat & 0x0001) != 0;

        self.timers.step(cycles, self.bus.io_mut());
        self.dma
            .service(&mut self.bus, entered_hblank, entered_vblank);
        self.keypad.sync_to_io(self.bus.io_mut());

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
