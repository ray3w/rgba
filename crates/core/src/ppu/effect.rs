use crate::io::IoRegs;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlendMode {
    Off,
    Alpha,
    Brighten,
    Darken,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EffectConfig {
    pub mode: BlendMode,
    pub first_target_mask: u16,
    pub second_target_mask: u16,
    pub eva: u8,
    pub evb: u8,
    pub evy: u8,
}

#[cfg_attr(not(test), allow(dead_code))]
impl EffectConfig {
    pub const fn new(
        mode: BlendMode,
        first_target_mask: u16,
        second_target_mask: u16,
        eva: u8,
        evb: u8,
        evy: u8,
    ) -> Self {
        Self {
            mode,
            first_target_mask,
            second_target_mask,
            eva,
            evb,
            evy,
        }
    }

    pub fn from_io(io: &IoRegs) -> Self {
        let bldcnt = io.bldcnt();
        let bldalpha = io.bldalpha();
        let bldy = io.bldy();
        let mode = match (bldcnt >> 6) & 0x0003 {
            1 => BlendMode::Alpha,
            2 => BlendMode::Brighten,
            3 => BlendMode::Darken,
            _ => BlendMode::Off,
        };

        Self {
            mode,
            first_target_mask: bldcnt & 0x003f,
            second_target_mask: (bldcnt >> 8) & 0x003f,
            eva: ((bldalpha & 0x001f).min(16)) as u8,
            evb: (((bldalpha >> 8) & 0x001f).min(16)) as u8,
            evy: (bldy & 0x001f).min(16) as u8,
        }
    }

    pub fn first_target_enabled(&self, target_bit: u16) -> bool {
        (self.first_target_mask & target_bit) != 0
    }

    pub fn second_target_enabled(&self, target_bit: u16) -> bool {
        (self.second_target_mask & target_bit) != 0
    }
}

pub fn alpha_blend(top: u16, bottom: u16, eva: u8, evb: u8) -> u16 {
    blend_channels(top, bottom, eva, evb, BlendChannels::Alpha)
}

pub fn brighten(color: u16, evy: u8) -> u16 {
    blend_channels(color, 0, evy, 0, BlendChannels::Brighten)
}

pub fn darken(color: u16, evy: u8) -> u16 {
    blend_channels(color, 0, evy, 0, BlendChannels::Darken)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BlendChannels {
    Alpha,
    Brighten,
    Darken,
}

fn blend_channels(top: u16, bottom: u16, a: u8, b: u8, mode: BlendChannels) -> u16 {
    let tr = (top & 0x001f) as u8;
    let tg = ((top >> 5) & 0x001f) as u8;
    let tb = ((top >> 10) & 0x001f) as u8;
    let br = (bottom & 0x001f) as u8;
    let bg = ((bottom >> 5) & 0x001f) as u8;
    let bb = ((bottom >> 10) & 0x001f) as u8;

    let (r, g, b) = match mode {
        BlendChannels::Alpha => (
            alpha_channel(tr, br, a, b),
            alpha_channel(tg, bg, a, b),
            alpha_channel(tb, bb, a, b),
        ),
        BlendChannels::Brighten => (
            brighten_channel(tr, a),
            brighten_channel(tg, a),
            brighten_channel(tb, a),
        ),
        BlendChannels::Darken => (
            darken_channel(tr, a),
            darken_channel(tg, a),
            darken_channel(tb, a),
        ),
    };

    u16::from(r) | (u16::from(g) << 5) | (u16::from(b) << 10)
}

fn alpha_channel(top: u8, bottom: u8, eva: u8, evb: u8) -> u8 {
    let value = (u16::from(top) * u16::from(eva) + u16::from(bottom) * u16::from(evb)) >> 4;
    value.min(31) as u8
}

fn brighten_channel(value: u8, evy: u8) -> u8 {
    let delta = ((31 - u16::from(value)) * u16::from(evy)) >> 4;
    (u16::from(value) + delta).min(31) as u8
}

fn darken_channel(value: u8, evy: u8) -> u8 {
    let delta = (u16::from(value) * u16::from(evy)) >> 4;
    u16::from(value).saturating_sub(delta) as u8
}

#[cfg(test)]
mod tests {
    use super::{alpha_blend, brighten, darken, BlendMode, EffectConfig};
    use crate::io::IoRegs;

    #[test]
    fn alpha_blend_combines_two_rgb555_colors() {
        let color = alpha_blend(0x001f, 0x03e0, 8, 8);
        assert_eq!(color, 0x01ef);
    }

    #[test]
    fn brighten_moves_channels_toward_white() {
        let color = brighten(0x001f, 8);
        assert!(color > 0x001f);
    }

    #[test]
    fn darken_moves_channels_toward_black() {
        let color = darken(0x7fff, 8);
        assert!(color < 0x7fff);
    }

    #[test]
    fn effect_config_decodes_mmio_registers() {
        let mut io = IoRegs::new();
        io.write_16(0x0400_0050, 0x2144);
        io.write_16(0x0400_0052, 0x1008);
        io.write_16(0x0400_0054, 0x0012);

        let config = EffectConfig::from_io(&io);
        assert_eq!(config.mode, BlendMode::Alpha);
        assert_eq!(config.first_target_mask, 0x0004);
        assert_eq!(config.second_target_mask, 0x0021);
        assert_eq!(config.eva, 8);
        assert_eq!(config.evb, 16);
        assert_eq!(config.evy, 16);
    }
}
