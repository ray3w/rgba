/// ARM7TDMI CPU emulation.
///
/// This crate is intentionally independent of GBA hardware details.
/// It communicates with the outside world through the `BusInterface` trait,
/// allowing it to be tested in isolation with a fake bus.

/// The interface that the CPU uses to access memory.
/// Implemented by the GBA bus in `rgba-core`, or by a fake bus in tests.
pub trait BusInterface {
    fn read_8(&mut self, addr: u32) -> u8;
    fn read_16(&mut self, addr: u32) -> u16;
    fn read_32(&mut self, addr: u32) -> u32;
    fn write_8(&mut self, addr: u32, val: u8);
    fn write_16(&mut self, addr: u32, val: u16);
    fn write_32(&mut self, addr: u32, val: u32);
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
