use rgba_arm7tdmi::{Arm7tdmi, BusInterface, Mode, Psr, LR};

use crate::Bus;

const SWI_SOFT_RESET: u8 = 0x00;
const SWI_REGISTER_RAM_RESET: u8 = 0x01;
const SWI_INTR_WAIT: u8 = 0x04;
const SWI_VBLANK_INTR_WAIT: u8 = 0x05;
const SWI_DIV: u8 = 0x06;

const IF_ADDR: u32 = 0x0400_0202;
const REG_IFBIOS_ADDR: u32 = 0x03ff_fff8;
const IRQ_VBLANK: u16 = 1 << 0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BiosBackend {
    Hle,
    External,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct InterruptWait {
    irq_mask: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bios {
    backend: BiosBackend,
    wait: Option<InterruptWait>,
}

impl Default for Bios {
    fn default() -> Self {
        Self::new()
    }
}

impl Bios {
    pub fn new() -> Self {
        Self {
            backend: BiosBackend::Hle,
            wait: None,
        }
    }

    pub fn backend(&self) -> BiosBackend {
        self.backend
    }

    pub fn use_external(&mut self) {
        self.backend = BiosBackend::External;
        self.wait = None;
    }

    pub fn use_hle(&mut self) {
        self.backend = BiosBackend::Hle;
    }

    pub fn handle_step(&mut self, cpu: &mut Arm7tdmi, bus: &mut Bus) -> Result<Option<u32>, BiosError> {
        if self.backend != BiosBackend::Hle {
            return Ok(None);
        }

        if let Some(cycles) = self.handle_intr_wait(bus) {
            return Ok(Some(cycles));
        }

        if let Some(cycles) = self.handle_irq_vector(cpu, bus)? {
            return Ok(Some(cycles));
        }

        let Some((swi, saved, lr)) = decode_pending_swi(cpu, bus)? else {
            return Ok(None);
        };

        match swi {
            SWI_SOFT_RESET => {
                cpu.set_cpsr(saved);
                cpu.set_pc(0x0800_0000);
                Ok(Some(3))
            }
            SWI_REGISTER_RAM_RESET => {
                let reset_mask = cpu.read_reg(0) as u8;
                bus.register_ram_reset(reset_mask);
                return_from_exception(cpu, saved, lr);
                Ok(Some(6))
            }
            SWI_INTR_WAIT => {
                let clear_old = cpu.read_reg(0) != 0;
                let irq_mask = cpu.read_reg(1) as u16;
                self.begin_intr_wait(cpu, bus, saved, lr, irq_mask, clear_old);
                Ok(Some(4))
            }
            SWI_VBLANK_INTR_WAIT => {
                self.begin_intr_wait(cpu, bus, saved, lr, IRQ_VBLANK, true);
                Ok(Some(4))
            }
            SWI_DIV => {
                let numerator = cpu.read_reg(0) as i32;
                let denominator = cpu.read_reg(1) as i32;
                if denominator == 0 {
                    return Err(BiosError::DivideByZero);
                }

                let quotient = numerator / denominator;
                let remainder = numerator % denominator;
                cpu.write_reg(0, quotient as u32);
                cpu.write_reg(1, remainder as u32);
                cpu.write_reg(3, quotient.wrapping_abs() as u32);
                return_from_exception(cpu, saved, lr);
                Ok(Some(4))
            }
            function => Err(BiosError::UnsupportedSwi(function)),
        }
    }

    fn begin_intr_wait(
        &mut self,
        cpu: &mut Arm7tdmi,
        bus: &mut Bus,
        saved: Psr,
        lr: u32,
        irq_mask: u16,
        clear_old: bool,
    ) {
        if clear_old {
            clear_ifbios_bits(bus, irq_mask);
            bus.io_mut().write_16(IF_ADDR, irq_mask);
        }

        return_from_exception(cpu, saved, lr);

        if irq_mask == 0 {
            return;
        }

        if current_irq_flags(bus) & irq_mask != 0 {
            return;
        }

        self.wait = Some(InterruptWait { irq_mask });
    }

    fn handle_intr_wait(&mut self, bus: &mut Bus) -> Option<u32> {
        let wait = self.wait?;
        if current_irq_flags(bus) & wait.irq_mask != 0 {
            self.wait = None;
            None
        } else {
            Some(1)
        }
    }

    fn handle_irq_vector(&mut self, cpu: &mut Arm7tdmi, bus: &mut Bus) -> Result<Option<u32>, BiosError> {
        if cpu.mode() != Mode::Irq || cpu.pc() != 0x0000_0018 {
            return Ok(None);
        }

        let saved = cpu.spsr(Mode::Irq).ok_or(BiosError::MissingIrqSpsr)?;
        let lr = cpu.read_reg_for_mode(Mode::Irq, LR);
        let pending = bus.io().irq_pending_mask();

        if pending != 0 {
            let ifbios = read_ifbios(bus);
            write_ifbios(bus, ifbios | pending);
            bus.io_mut().write_16(IF_ADDR, pending);
        }

        return_from_irq_exception(cpu, saved, lr);
        Ok(Some(3))
    }
}

fn decode_pending_swi<B: BusInterface>(
    cpu: &Arm7tdmi,
    bus: &mut B,
) -> Result<Option<(u8, Psr, u32)>, BiosError> {
    if cpu.mode() != Mode::Supervisor || cpu.pc() != 0x0000_0008 {
        return Ok(None);
    }

    let saved = cpu
        .spsr(Mode::Supervisor)
        .ok_or(BiosError::MissingSupervisorSpsr)?;
    let lr = cpu.read_reg_for_mode(Mode::Supervisor, LR);
    let swi_addr = if saved.thumb() {
        lr.wrapping_sub(2)
    } else {
        lr.wrapping_sub(4)
    };

    let function = if saved.thumb() {
        let opcode = bus.read_16(swi_addr);
        (opcode & 0x00ff) as u8
    } else {
        let opcode = bus.read_32(swi_addr);
        (opcode & 0x0000_00ff) as u8
    };

    Ok(Some((function, saved, lr)))
}

fn return_from_exception(cpu: &mut Arm7tdmi, saved: Psr, lr: u32) {
    cpu.set_cpsr(saved);
    cpu.set_pc(if saved.thumb() { lr & !1 } else { lr & !3 });
}

fn return_from_irq_exception(cpu: &mut Arm7tdmi, saved: Psr, lr: u32) {
    let resume_pc = lr.wrapping_sub(4);
    cpu.set_cpsr(saved);
    cpu.set_pc(if saved.thumb() {
        resume_pc & !1
    } else {
        resume_pc & !3
    });
}

fn read_ifbios<B: BusInterface>(bus: &mut B) -> u16 {
    bus.read_16(REG_IFBIOS_ADDR)
}

fn write_ifbios<B: BusInterface>(bus: &mut B, value: u16) {
    bus.write_16(REG_IFBIOS_ADDR, value);
}

fn clear_ifbios_bits(bus: &mut Bus, mask: u16) {
    let cleared = read_ifbios(bus) & !mask;
    write_ifbios(bus, cleared);
}

fn current_irq_flags(bus: &mut Bus) -> u16 {
    bus.io().irq_pending_mask() | (read_ifbios(bus) & bus.io().ie())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BiosError {
    MissingSupervisorSpsr,
    MissingIrqSpsr,
    UnsupportedSwi(u8),
    DivideByZero,
}

impl core::fmt::Display for BiosError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::MissingSupervisorSpsr => {
                write!(f, "BIOS HLE entered supervisor mode without an SPSR")
            }
            Self::MissingIrqSpsr => write!(f, "BIOS HLE entered IRQ mode without an SPSR"),
            Self::UnsupportedSwi(function) => {
                write!(f, "unsupported BIOS SWI 0x{function:02x}")
            }
            Self::DivideByZero => write!(f, "BIOS Div SWI called with a zero denominator"),
        }
    }
}

