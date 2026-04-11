//! Register banks for the ARM7TDMI programmer's model.

use crate::psr::Mode;

pub const SP: u8 = 13;
pub const LR: u8 = 14;
pub const PC: u8 = 15;

/// Physical register banks.
///
/// The current mode decides which physical register is visible for logical
/// register numbers R8-R14.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Registers {
    common: [u32; 16],
    fiq: [u32; 7],
    svc: [u32; 2],
    abt: [u32; 2],
    irq: [u32; 2],
    und: [u32; 2],
}

impl Registers {
    pub fn read(&self, mode: Mode, reg: u8) -> u32 {
        assert!(reg < 16, "register index out of range");

        match reg {
            0..=7 => self.common[reg as usize],
            8..=12 if matches!(mode, Mode::Fiq) => self.fiq[(reg - 8) as usize],
            8..=12 => self.common[reg as usize],
            13..=14 => match mode {
                Mode::Fiq => self.fiq[(reg - 8) as usize],
                Mode::Supervisor => self.svc[(reg - 13) as usize],
                Mode::Abort => self.abt[(reg - 13) as usize],
                Mode::Irq => self.irq[(reg - 13) as usize],
                Mode::Undefined => self.und[(reg - 13) as usize],
                Mode::User | Mode::System => self.common[reg as usize],
            },
            PC => self.common[PC as usize],
            _ => unreachable!(),
        }
    }

    pub fn write(&mut self, mode: Mode, reg: u8, value: u32) {
        assert!(reg < 16, "register index out of range");

        match reg {
            0..=7 => self.common[reg as usize] = value,
            8..=12 if matches!(mode, Mode::Fiq) => self.fiq[(reg - 8) as usize] = value,
            8..=12 => self.common[reg as usize] = value,
            13..=14 => match mode {
                Mode::Fiq => self.fiq[(reg - 8) as usize] = value,
                Mode::Supervisor => self.svc[(reg - 13) as usize] = value,
                Mode::Abort => self.abt[(reg - 13) as usize] = value,
                Mode::Irq => self.irq[(reg - 13) as usize] = value,
                Mode::Undefined => self.und[(reg - 13) as usize] = value,
                Mode::User | Mode::System => self.common[reg as usize] = value,
            },
            PC => self.common[PC as usize] = value,
            _ => unreachable!(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Registers, LR, SP};
    use crate::psr::Mode;

    #[test]
    fn user_and_system_share_the_same_register_bank() {
        let mut regs = Registers::default();
        regs.write(Mode::User, 0, 0x11);
        regs.write(Mode::User, SP, 0x1000);
        regs.write(Mode::User, LR, 0x2000);

        assert_eq!(regs.read(Mode::System, 0), 0x11);
        assert_eq!(regs.read(Mode::System, SP), 0x1000);
        assert_eq!(regs.read(Mode::System, LR), 0x2000);
    }

    #[test]
    fn irq_banks_sp_and_lr_but_shares_general_registers() {
        let mut regs = Registers::default();
        regs.write(Mode::User, 12, 0xaaaa_bbbb);
        regs.write(Mode::User, SP, 0x1000);
        regs.write(Mode::User, LR, 0x2000);

        regs.write(Mode::Irq, SP, 0x3000);
        regs.write(Mode::Irq, LR, 0x4000);

        assert_eq!(regs.read(Mode::Irq, 12), 0xaaaa_bbbb);
        assert_eq!(regs.read(Mode::Irq, SP), 0x3000);
        assert_eq!(regs.read(Mode::Irq, LR), 0x4000);
        assert_eq!(regs.read(Mode::User, SP), 0x1000);
        assert_eq!(regs.read(Mode::User, LR), 0x2000);
    }

    #[test]
    fn fiq_has_private_high_registers() {
        let mut regs = Registers::default();
        regs.write(Mode::User, 7, 0x07);
        regs.write(Mode::User, 8, 0x08);
        regs.write(Mode::User, 12, 0x0c);
        regs.write(Mode::User, SP, 0x1000);
        regs.write(Mode::User, LR, 0x2000);

        regs.write(Mode::Fiq, 8, 0x8000);
        regs.write(Mode::Fiq, 12, 0xc000);
        regs.write(Mode::Fiq, SP, 0xf000);
        regs.write(Mode::Fiq, LR, 0xf100);

        assert_eq!(regs.read(Mode::Fiq, 7), 0x07);
        assert_eq!(regs.read(Mode::Fiq, 8), 0x8000);
        assert_eq!(regs.read(Mode::Fiq, 12), 0xc000);
        assert_eq!(regs.read(Mode::Fiq, SP), 0xf000);
        assert_eq!(regs.read(Mode::Fiq, LR), 0xf100);

        assert_eq!(regs.read(Mode::User, 8), 0x08);
        assert_eq!(regs.read(Mode::User, 12), 0x0c);
        assert_eq!(regs.read(Mode::User, SP), 0x1000);
        assert_eq!(regs.read(Mode::User, LR), 0x2000);
    }
}
