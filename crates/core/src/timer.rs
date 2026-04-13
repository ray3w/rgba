use crate::io::{IoRegs, IRQ_TIMER0, IRQ_TIMER1, IRQ_TIMER2, IRQ_TIMER3};

const TIMER_ENABLE: u16 = 1 << 7;
const TIMER_IRQ_ENABLE: u16 = 1 << 6;
const TIMER_CASCADE: u16 = 1 << 2;

const TIMER_IRQS: [u16; 4] = [IRQ_TIMER0, IRQ_TIMER1, IRQ_TIMER2, IRQ_TIMER3];

/// Four hardware timers that can tick from CPU cycles or from the previous
/// timer's overflow.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Timers {
    accumulators: [u32; 4],
    enabled: [bool; 4],
}

impl Timers {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn step(&mut self, cycles: u32, io: &mut IoRegs) {
        for index in 0..4 {
            let control = io.timer_control(index);
            let enabled = (control & TIMER_ENABLE) != 0;

            if enabled && !self.enabled[index] {
                self.enabled[index] = true;
                self.accumulators[index] = 0;
                io.set_timer_counter(index, io.timer_reload(index));
            } else if !enabled {
                self.enabled[index] = false;
                self.accumulators[index] = 0;
                continue;
            }

            if (control & TIMER_CASCADE) != 0 {
                continue;
            }

            let divisor = prescaler(control);
            self.accumulators[index] = self.accumulators[index].wrapping_add(cycles);
            let ticks = self.accumulators[index] / divisor;
            self.accumulators[index] %= divisor;

            if ticks != 0 {
                self.increment_timer(io, index, ticks);
            }
        }
    }

    fn increment_timer(&mut self, io: &mut IoRegs, index: usize, ticks: u32) {
        if ticks == 0 || index >= 4 || !self.enabled[index] {
            return;
        }

        let control = io.timer_control(index);
        let mut counter = io.timer_counter(index);
        let mut remaining = ticks;

        while remaining > 0 {
            let until_overflow = 0x1_0000u32 - u32::from(counter);
            if remaining < until_overflow {
                counter = counter.wrapping_add(remaining as u16);
                io.set_timer_counter(index, counter);
                break;
            }

            remaining -= until_overflow;
            counter = io.timer_reload(index);
            io.set_timer_counter(index, counter);

            if (control & TIMER_IRQ_ENABLE) != 0 {
                io.request_interrupt(TIMER_IRQS[index]);
            }

            self.increment_cascade(io, index + 1);
        }
    }

    fn increment_cascade(&mut self, io: &mut IoRegs, index: usize) {
        if index >= 4 || !self.enabled[index] {
            return;
        }

        let control = io.timer_control(index);
        if (control & TIMER_CASCADE) == 0 {
            return;
        }

        self.increment_timer(io, index, 1);
    }
}

fn prescaler(control: u16) -> u32 {
    match control & 0b11 {
        0 => 1,
        1 => 64,
        2 => 256,
        _ => 1024,
    }
}

#[cfg(test)]
mod tests {
    use super::Timers;
    use crate::io::{IoRegs, IRQ_TIMER0};

    #[test]
    fn enabling_timer_loads_reload_value() {
        let mut timers = Timers::new();
        let mut io = IoRegs::new();
        io.write_16(0x0400_0100, 0xff80);
        io.write_16(0x0400_0102, 0x0080);

        timers.step(1, &mut io);

        assert_eq!(io.timer_counter(0), 0xff81);
    }

    #[test]
    fn overflow_reloads_counter_and_requests_irq() {
        let mut timers = Timers::new();
        let mut io = IoRegs::new();
        io.write_16(0x0400_0100, 0xfffe);
        io.write_16(0x0400_0102, 0x00c0);

        timers.step(1, &mut io);
        assert_eq!(io.timer_counter(0), 0xffff);

        timers.step(1, &mut io);
        assert_eq!(io.timer_counter(0), 0xfffe);
        assert_ne!(io.if_() & IRQ_TIMER0, 0);
    }

    #[test]
    fn cascade_timer_ticks_on_previous_overflow() {
        let mut timers = Timers::new();
        let mut io = IoRegs::new();
        io.write_16(0x0400_0100, 0xffff);
        io.write_16(0x0400_0102, 0x0080);
        io.write_16(0x0400_0104, 0x1000);
        io.write_16(0x0400_0106, 0x0084);

        timers.step(1, &mut io);
        timers.step(1, &mut io);

        assert_eq!(io.timer_counter(1), 0x1001);
    }
}
