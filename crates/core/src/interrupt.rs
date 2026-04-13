use rgba_arm7tdmi::{Arm7tdmi, Exception};

use crate::io::IoRegs;

/// Returns true when the MMIO interrupt controller should present an IRQ to
/// the CPU.
pub fn irq_pending(io: &IoRegs) -> bool {
    io.ime_enabled() && io.irq_pending_mask() != 0
}

/// Services a pending IRQ if one is visible to the CPU.
///
/// The current CPU core models exception entry explicitly, so the outer GBA
/// loop provides the architectural LR value.
pub fn service_irq(cpu: &mut Arm7tdmi, io: &IoRegs) -> bool {
    if cpu.cpsr().irq_disabled() || !irq_pending(io) {
        return false;
    }

    cpu.enter_exception(Exception::Irq, cpu.pc().wrapping_add(4));
    true
}

#[cfg(test)]
mod tests {
    use rgba_arm7tdmi::{Arm7tdmi, Mode};

    use super::{irq_pending, service_irq};
    use crate::io::{IoRegs, IRQ_VBLANK};

    #[test]
    fn irq_pending_requires_ime_and_matching_ie_if_bits() {
        let mut io = IoRegs::new();
        io.write_16(0x0400_0200, IRQ_VBLANK);
        io.request_interrupt(IRQ_VBLANK);

        assert!(!irq_pending(&io));

        io.write_16(0x0400_0208, 1);
        assert!(irq_pending(&io));
    }

    #[test]
    fn service_irq_vectors_cpu_into_irq_mode() {
        let mut cpu = Arm7tdmi::new();
        let mut io = IoRegs::new();
        cpu.set_pc(0x0800_0004);
        io.write_16(0x0400_0200, IRQ_VBLANK);
        io.write_16(0x0400_0208, 1);
        io.request_interrupt(IRQ_VBLANK);

        assert!(service_irq(&mut cpu, &io));
        assert_eq!(cpu.mode(), Mode::Irq);
        assert_eq!(cpu.pc(), 0x0000_0018);
        assert_eq!(cpu.lr(), 0x0800_0008);
    }
}
