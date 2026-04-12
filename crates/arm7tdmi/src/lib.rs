//! ARM7TDMI CPU emulation primitives.
//!
//! Phase 1a focuses on the CPU state model rather than instruction execution:
//! registers, PSRs, exception entry, and ALU helpers. The CPU remains
//! independent from the rest of the GBA hardware and talks to memory through
//! `BusInterface`.

pub mod alu;
pub mod arm;
pub mod exception;
pub mod psr;
pub mod reg;

pub use alu::{
    add_with_carry, shift_immediate, shift_register, ArithmeticResult, ShiftKind, ShiftResult,
};
pub use exception::Exception;
pub use psr::{Mode, Psr, SavedProgramStatusRegisters};
pub use reg::{Registers, LR, PC, SP};

/// The interface that the CPU uses to access memory.
/// Implemented by the GBA bus in `rgba-core`, or by a fake bus in tests.
pub trait BusInterface {
    fn read_8(&mut self, addr: u32) -> u8;
    fn read_16(&mut self, addr: u32) -> u16;
    fn read_32(&mut self, addr: u32) -> u32;
    fn write_8(&mut self, addr: u32, val: u8);
    fn write_16(&mut self, addr: u32, val: u16);
    fn write_32(&mut self, addr: u32, val: u32);
}

/// ARM7TDMI core state.
///
/// Phase 1a intentionally keeps this small and explicit:
/// - the architecturally visible register banks
/// - the current program status register
/// - saved PSRs for exception modes
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Arm7tdmi {
    regs: Registers,
    cpsr: Psr,
    spsrs: SavedProgramStatusRegisters,
}

impl Default for Arm7tdmi {
    fn default() -> Self {
        Self::new()
    }
}

impl Arm7tdmi {
    /// Creates a reset-like CPU state suitable for unit tests.
    ///
    /// The CPU starts in User mode, ARM state, with zeroed registers.
    pub fn new() -> Self {
        Self {
            regs: Registers::default(),
            cpsr: Psr::new(Mode::User),
            spsrs: SavedProgramStatusRegisters::default(),
        }
    }

    /// Phase 1b executes ARM-state instructions. Thumb-state execution lands in
    /// Phase 1c.
    pub fn step<B: BusInterface>(&mut self, bus: &mut B) -> u32 {
        assert!(!self.is_thumb(), "thumb-state execution lands in phase 1c");

        let fetch_pc = self.pc();
        let opcode = bus.read_32(fetch_pc);
        let outcome = arm::execute(self, bus, opcode, fetch_pc);

        if !outcome.wrote_pc {
            self.set_pc(fetch_pc.wrapping_add(4));
        }

        outcome.cycles
    }

    pub fn mode(&self) -> Mode {
        self.cpsr.mode()
    }

    pub fn set_mode(&mut self, mode: Mode) {
        self.cpsr.set_mode(mode);
    }

    pub fn is_thumb(&self) -> bool {
        self.cpsr.thumb()
    }

    pub fn set_thumb(&mut self, thumb: bool) {
        self.cpsr.set_thumb(thumb);
    }

    pub fn cpsr(&self) -> Psr {
        self.cpsr
    }

    pub fn set_cpsr(&mut self, psr: Psr) {
        self.cpsr = psr;
    }

    pub fn spsr(&self, mode: Mode) -> Option<Psr> {
        self.spsrs.get(mode)
    }

    pub fn set_spsr(&mut self, mode: Mode, value: Psr) -> bool {
        self.spsrs.set(mode, value)
    }

    pub fn registers(&self) -> &Registers {
        &self.regs
    }

    pub fn registers_mut(&mut self) -> &mut Registers {
        &mut self.regs
    }

    pub fn read_reg(&self, reg: u8) -> u32 {
        self.regs.read(self.mode(), reg)
    }

    pub fn write_reg(&mut self, reg: u8, value: u32) {
        self.regs.write(self.mode(), reg, value);
    }

    pub fn read_reg_for_mode(&self, mode: Mode, reg: u8) -> u32 {
        self.regs.read(mode, reg)
    }

    pub fn write_reg_for_mode(&mut self, mode: Mode, reg: u8, value: u32) {
        self.regs.write(mode, reg, value);
    }

    pub fn sp(&self) -> u32 {
        self.read_reg(SP)
    }

    pub fn set_sp(&mut self, value: u32) {
        self.write_reg(SP, value);
    }

    pub fn lr(&self) -> u32 {
        self.read_reg(LR)
    }

    pub fn set_lr(&mut self, value: u32) {
        self.write_reg(LR, value);
    }

    pub fn pc(&self) -> u32 {
        self.read_reg(PC)
    }

    pub fn set_pc(&mut self, value: u32) {
        self.write_reg(PC, value);
    }
}
