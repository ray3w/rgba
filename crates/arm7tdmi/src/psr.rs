//! Program status registers and operating modes.

/// ARM7TDMI operating modes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Mode {
    User = 0b10000,
    Fiq = 0b10001,
    Irq = 0b10010,
    Supervisor = 0b10011,
    Abort = 0b10111,
    Undefined = 0b11011,
    System = 0b11111,
}

impl Mode {
    pub const fn bits(self) -> u8 {
        self as u8
    }

    pub const fn from_bits(bits: u8) -> Option<Self> {
        match bits {
            0b10000 => Some(Self::User),
            0b10001 => Some(Self::Fiq),
            0b10010 => Some(Self::Irq),
            0b10011 => Some(Self::Supervisor),
            0b10111 => Some(Self::Abort),
            0b11011 => Some(Self::Undefined),
            0b11111 => Some(Self::System),
            _ => None,
        }
    }

    pub const fn has_spsr(self) -> bool {
        !matches!(self, Self::User | Self::System)
    }
}

/// Program Status Register wrapper.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Psr(u32);

impl Default for Psr {
    fn default() -> Self {
        Self::new(Mode::User)
    }
}

impl Psr {
    const N_BIT: u32 = 1 << 31;
    const Z_BIT: u32 = 1 << 30;
    const C_BIT: u32 = 1 << 29;
    const V_BIT: u32 = 1 << 28;
    const I_BIT: u32 = 1 << 7;
    const F_BIT: u32 = 1 << 6;
    const T_BIT: u32 = 1 << 5;
    const MODE_MASK: u32 = 0x1f;

    pub const fn new(mode: Mode) -> Self {
        Self(mode.bits() as u32)
    }

    pub const fn from_bits(bits: u32) -> Self {
        Self(bits)
    }

    pub const fn bits(self) -> u32 {
        self.0
    }

    pub fn negative(self) -> bool {
        self.bit(Self::N_BIT)
    }

    pub fn zero(self) -> bool {
        self.bit(Self::Z_BIT)
    }

    pub fn carry(self) -> bool {
        self.bit(Self::C_BIT)
    }

    pub fn overflow(self) -> bool {
        self.bit(Self::V_BIT)
    }

    pub fn irq_disabled(self) -> bool {
        self.bit(Self::I_BIT)
    }

    pub fn fiq_disabled(self) -> bool {
        self.bit(Self::F_BIT)
    }

    pub fn thumb(self) -> bool {
        self.bit(Self::T_BIT)
    }

    pub fn mode(self) -> Mode {
        Mode::from_bits((self.0 & Self::MODE_MASK) as u8).expect("invalid PSR mode bits")
    }

    pub fn set_negative(&mut self, value: bool) {
        self.set_bit(Self::N_BIT, value);
    }

    pub fn set_zero(&mut self, value: bool) {
        self.set_bit(Self::Z_BIT, value);
    }

    pub fn set_carry(&mut self, value: bool) {
        self.set_bit(Self::C_BIT, value);
    }

    pub fn set_overflow(&mut self, value: bool) {
        self.set_bit(Self::V_BIT, value);
    }

    pub fn set_irq_disabled(&mut self, value: bool) {
        self.set_bit(Self::I_BIT, value);
    }

    pub fn set_fiq_disabled(&mut self, value: bool) {
        self.set_bit(Self::F_BIT, value);
    }

    pub fn set_thumb(&mut self, value: bool) {
        self.set_bit(Self::T_BIT, value);
    }

    pub fn set_mode(&mut self, mode: Mode) {
        self.0 = (self.0 & !Self::MODE_MASK) | mode.bits() as u32;
    }

    pub fn set_nz(&mut self, result: u32) {
        self.set_negative((result & 0x8000_0000) != 0);
        self.set_zero(result == 0);
    }

    pub fn set_nzcv(&mut self, result: u32, carry: bool, overflow: bool) {
        self.set_nz(result);
        self.set_carry(carry);
        self.set_overflow(overflow);
    }

    fn bit(self, mask: u32) -> bool {
        (self.0 & mask) != 0
    }

    fn set_bit(&mut self, mask: u32, value: bool) {
        if value {
            self.0 |= mask;
        } else {
            self.0 &= !mask;
        }
    }
}

/// Saved program status registers for exception modes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SavedProgramStatusRegisters {
    fiq: Psr,
    irq: Psr,
    svc: Psr,
    abt: Psr,
    und: Psr,
}

impl Default for SavedProgramStatusRegisters {
    fn default() -> Self {
        let initial = Psr::new(Mode::User);
        Self {
            fiq: initial,
            irq: initial,
            svc: initial,
            abt: initial,
            und: initial,
        }
    }
}

impl SavedProgramStatusRegisters {
    pub fn get(&self, mode: Mode) -> Option<Psr> {
        match mode {
            Mode::Fiq => Some(self.fiq),
            Mode::Irq => Some(self.irq),
            Mode::Supervisor => Some(self.svc),
            Mode::Abort => Some(self.abt),
            Mode::Undefined => Some(self.und),
            Mode::User | Mode::System => None,
        }
    }

    pub fn get_mut(&mut self, mode: Mode) -> Option<&mut Psr> {
        match mode {
            Mode::Fiq => Some(&mut self.fiq),
            Mode::Irq => Some(&mut self.irq),
            Mode::Supervisor => Some(&mut self.svc),
            Mode::Abort => Some(&mut self.abt),
            Mode::Undefined => Some(&mut self.und),
            Mode::User | Mode::System => None,
        }
    }

    pub fn set(&mut self, mode: Mode, value: Psr) -> bool {
        if let Some(slot) = self.get_mut(mode) {
            *slot = value;
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Mode, Psr, SavedProgramStatusRegisters};

    #[test]
    fn psr_tracks_flags_state_and_mode() {
        let mut psr = Psr::new(Mode::User);
        psr.set_negative(true);
        psr.set_zero(true);
        psr.set_carry(true);
        psr.set_overflow(false);
        psr.set_thumb(true);
        psr.set_irq_disabled(true);
        psr.set_mode(Mode::Irq);

        assert!(psr.negative());
        assert!(psr.zero());
        assert!(psr.carry());
        assert!(!psr.overflow());
        assert!(psr.thumb());
        assert!(psr.irq_disabled());
        assert_eq!(psr.mode(), Mode::Irq);
    }

    #[test]
    fn set_nzcv_updates_all_condition_flags() {
        let mut psr = Psr::new(Mode::Supervisor);
        psr.set_nzcv(0x8000_0000, true, true);

        assert!(psr.negative());
        assert!(!psr.zero());
        assert!(psr.carry());
        assert!(psr.overflow());
    }

    #[test]
    fn spsr_banks_exist_only_for_exception_modes() {
        let mut spsrs = SavedProgramStatusRegisters::default();
        let irq_psr = Psr::from_bits(0xf000_00d2);

        assert!(spsrs.set(Mode::Irq, irq_psr));
        assert_eq!(spsrs.get(Mode::Irq), Some(irq_psr));
        assert_eq!(spsrs.get(Mode::User), None);
        assert!(!spsrs.set(Mode::System, irq_psr));
    }
}
