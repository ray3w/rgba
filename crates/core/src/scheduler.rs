use std::cmp::Reverse;
use std::collections::BinaryHeap;

/// Coarsely scheduled hardware events that fire at a future cycle count.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum EventKind {
    HBlank,
    VBlank,
    TimerOverflow(usize),
    DmaComplete(usize),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Event {
    pub fire_at: u64,
    pub kind: EventKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ScheduledEvent {
    fire_at: u64,
    seq: u64,
    kind: EventKind,
}

impl Ord for ScheduledEvent {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.fire_at
            .cmp(&other.fire_at)
            .then_with(|| self.seq.cmp(&other.seq))
    }
}

impl PartialOrd for ScheduledEvent {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

/// Global cycle scheduler used by the GBA core.
#[derive(Debug, Clone, Default)]
pub struct Scheduler {
    timestamp: u64,
    next_seq: u64,
    events: BinaryHeap<Reverse<ScheduledEvent>>,
}

impl Scheduler {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn timestamp(&self) -> u64 {
        self.timestamp
    }

    pub fn schedule(&mut self, fire_at: u64, kind: EventKind) {
        let event = ScheduledEvent {
            fire_at,
            seq: self.next_seq,
            kind,
        };
        self.next_seq = self.next_seq.wrapping_add(1);
        self.events.push(Reverse(event));
    }

    pub fn schedule_in(&mut self, delta: u64, kind: EventKind) {
        self.schedule(self.timestamp.wrapping_add(delta), kind);
    }

    pub fn advance(&mut self, cycles: u32) {
        self.timestamp = self.timestamp.wrapping_add(u64::from(cycles));
    }

    pub fn peek_pending_time(&self) -> Option<u64> {
        self.events.peek().map(|entry| entry.0.fire_at)
    }

    pub fn pop_pending(&mut self) -> Option<Event> {
        let pending = self.events.peek()?.0;
        if pending.fire_at > self.timestamp {
            return None;
        }

        let event = self.events.pop()?.0;
        Some(Event {
            fire_at: event.fire_at,
            kind: event.kind,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{EventKind, Scheduler};

    #[test]
    fn events_pop_in_fire_order() {
        let mut scheduler = Scheduler::new();
        scheduler.schedule(10, EventKind::VBlank);
        scheduler.schedule(4, EventKind::HBlank);
        scheduler.schedule(7, EventKind::TimerOverflow(0));
        scheduler.advance(10);

        assert_eq!(
            scheduler.pop_pending().map(|event| event.kind),
            Some(EventKind::HBlank)
        );
        assert_eq!(
            scheduler.pop_pending().map(|event| event.kind),
            Some(EventKind::TimerOverflow(0))
        );
        assert_eq!(
            scheduler.pop_pending().map(|event| event.kind),
            Some(EventKind::VBlank)
        );
    }

    #[test]
    fn same_cycle_events_preserve_schedule_order() {
        let mut scheduler = Scheduler::new();
        scheduler.schedule(3, EventKind::HBlank);
        scheduler.schedule(3, EventKind::VBlank);
        scheduler.advance(3);

        assert_eq!(
            scheduler.pop_pending().map(|event| event.kind),
            Some(EventKind::HBlank)
        );
        assert_eq!(
            scheduler.pop_pending().map(|event| event.kind),
            Some(EventKind::VBlank)
        );
    }

    #[test]
    fn events_do_not_fire_early() {
        let mut scheduler = Scheduler::new();
        scheduler.schedule_in(5, EventKind::DmaComplete(0));
        scheduler.advance(4);

        assert_eq!(scheduler.pop_pending(), None);

        scheduler.advance(1);
        assert_eq!(
            scheduler.pop_pending().map(|event| event.kind),
            Some(EventKind::DmaComplete(0))
        );
    }
}
