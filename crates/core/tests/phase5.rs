use rgba_arm7tdmi::{BusInterface, Mode};
use rgba_core::{Button, Cartridge, Gba};

const DISPSTAT_ADDR: u32 = 0x0400_0004;
const TM0CNT_L_ADDR: u32 = 0x0400_0100;
const TM0CNT_H_ADDR: u32 = 0x0400_0102;
const KEYINPUT_ADDR: u32 = 0x0400_0130;
const IE_ADDR: u32 = 0x0400_0200;
const IME_ADDR: u32 = 0x0400_0208;

const DMA0SAD_ADDR: u32 = 0x0400_00b0;
const DMA0DAD_ADDR: u32 = 0x0400_00b4;
const DMA0CNT_L_ADDR: u32 = 0x0400_00b8;
const DMA0CNT_H_ADDR: u32 = 0x0400_00ba;

const IRQ_VBLANK: u16 = 1 << 0;
const IRQ_TIMER0: u16 = 1 << 3;

fn push_word(rom: &mut Vec<u8>, value: u32) {
    rom.extend_from_slice(&value.to_le_bytes());
}

fn idle_rom(words: usize) -> Vec<u8> {
    let mut rom = Vec::with_capacity(words * 4);
    for _ in 0..words {
        push_word(&mut rom, 0xe3a0_0000); // MOV r0, #0
    }
    rom
}

#[test]
fn vblank_irq_enters_cpu_irq_vector() {
    let mut gba = Gba::new(Cartridge::new(idle_rom(200_000)));
    gba.cpu_mut().set_pc(0x0800_0000);
    gba.bus_mut().write_16(DISPSTAT_ADDR, 1 << 3);
    gba.bus_mut().write_16(IE_ADDR, IRQ_VBLANK);
    gba.bus_mut().write_16(IME_ADDR, 1);

    for _ in 0..=(160 * 1232) {
        gba.step();
        if gba.cpu().mode() == Mode::Irq {
            break;
        }
    }

    assert_eq!(gba.cpu().mode(), Mode::Irq);
    assert_eq!(gba.cpu().pc(), 0x0000_0018);
}

#[test]
fn timer0_overflow_requests_interrupt_and_vectors_cpu() {
    let mut gba = Gba::new(Cartridge::new(idle_rom(64)));
    gba.cpu_mut().set_pc(0x0800_0000);
    gba.bus_mut().write_16(IE_ADDR, IRQ_TIMER0);
    gba.bus_mut().write_16(IME_ADDR, 1);
    gba.bus_mut().write_16(TM0CNT_L_ADDR, 0xfffe);
    gba.bus_mut().write_16(TM0CNT_H_ADDR, 0x00c0);

    for _ in 0..4 {
        gba.step();
        if gba.cpu().mode() == Mode::Irq {
            break;
        }
    }

    assert_eq!(gba.cpu().mode(), Mode::Irq);
    assert_eq!(gba.cpu().pc(), 0x0000_0018);
}

#[test]
fn keypad_state_is_reflected_in_keyinput() {
    let mut gba = Gba::with_rom(idle_rom(4));

    gba.set_button_pressed(Button::A, true);
    gba.set_button_pressed(Button::Start, true);

    let keys = gba.bus().io().read_16(KEYINPUT_ADDR);
    assert_eq!(keys & Button::A.mask(), 0);
    assert_eq!(keys & Button::Start.mask(), 0);

    gba.set_button_pressed(Button::A, false);
    let keys = gba.bus().io().read_16(KEYINPUT_ADDR);
    assert_ne!(keys & Button::A.mask(), 0);
    assert_eq!(keys & Button::Start.mask(), 0);
}

#[test]
fn dma_immediate_moves_data_through_real_bus() {
    let mut gba = Gba::new(Cartridge::new(idle_rom(8)));
    gba.cpu_mut().set_pc(0x0800_0000);
    gba.bus_mut().write_32(0x0200_0000, 0x1234_5678);
    gba.bus_mut().write_32(0x0200_0004, 0x89ab_cdef);

    gba.bus_mut().write_32(DMA0SAD_ADDR, 0x0200_0000);
    gba.bus_mut().write_32(DMA0DAD_ADDR, 0x0300_0000);
    gba.bus_mut().write_16(DMA0CNT_L_ADDR, 2);
    gba.bus_mut().write_16(DMA0CNT_H_ADDR, 0x8400);

    gba.step();

    assert_eq!(gba.bus_mut().read_32(0x0300_0000), 0x1234_5678);
    assert_eq!(gba.bus_mut().read_32(0x0300_0004), 0x89ab_cdef);
}
