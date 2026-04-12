use std::collections::VecDeque;
use std::fmt::Write;

use rgba_arm7tdmi::Arm7tdmi;

use super::test_bus::TestBus;

#[derive(Debug, Clone)]
pub struct TraceEntry {
    step: u64,
    pc: u32,
    opcode: u32,
    thumb: bool,
    cpsr: u32,
    regs: [u32; 16],
}

impl TraceEntry {
    pub fn capture(step: u64, cpu: &Arm7tdmi, bus: &TestBus) -> Self {
        let pc = cpu.pc();
        let opcode = if cpu.is_thumb() {
            u32::from(bus.read_rom_word(pc & !1) as u16)
        } else {
            bus.read_rom_word(pc)
        };

        let mut regs = [0; 16];
        for reg in 0..16u8 {
            regs[reg as usize] = cpu.read_reg(reg);
        }

        Self {
            step,
            pc,
            opcode,
            thumb: cpu.is_thumb(),
            cpsr: cpu.cpsr().bits(),
            regs,
        }
    }

    fn format_line(&self) -> String {
        let mut line = String::new();
        let _ = write!(
            line,
            "#{:06} {} pc={:08x} op={:08x} cpsr={:08x}",
            self.step,
            if self.thumb { "T" } else { "A" },
            self.pc,
            self.opcode,
            self.cpsr
        );

        for (idx, value) in self.regs.iter().enumerate() {
            let _ = write!(line, " r{}={:08x}", idx, value);
        }

        line
    }
}

#[derive(Debug, Default)]
pub struct RecentTrace {
    entries: VecDeque<TraceEntry>,
    capacity: usize,
}

impl RecentTrace {
    pub fn new(capacity: usize) -> Self {
        Self {
            entries: VecDeque::with_capacity(capacity),
            capacity,
        }
    }

    pub fn push(&mut self, entry: TraceEntry) {
        if self.entries.len() == self.capacity {
            self.entries.pop_front();
        }
        self.entries.push_back(entry);
    }

    pub fn render(&self) -> String {
        let mut out = String::new();
        for entry in &self.entries {
            let _ = writeln!(out, "{}", entry.format_line());
        }
        out
    }
}
