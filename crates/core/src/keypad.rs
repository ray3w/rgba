use crate::io::{IoRegs, IRQ_KEYPAD};

const KEYCNT_IRQ_ENABLE: u16 = 1 << 14;
const KEYCNT_AND_CONDITION: u16 = 1 << 15;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum Button {
    A = 1 << 0,
    B = 1 << 1,
    Select = 1 << 2,
    Start = 1 << 3,
    Right = 1 << 4,
    Left = 1 << 5,
    Up = 1 << 6,
    Down = 1 << 7,
    R = 1 << 8,
    L = 1 << 9,
}

impl Button {
    pub const fn mask(self) -> u16 {
        self as u16
    }
}

/// Host input state mirrored into the GBA KEYINPUT/KEYCNT registers.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Keypad {
    pressed: u16,
    irq_condition_active: bool,
}

impl Keypad {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_pressed(&mut self, button: Button, pressed: bool) {
        if pressed {
            self.pressed |= button.mask();
        } else {
            self.pressed &= !button.mask();
        }
    }

    pub fn keyinput(&self) -> u16 {
        (!self.pressed) & 0x03ff
    }

    pub fn sync_to_io(&mut self, io: &mut IoRegs) {
        io.set_keyinput(self.keyinput());

        let keycnt = io.keycnt();
        let irq_enabled = (keycnt & KEYCNT_IRQ_ENABLE) != 0;
        let watched = keycnt & 0x03ff;

        let condition = if irq_enabled && watched != 0 {
            if (keycnt & KEYCNT_AND_CONDITION) != 0 {
                (self.pressed & watched) == watched
            } else {
                (self.pressed & watched) != 0
            }
        } else {
            false
        };

        if condition && !self.irq_condition_active {
            io.request_interrupt(IRQ_KEYPAD);
        }

        self.irq_condition_active = condition;
    }
}

#[cfg(test)]
mod tests {
    use super::{Button, Keypad};
    use crate::io::{IoRegs, IRQ_KEYPAD};

    #[test]
    fn keyinput_is_low_active() {
        let mut keypad = Keypad::new();
        let mut io = IoRegs::new();

        keypad.set_pressed(Button::A, true);
        keypad.sync_to_io(&mut io);
        assert_eq!(io.keyinput() & Button::A.mask(), 0);

        keypad.set_pressed(Button::A, false);
        keypad.sync_to_io(&mut io);
        assert_ne!(io.keyinput() & Button::A.mask(), 0);
    }

    #[test]
    fn keycnt_can_request_interrupt_once_per_new_match() {
        let mut keypad = Keypad::new();
        let mut io = IoRegs::new();
        io.write_16(0x0400_0132, (1 << 14) | Button::Start.mask());

        keypad.sync_to_io(&mut io);
        assert_eq!(io.if_() & IRQ_KEYPAD, 0);

        keypad.set_pressed(Button::Start, true);
        keypad.sync_to_io(&mut io);
        assert_ne!(io.if_() & IRQ_KEYPAD, 0);

        let flags = io.if_();
        keypad.sync_to_io(&mut io);
        assert_eq!(io.if_(), flags);
    }
}
