use std::fs;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::{Path, PathBuf};

use rgba_arm7tdmi::{Arm7tdmi, BusInterface, Mode, Psr};

use super::test_bus::TestBus;
use super::trace::{RecentTrace, TraceEntry};

const ROM_ENTRY: u32 = 0x0800_0000;
const SYS_SP: u32 = 0x0300_7f00;
const SVC_SP: u32 = 0x0300_7fe0;
const IRQ_SP: u32 = 0x0300_7fa0;
const MAX_STEPS: u64 = 2_000_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RomKind {
    Arm,
    Thumb,
    Memory,
}

impl RomKind {
    fn result_register(self) -> u8 {
        match self {
            Self::Arm | Self::Memory => 12,
            Self::Thumb => 7,
        }
    }

    fn rom_path(self) -> &'static str {
        match self {
            Self::Arm => "arm.gba",
            Self::Thumb => "thumb.gba",
            Self::Memory => "memory.gba",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RomOutcome {
    pub kind: RomKind,
    pub steps: u64,
    pub result: u32,
}

pub fn run_gba_test(kind: RomKind) -> Result<RomOutcome, String> {
    let rom_path = workspace_rom_path(kind);
    let rom = fs::read(&rom_path)
        .map_err(|err| format!("failed to read {}: {err}", rom_path.display()))?;

    let mut cpu = seeded_cpu();
    let mut bus = TestBus::new(rom);
    let mut recent = RecentTrace::new(64);

    for step in 0..MAX_STEPS {
        if is_idle_loop(&cpu, &bus) {
            let result = cpu.read_reg(kind.result_register());
            if result == 0 {
                return Ok(RomOutcome {
                    kind,
                    steps: step,
                    result,
                });
            }

            return Err(format!(
                "{} failed at test {} after {} steps\n\nRecent trace:\n{}",
                kind.rom_path(),
                result,
                step,
                recent.render()
            ));
        }

        if handle_bios_hle(&mut cpu, &mut bus)
            .map_err(|err| format!("{err}\n\nRecent trace:\n{}", recent.render()))?
        {
            continue;
        }

        recent.push(TraceEntry::capture(step, &cpu, &bus));

        let step_result = catch_unwind(AssertUnwindSafe(|| cpu.step(&mut bus)));
        if let Err(payload) = step_result {
            let reason = if let Some(msg) = payload.downcast_ref::<&str>() {
                (*msg).to_owned()
            } else if let Some(msg) = payload.downcast_ref::<String>() {
                msg.clone()
            } else {
                "unknown panic payload".to_owned()
            };

            return Err(format!(
                "{} panicked after {} steps: {}\n\nRecent trace:\n{}",
                kind.rom_path(),
                step,
                reason,
                recent.render()
            ));
        }
    }

    Err(format!(
        "{} exceeded the step limit ({MAX_STEPS})\n\nRecent trace:\n{}",
        kind.rom_path(),
        recent.render()
    ))
}

fn workspace_rom_path(kind: RomKind) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("roms")
        .join("gba-tests")
        .join(kind.rom_path())
}

fn seeded_cpu() -> Arm7tdmi {
    let mut cpu = Arm7tdmi::new();
    let mut cpsr = Psr::new(Mode::System);
    cpsr.set_irq_disabled(true);
    cpsr.set_fiq_disabled(true);
    cpu.set_cpsr(cpsr);
    cpu.set_pc(ROM_ENTRY);
    cpu.write_reg_for_mode(Mode::System, 13, SYS_SP);
    cpu.write_reg_for_mode(Mode::Supervisor, 13, SVC_SP);
    cpu.write_reg_for_mode(Mode::Irq, 13, IRQ_SP);
    cpu
}

fn is_idle_loop(cpu: &Arm7tdmi, bus: &TestBus) -> bool {
    if cpu.is_thumb() {
        return false;
    }

    let pc = cpu.pc();
    let opcode = bus.read_rom_word(pc);
    if (opcode >> 28) != 0xe {
        return false;
    }
    if (opcode & 0x0f00_0000) != 0x0a00_0000 {
        return false;
    }

    let offset = (((opcode & 0x00ff_ffff) << 2) as i32) << 6 >> 6;
    let target = pc.wrapping_add(8).wrapping_add(offset as u32);
    target == pc
}

fn handle_bios_hle(cpu: &mut Arm7tdmi, bus: &mut TestBus) -> Result<bool, String> {
    if cpu.mode() != Mode::Supervisor || cpu.pc() != 0x0000_0008 {
        return Ok(false);
    }

    let saved = cpu
        .spsr(Mode::Supervisor)
        .ok_or("SWI entered supervisor mode without an SPSR")?;
    let lr = cpu.lr();
    let swi_addr = if saved.thumb() {
        lr.wrapping_sub(2)
    } else {
        lr.wrapping_sub(4)
    };

    let function = if saved.thumb() {
        let opcode = bus.read_16(swi_addr);
        u32::from(opcode & 0x00ff)
    } else {
        let opcode = bus.read_32(swi_addr);
        (opcode >> 16) & 0xff
    };

    match function {
        0x06 => {
            let numerator = cpu.read_reg(0) as i32;
            let denominator = cpu.read_reg(1) as i32;
            if denominator == 0 {
                return Err("BIOS Div SWI called with a zero denominator".to_owned());
            }

            let quotient = numerator / denominator;
            let remainder = numerator % denominator;
            cpu.write_reg(0, quotient as u32);
            cpu.write_reg(1, remainder as u32);
            cpu.write_reg(3, quotient.wrapping_abs() as u32);
        }
        _ => {
            return Err(format!("unsupported BIOS SWI 0x{function:02x}"));
        }
    }

    cpu.set_cpsr(saved);
    cpu.set_pc(if saved.thumb() { lr & !1 } else { lr & !3 });
    Ok(true)
}
