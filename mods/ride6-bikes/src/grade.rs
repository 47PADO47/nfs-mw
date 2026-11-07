//! How the base colour of a bike is graded on its way into the car's diffuse picture: the occlusion, the lift and the reflection
//! of a dielectric. All of it is a modelling choice, measured in
//! [the material model](../../../docs/formats/ride6-materials.md#how-bright-the-baked-bikes-are).

/// The occlusion factor of a fully occluded texel: one minus the share of indirect light in the installed light rigs (0.46 of
/// the light a surface receives, averaged over its normals), because RIDE 6 applies its occlusion to the indirect light only
/// (docs/formats/ride6-materials.md). A choice, derived from the rig.
pub const DEFAULT_AO_FLOOR: f32 = 0.55;
/// The occlusion factor of a fully occluded texel of a livery (the painted plastics, tank, fairings and fenders): the paint is a
/// clear-coated surface that the dark baked occlusion of the livery atlas (concave areas under the tank, between panels) turned into
/// muddy patches, where RIDE 6 shows a clean glossy colour (the occlusion only dims its indirect light). A choice, made against the
/// renders of the KTM RC 390 (orange panels with brown blotches) and the Honda CRF 450 R; the mechanics keep [`DEFAULT_AO_FLOOR`].
pub const PAINT_AO_FLOOR: f32 = 0.9;
/// The largest lift [`lift`] allows.
pub const MAX_GAIN: f32 = 2.0;
/// The lift of the base colour: the car rigs light a surface with 0.5 to 0.85 of the colour of its texture (the multiplier of
/// a rig is a fraction in display space), where RIDE 6 shows its lit bodies at about 1.1 times the albedo; 1.35 closes
/// that gap in the middle tones and keeps white white. A choice, measured (docs/formats/ride6-materials.md).
pub const DEFAULT_GAIN: f32 = 1.35;
/// The mean radiance of the environment a dielectric reflects (linear, 0 to 1). RIDE 6 adds `F0 * radiance` to a dielectric's
/// colour (`F0 = 0.08 * specular`, 4 % for the picture's usual 0.5), which is why its black plastics are dark grey and not
/// black; the car light materials of the bikes (`SHINYPLASTIC`, `DULLPLASTIC`) reflect nothing, so a part of that amount is
/// baked into the colour. A choice: a quarter, half of what a bright sky would give, because the baked amount is lit by the
/// car rig as well. It is applied in proportion to how little saturated the texel is (`bake::bake_diffuse`): a veil of one grey on a
/// saturated paint (the red (184, 0, 26) of the Honda CRF 450 R came out pink) is what made the bikes look washed out.
pub const DEFAULT_REFLECTED: f32 = 0.25;

/// How much of the occlusion a bare metal is spared (0 to 1): the floor of the occlusion factor of a texel of metallic value `m`
/// is `floor + (1 - floor) * m * METAL_AO_RELIEF`. RIDE 6 applies the occlusion to the light a surface receives indirectly, and a
/// metal shows mostly the environment it reflects, so the baked occlusion of a silencer or an engine atlas (the stock can of the
/// Honda CRF 450 R measures 0.27 on average with a metallic value of 0.88) must not turn polished silver into gunmetal. A choice,
/// made against the garage pictures of RIDE 6 (a silver can and silver frame cases that the full floor made dark grey).
pub const METAL_AO_RELIEF: f32 = 0.8;

/// The floor of the occlusion factor for a texel of metallic value `metal` (0 to 1) when the plain floor is `ao_floor`.
pub fn metal_floor(ao_floor: f32, metal: f32) -> f32 {
    ao_floor + (1.0 - ao_floor) * metal.clamp(0.0, 1.0) * METAL_AO_RELIEF
}

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
    let gain = gain.clamp(1.0, MAX_GAIN);
    let x = f32::from(value) / 255.0;
    (x * (gain - (gain - 1.0) * x) * 255.0 + 0.5) as u8
}

/// An encoded colour lifted by `gain` with its hue kept: the brightest channel is lifted as [`lift`] does and the others by the
/// same factor, so a saturated paint (the red (184, 0, 26) of the Honda CRF 450 R) gets brighter and not pinker (lifting each
/// channel alone raised the blue of that red from 26 to 33 and made it crimson). Greys come out as [`lift`] gives them.
pub fn lift_colour([r, g, b]: [u8; 3], gain: f32) -> [u8; 3] {
    let top = r.max(g).max(b);
    if top == 0 {
        return [0, 0, 0];
    }
    let factor = f32::from(lift(top, gain)) / f32::from(top);
    [r, g, b].map(|c| (f32::from(c) * factor + 0.5).min(255.0) as u8)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_hue_lift_keeps_the_ratio_of_the_channels_and_lifts_greys_like_the_plain_lift() {
        assert_eq!(lift_colour([0, 0, 0], 1.35), [0, 0, 0]);
        assert_eq!(lift_colour([100, 100, 100], 1.35), [lift(100, 1.35); 3]);
        assert_eq!(lift_colour([255, 255, 255], 1.35), [255, 255, 255]);
        let red = lift_colour([184, 0, 26], 1.35);
        assert!(red[0] > 184 && red[1] == 0, "{red:?}");
        let (before, after) = (26.0 / 184.0, f32::from(red[2]) / f32::from(red[0]));
        assert!((before - after).abs() < 0.01, "the blue share of the red stays: {before} {after}");
    }

    #[test]
    fn a_metal_is_spared_most_of_the_occlusion() {
        assert_eq!(metal_floor(0.55, 0.0), 0.55);
        assert!((metal_floor(0.55, 1.0) - (0.55 + 0.45 * METAL_AO_RELIEF)).abs() < 1e-6);
        assert!(metal_floor(0.55, 1.0) > 0.9);
        assert_eq!(metal_floor(1.0, 1.0), 1.0, "no occlusion floor below one is lowered by the relief");
    }

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
