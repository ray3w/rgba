/// GBA hardware integration.
///
/// This crate wires together the CPU, bus, PPU, and other subsystems
/// into a complete GBA emulation core.

pub use rgba_arm7tdmi as arm7tdmi;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
