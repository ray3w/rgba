//! Exception metadata and CPU exception-entry helpers.

use crate::{psr::Mode, Arm7tdmi, LR, PC};

/// Exceptions recognized by the ARM7TDMI core.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Exception {
    Reset,
    UndefinedInstruction,
    SoftwareInterrupt,
    PrefetchAbort,
    DataAbort,
    Irq,
    Fiq,
}

impl Exception {
    pub const fn vector_address(self) -> u32 {
        match self {
            Self::Reset => 0x0000_0000,
            Self::UndefinedInstruction => 0x0000_0004,
            Self::SoftwareInterrupt => 0x0000_0008,
            Self::PrefetchAbort => 0x0000_000c,
            Self::DataAbort => 0x0000_0010,
            Self::Irq => 0x0000_0018,
            Self::Fiq => 0x0000_001c,
        }
    }

    pub const fn entry_mode(self) -> Mode {
        match self {
            Self::Reset | Self::SoftwareInterrupt => Mode::Supervisor,
            Self::UndefinedInstruction => Mode::Undefined,
            Self::PrefetchAbort | Self::DataAbort => Mode::Abort,
            Self::Irq => Mode::Irq,
            Self::Fiq => Mode::Fiq,
        }
    }

    pub const fn sets_irq_disable(self) -> bool {
        true
    }

    pub const fn sets_fiq_disable(self) -> bool {
        matches!(self, Self::Reset | Self::Fiq)
    }

    /// The amount a typical exception return sequence subtracts from LR.
    ///
    /// This is useful later when instruction execution lands. For example,
    /// `SUBS PC, LR_irq, #4` returns from an IRQ.
    pub const fn return_subtract(self) -> Option<u32> {
        match self {
            Self::Reset => None,
            Self::SoftwareInterrupt | Self::UndefinedInstruction => Some(0),
            Self::PrefetchAbort | Self::Irq | Self::Fiq => Some(4),
            Self::DataAbort => Some(8),
        }
    }
}

impl Arm7tdmi {
    /// Enters an exception with an explicitly supplied architectural return
    /// address.
    ///
    /// The caller will later decide how to derive that return address from the
    /// current pipeline state. Phase 1a only models the state transition itself.
    pub fn enter_exception(&mut self, exception: Exception, return_address: u32) {
        if matches!(exception, Exception::Reset) {
            self.cpsr.set_mode(exception.entry_mode());
            self.cpsr.set_irq_disabled(exception.sets_irq_disable());
            self.cpsr.set_fiq_disabled(exception.sets_fiq_disable());
            self.cpsr.set_thumb(false);
            self.regs
                .write(Mode::Supervisor, PC, exception.vector_address());
            return;
        }

        let old_cpsr = self.cpsr;
        let target_mode = exception.entry_mode();

        self.spsrs
            .get_mut(target_mode)
            .expect("exception mode must have an SPSR")
            .clone_from(&old_cpsr);
        self.regs.write(target_mode, LR, return_address);

        self.cpsr.set_mode(target_mode);
        self.cpsr.set_irq_disabled(exception.sets_irq_disable());
        if exception.sets_fiq_disable() {
            self.cpsr.set_fiq_disabled(true);
        }
        self.cpsr.set_thumb(false);
        self.regs.write(target_mode, PC, exception.vector_address());
    }
}

#[cfg(test)]
mod tests {
    use super::Exception;
    use crate::{Arm7tdmi, Mode, Psr, LR, PC};

    #[test]
    fn irq_entry_saves_cpsr_and_switches_mode() {
        let mut cpu = Arm7tdmi::new();
        let mut cpsr = Psr::new(Mode::User);
        cpsr.set_thumb(true);
        cpsr.set_negative(true);
        cpu.set_cpsr(cpsr);
        cpu.set_pc(0x0800_0100);

        cpu.enter_exception(Exception::Irq, 0x0800_0104);

        assert_eq!(cpu.mode(), Mode::Irq);
        assert!(!cpu.is_thumb());
        assert!(cpu.cpsr().irq_disabled());
        assert_eq!(cpu.spsr(Mode::Irq), Some(cpsr));
        assert_eq!(cpu.read_reg_for_mode(Mode::Irq, LR), 0x0800_0104);
        assert_eq!(cpu.read_reg_for_mode(Mode::Irq, PC), 0x0000_0018);
    }

    #[test]
    fn fiq_entry_uses_fiq_bank_and_masks_fiqs() {
        let mut cpu = Arm7tdmi::new();
        cpu.write_reg(8, 0x1111);
        cpu.write_reg(12, 0x2222);

        cpu.enter_exception(Exception::Fiq, 0x0800_0004);
        cpu.write_reg(8, 0xaaaa);
        cpu.write_reg(12, 0xbbbb);

        assert_eq!(cpu.mode(), Mode::Fiq);
        assert!(cpu.cpsr().irq_disabled());
        assert!(cpu.cpsr().fiq_disabled());
        assert_eq!(cpu.read_reg(8), 0xaaaa);
        assert_eq!(cpu.read_reg(12), 0xbbbb);
        assert_eq!(cpu.read_reg_for_mode(Mode::User, 8), 0x1111);
        assert_eq!(cpu.read_reg_for_mode(Mode::User, 12), 0x2222);
    }

    #[test]
    fn reset_forces_supervisor_arm_state_and_vectors_to_zero() {
        let mut cpu = Arm7tdmi::new();
        let mut cpsr = Psr::new(Mode::System);
        cpsr.set_thumb(true);
        cpu.set_cpsr(cpsr);

        cpu.enter_exception(Exception::Reset, 0);

        assert_eq!(cpu.mode(), Mode::Supervisor);
        assert!(!cpu.is_thumb());
        assert!(cpu.cpsr().irq_disabled());
        assert!(cpu.cpsr().fiq_disabled());
        assert_eq!(cpu.read_reg_for_mode(Mode::Supervisor, PC), 0x0000_0000);
    }
}
