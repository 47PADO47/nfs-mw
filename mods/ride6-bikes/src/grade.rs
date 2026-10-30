//! How the base colour of a bike is graded on its way into the car's diffuse picture: the occlusion, the lift and the reflection
//! of a dielectric. All of it is a modelling choice, measured in
//! [the material model](../../../docs/formats/ride6-materials.md#how-bright-the-baked-bikes-are).

/// The occlusion factor of a fully occluded texel: one minus the share of indirect light in the installed light rigs (0.46 of
/// the light a surface receives, averaged over its normals), because RIDE 6 applies its occlusion to the indirect light only
/// (docs/formats/ride6-materials.md). A choice, derived from the rig.
pub const DEFAULT_AO_FLOOR: f32 = 0.55;
/// The lift of the base colour: the car rigs light a surface with 0.5 to 0.85 of the colour of its texture (the multiplier of
/// a rig is a fraction in display space), where RIDE 6 shows its lit bodies at about 1.1 times the albedo; 1.35 closes
/// that gap in the middle tones and keeps white white. A choice, measured (docs/formats/ride6-materials.md).
pub const DEFAULT_GAIN: f32 = 1.35;
/// The mean radiance of the environment a dielectric reflects (linear, 0 to 1). RIDE 6 adds `F0 * radiance` to a dielectric's
/// colour (`F0 = 0.08 * specular`, 4 % for the picture's usual 0.5), which is why its black plastics are dark grey and not
/// black; the car light materials of the bikes (`SHINYPLASTIC`, `DULLPLASTIC`) reflect nothing, so a part of that amount is
/// baked into the colour. A choice: a quarter, half of what a bright sky would give, because the baked amount is lit by the
/// car rig as well and, added to a saturated dark colour (the blue of a Yamaha), it also takes saturation away.
pub const DEFAULT_REFLECTED: f32 = 0.25;

/// How the base colour is graded on the way into the diffuse picture.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Grade {
    /// See [`Options::ao_floor`](crate::bake::Options::ao_floor).
    pub ao_floor: f32,
    /// See [`Options::gain`](crate::bake::Options::gain).
    pub gain: f32,
    /// See [`Options::reflected`](crate::bake::Options::reflected).
    pub reflected: f32,
}

impl Default for Grade {
    fn default() -> Self {
        Self { ao_floor: DEFAULT_AO_FLOOR, gain: DEFAULT_GAIN, reflected: DEFAULT_REFLECTED }
    }
}

impl Grade {
    /// No grading: the base colour as it is.
    pub const NONE: Self = Self { ao_floor: 1.0, gain: 1.0, reflected: 0.0 };
}

/// An encoded colour channel lifted by `gain`: `x * (gain - (gain - 1) * x)` on the 0 to 1 value, which multiplies the dark
/// and middle tones by `gain` and leaves white white (the slope falls to `2 - gain` there), so the lift never clips a bright
/// texel. The car shader multiplies the encoded texture by its light, so the lift is made in the encoded domain too.
pub fn lift(value: u8, gain: f32) -> u8 {
    let gain = gain.clamp(1.0, 2.0);
    let x = f32::from(value) / 255.0;
    (x * (gain - (gain - 1.0) * x) * 255.0 + 0.5) as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_lift_multiplies_the_tones_below_white_and_keeps_black_and_white() {
        assert_eq!(lift(0, 1.35), 0);
        assert_eq!(lift(255, 1.35), 255, "white stays white");
        // A dark tone is lifted by about the gain; a middle tone by less; the order of the tones is kept.
        let dark = f32::from(lift(20, 1.35)) / 20.0;
        let middle = f32::from(lift(128, 1.35)) / 128.0;
        assert!(dark > 1.29 && dark < 1.36 && middle > 1.1 && middle < dark, "{dark} {middle}");
        assert!((0..255).all(|v| lift(v, 1.35) <= lift(v + 1, 1.35)), "monotonic");
        assert_eq!(lift(100, 1.0), 100, "a gain of one is no lift");
        assert_eq!(lift(100, 0.2), 100, "a gain below one is taken as one");
    }
}