impl std::error::Error for BiosError {}

#[cfg(test)]
mod tests {
    use rgba_arm7tdmi::{Arm7tdmi, BusInterface, Exception};

    use super::{
        decode_pending_swi, read_ifbios, return_from_exception, return_from_irq_exception, Bios,
        BiosBackend, BiosError, REG_IFBIOS_ADDR,
    };
    use crate::{Bus, Cartridge, Gba};

    #[test]
    fn decode_pending_arm_swi_extracts_immediate() {
        let mut cpu = Arm7tdmi::new();
        let mut bus = Bus::new(Cartridge::new(0xef00_0006u32.to_le_bytes().to_vec()));
        cpu.enter_exception(Exception::SoftwareInterrupt, 0x0800_0004);

        let decoded = decode_pending_swi(&cpu, &mut bus).unwrap();

        assert_eq!(decoded.map(|value| value.0), Some(0x06));
    }

    #[test]
    fn decode_pending_thumb_swi_extracts_immediate() {
        let mut rom = vec![0; 4];
        rom[0..2].copy_from_slice(&0xdf01u16.to_le_bytes());
        let mut cpu = Arm7tdmi::new();
        let mut bus = Bus::new(Cartridge::new(rom));
        cpu.set_thumb(true);
        cpu.enter_exception(Exception::SoftwareInterrupt, 0x0800_0002);

        let decoded = decode_pending_swi(&cpu, &mut bus).unwrap();

        assert_eq!(decoded.map(|value| value.0), Some(0x01));
    }

    #[test]
    fn return_from_exception_restores_thumb_state() {
        let mut gba = Gba::new(Cartridge::new(vec![0; 16]));
        let mut saved = gba.cpu().cpsr();
        saved.set_thumb(true);

        return_from_exception(gba.cpu_mut(), saved, 0x0800_0002);

        assert!(gba.cpu().is_thumb());
        assert_eq!(gba.cpu().pc(), 0x0800_0002);
    }

    #[test]
    fn return_from_irq_exception_applies_irq_resume_offset() {
        let mut gba = Gba::new(Cartridge::new(vec![0; 16]));
        let mut saved = gba.cpu().cpsr();
        saved.set_thumb(true);

        return_from_irq_exception(gba.cpu_mut(), saved, 0x0800_0006);

        assert!(gba.cpu().is_thumb());
        assert_eq!(gba.cpu().pc(), 0x0800_0002);
    }

    #[test]
    fn bios_defaults_to_hle_backend() {
        let mut bios = Bios::new();
        assert_eq!(bios.backend(), BiosBackend::Hle);
        bios.use_external();
        assert_eq!(bios.backend(), BiosBackend::External);
        bios.use_hle();
        assert_eq!(bios.backend(), BiosBackend::Hle);
    }

    #[test]
    fn unsupported_swi_returns_error() {
        let mut cpu = Arm7tdmi::new();
        let mut bus = Bus::new(Cartridge::new(0xef00_00ffu32.to_le_bytes().to_vec()));
        cpu.enter_exception(Exception::SoftwareInterrupt, 0x0800_0004);

        let err = Bios::new().handle_step(&mut cpu, &mut bus).unwrap_err();

        assert_eq!(err, BiosError::UnsupportedSwi(0xff));
    }

    #[test]
    fn ifbios_round_trips_through_iwram_mirror() {
        let mut bus = Bus::new(Cartridge::new(vec![0; 4]));
        bus.write_16(REG_IFBIOS_ADDR, 0x1234);

        assert_eq!(read_ifbios(&mut bus), 0x1234);
    }
}
