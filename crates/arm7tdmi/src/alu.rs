//! ALU and barrel-shifter helpers shared by ARM and Thumb instructions.

/// ARM barrel-shifter operation kinds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShiftKind {
    Lsl,
    Lsr,
    Asr,
    Ror,
}

/// Result of a shift/rotate operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShiftResult {
    pub value: u32,
    pub carry_out: bool,
}

/// Result of add/sub-style arithmetic.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ArithmeticResult {
    pub value: u32,
    pub carry_out: bool,
    pub overflow: bool,
}

pub fn add_with_carry(lhs: u32, rhs: u32, carry_in: bool) -> ArithmeticResult {
    let carry = u64::from(carry_in);
    let unsigned_sum = lhs as u64 + rhs as u64 + carry;
    let value = unsigned_sum as u32;
    let carry_out = unsigned_sum > u32::MAX as u64;

    let signed_sum = lhs as i32 as i64 + rhs as i32 as i64 + carry as i64;
    let overflow = signed_sum < i32::MIN as i64 || signed_sum > i32::MAX as i64;

    ArithmeticResult {
        value,
        carry_out,
        overflow,
    }
}

pub fn shift_immediate(kind: ShiftKind, value: u32, amount: u8, carry_in: bool) -> ShiftResult {
    match kind {
        ShiftKind::Lsl => lsl(value, amount, carry_in),
        ShiftKind::Lsr => lsr_immediate(value, amount),
        ShiftKind::Asr => asr_immediate(value, amount),
        ShiftKind::Ror => ror_immediate(value, amount, carry_in),
    }
}

pub fn shift_register(kind: ShiftKind, value: u32, amount: u8, carry_in: bool) -> ShiftResult {
    if amount == 0 {
        return ShiftResult {
            value,
            carry_out: carry_in,
        };
    }

    match kind {
        ShiftKind::Lsl => lsl(value, amount, carry_in),
        ShiftKind::Lsr => lsr_register(value, amount),
        ShiftKind::Asr => asr_register(value, amount),
        ShiftKind::Ror => ror_register(value, amount),
    }
}

fn lsl(value: u32, amount: u8, carry_in: bool) -> ShiftResult {
    match amount {
        0 => ShiftResult {
            value,
            carry_out: carry_in,
        },
        1..=31 => ShiftResult {
            value: value << amount,
            carry_out: bit(value, 32 - amount),
        },
        32 => ShiftResult {
            value: 0,
            carry_out: bit(value, 0),
        },
        _ => ShiftResult {
            value: 0,
            carry_out: false,
        },
    }
}

fn lsr_immediate(value: u32, amount: u8) -> ShiftResult {
    if amount == 0 {
        ShiftResult {
            value: 0,
            carry_out: bit(value, 31),
        }
    } else {
        lsr_register(value, amount)
    }
}

fn lsr_register(value: u32, amount: u8) -> ShiftResult {
    match amount {
        1..=31 => ShiftResult {
            value: value >> amount,
            carry_out: bit(value, amount - 1),
        },
        32 => ShiftResult {
            value: 0,
            carry_out: bit(value, 31),
        },
        _ => ShiftResult {
            value: 0,
            carry_out: false,
        },
    }
}

fn asr_immediate(value: u32, amount: u8) -> ShiftResult {
    if amount == 0 {
        ShiftResult {
            value: sign_fill(value),
            carry_out: bit(value, 31),
        }
    } else {
        asr_register(value, amount)
    }
}

fn asr_register(value: u32, amount: u8) -> ShiftResult {
    match amount {
        1..=31 => ShiftResult {
            value: ((value as i32) >> amount) as u32,
            carry_out: bit(value, amount - 1),
        },
        _ => ShiftResult {
            value: sign_fill(value),
            carry_out: bit(value, 31),
        },
    }
}

fn ror_immediate(value: u32, amount: u8, carry_in: bool) -> ShiftResult {
    if amount == 0 {
        ShiftResult {
            value: ((carry_in as u32) << 31) | (value >> 1),
            carry_out: bit(value, 0),
        }
    } else {
        ror_register(value, amount)
    }
}

fn ror_register(value: u32, amount: u8) -> ShiftResult {
    let rotate = amount & 0x1f;
    if rotate == 0 {
        ShiftResult {
            value,
            carry_out: bit(value, 31),
        }
    } else {
        ShiftResult {
            value: value.rotate_right(rotate as u32),
            carry_out: bit(value, rotate - 1),
        }
    }
}

fn sign_fill(value: u32) -> u32 {
    if bit(value, 31) {
        u32::MAX
    } else {
        0
    }
}

fn bit(value: u32, index: u8) -> bool {
    ((value >> index) & 1) != 0
}

#[cfg(test)]
mod tests {
    use super::{add_with_carry, shift_immediate, shift_register, ShiftKind};

    #[test]
    fn add_with_carry_reports_unsigned_carry() {
        let result = add_with_carry(0xffff_ffff, 1, false);
        assert_eq!(result.value, 0);
        assert!(result.carry_out);
        assert!(!result.overflow);
    }

    #[test]
    fn add_with_carry_reports_signed_overflow() {
        let result = add_with_carry(0x7fff_ffff, 1, false);
        assert_eq!(result.value, 0x8000_0000);
        assert!(!result.carry_out);
        assert!(result.overflow);
    }

    #[test]
    fn subtraction_can_be_expressed_via_add_with_carry() {
        let no_borrow = add_with_carry(3, !1, true);
        let with_borrow = add_with_carry(1, !3, true);

        assert_eq!(no_borrow.value, 2);
        assert!(no_borrow.carry_out);
        assert_eq!(with_borrow.value, 0xffff_fffe);
        assert!(!with_borrow.carry_out);
    }

    #[test]
    fn immediate_lsr_zero_means_shift_by_32() {
        let result = shift_immediate(ShiftKind::Lsr, 0x8000_0001, 0, false);
        assert_eq!(result.value, 0);
        assert!(result.carry_out);
    }

    #[test]
    fn immediate_asr_zero_sign_extends() {
        let result = shift_immediate(ShiftKind::Asr, 0x8000_0000, 0, false);
        assert_eq!(result.value, 0xffff_ffff);
        assert!(result.carry_out);
    }

    #[test]
    fn immediate_ror_zero_becomes_rrx() {
        let result = shift_immediate(ShiftKind::Ror, 0x0000_0003, 0, true);
        assert_eq!(result.value, 0x8000_0001);
        assert!(result.carry_out);
    }

    #[test]
    fn register_ror_32_keeps_value_and_uses_bit31_as_carry() {
        let result = shift_register(ShiftKind::Ror, 0x8000_0001, 32, false);
        assert_eq!(result.value, 0x8000_0001);
        assert!(result.carry_out);
    }

    #[test]
    fn register_zero_shift_preserves_carry_in() {
        let result = shift_register(ShiftKind::Lsl, 0x1234_5678, 0, true);
        assert_eq!(result.value, 0x1234_5678);
        assert!(result.carry_out);
    }
}
