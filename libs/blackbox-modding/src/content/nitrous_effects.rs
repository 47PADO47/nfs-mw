//! The particle effects of a nitrous variant (`[[effect]]` of `profile.toml`, see `nitrous_fx`): emitters played at every
//! tail pipe of the car when the nitrous lights, while it burns and at a blow-off or a backfire, drawn by the game's
//! own particle system with the mod's own textures.
//!
//! Each effect is one emitter: a texture (an RGBA PNG), how it blends, and the numbers that move it. The host turns them
//! into its emitter parameters (`crates/nfsmw` `exhaust/variant.rs`).
//!
//! ```toml
//! [[effect]]
//! on = "burn"                   # "start" (for `duration` seconds after the nitrous lights), "burn" or "pop"
//! texture = "NITROUS/MW2012/GLOW.PNG"
//! blend = "additive"            # or "alpha"; the default is additive
//! grid = 0                      # 2 to 16 cells per side for a flipbook, 0 for the whole picture
//! fps = 24                      # flipbook frames a second
//! rate = 20.0                   # particles a second, plus rate_per_speed for each m/s the car goes
//! life = 0.3                    # seconds at most; life_variance (0 to 1) is how much of it a particle may fall short of
//! speed = 1.0                   # metres a second along the pipe's axis
//! size = [0.1, 0.3, 0.3, 0.1]   # full width in metres at each key
//! colors = [[255, 255, 255, 0], [255, 255, 255, 255], [255, 255, 255, 255], [255, 255, 255, 0]]
//! keys = [0.0, 0.33, 0.66, 1.0]  # where the sizes and colours sit along the life (2 to 16; straight lines between)
//! damping = 0.0                 # the share of its velocity a particle loses every 60th of a second (0.75: all but a quarter)
//! ```

use serde::Deserialize;

use super::ContentError;
use super::engine_sound::is_png_path;

/// The most effects a profile may have.
pub const MAX_EFFECTS: usize = 16;
/// The most keys along a particle's life.
pub const MAX_KEYS: usize = 16;

/// When an effect plays.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EffectOn {
    /// For `duration` seconds after the nitrous lights.
    Start,
    /// For as long as the nitrous burns.
    Burn,
    /// At a gear change blow-off or a backfire: while the game's own flame would show.
    Pop,
}

/// One particle emitter of a variant, played at every tail pipe of the car in the pipe's frame (the local `z` axis is the way
/// the flame leaves).
#[derive(Debug, Clone, PartialEq)]
pub struct EffectProfile {
    pub on: EffectOn,
    /// An RGBA PNG, a relative path in the mod's `files/` (the same rule as the sounds).
    pub texture: String,
    pub additive: bool,
    /// 0 for the whole picture, else 2 to 16 cells per side, played as a flipbook.
    pub grid: u8,
    pub fps: u8,
    pub random_start: bool,
    /// Particles a second, plus `rate_per_speed` for each metre a second the car goes.
    pub rate: f32,
    pub rate_per_speed: f32,
    /// Seconds a particle lives at most, and the fraction of that it may fall short of: the life is `life * (1 - v)` to `life`.
    pub life: f32,
    pub life_variance: f32,
    /// Metres a second along the pipe's axis, with its variance, and the cone's full angle in degrees.
    pub speed: f32,
    pub speed_variance: f32,
    pub spread: f32,
    /// Metres a second squared the particles rise (negative falls).
    pub lift: f32,
    pub drag: f32,
    /// The share of its velocity a particle loses every 60th of a second, 0 to 1 (1 stops it at once).
    pub damping: f32,
    /// The share of the car's velocity a particle takes at birth, 0 to 1, and the fraction of it a particle may fall short of.
    pub inherit: f32,
    pub inherit_variance: f32,
    /// Degrees: the starting rotation is anywhere in this span (0 keeps the picture upright).
    pub angle_range: f32,
    /// Where the values of `size` and `colors` sit along the particle's life, 0 to 1, rising: 2 to 16 of them. Between the
    /// keys the size and the colour change in straight lines.
    pub keys: Vec<f32>,
    /// Full width in metres at each key, and the colour there.
    pub size: Vec<f32>,
    pub colors: Vec<[u8; 4]>,
    /// Degrees a second a particle turns at most, either way (0 keeps it as it was born).
    pub spin: f32,
    /// Seconds the effect spawns after its moment begins (the lighting of the nitrous, or the pop); 0 means for as long as
    /// the moment lasts.
    pub duration: f32,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct RawEffect {
    on: Option<String>,
    texture: String,
    blend: Option<String>,
    grid: Option<u8>,
    fps: Option<u8>,
    random_start: Option<bool>,
    rate: Option<f32>,
    rate_per_speed: Option<f32>,
    life: Option<f32>,
    life_variance: Option<f32>,
    speed: Option<f32>,
    speed_variance: Option<f32>,
    spread: Option<f32>,
    lift: Option<f32>,
    drag: Option<f32>,
    damping: Option<f32>,
    inherit: Option<f32>,
    inherit_variance: Option<f32>,
    angle_range: Option<f32>,
    keys: Option<Vec<f32>>,
    size: Option<Vec<f32>>,
    colors: Option<Vec<[u8; 4]>>,
    spin: Option<f32>,
    duration: Option<f32>,
}

fn within(value: Option<f32>, default: f32, low: f32, high: f32, field: &'static str) -> Result<f32, ContentError> {
    let value = value.unwrap_or(default);
    if !value.is_finite() || value < low || value > high {
        return Err(ContentError::Field { field, reason: "a number outside the range this field takes" });
    }
    Ok(value)
}

/// Check the effects of a nitrous profile: each says when it plays (`on`).
pub(super) fn check(raw: Vec<RawEffect>) -> Result<Vec<EffectProfile>, ContentError> {
    if raw.len() > MAX_EFFECTS {
        return Err(ContentError::Field { field: "effect", reason: "a profile has at most 16 effects" });
    }
    raw.into_iter().map(|raw| effect(raw, true)).collect()
}

/// Check the particle layers of a profile of another kind of effect (`[[layer]]`, the tire smoke's): the same keys, with no
/// `on` (a layer plays while its effect does, which is `EffectOn::Burn` here), and at most `most` of them.
pub(super) fn check_layers(raw: Vec<RawEffect>, most: usize) -> Result<Vec<EffectProfile>, ContentError> {
    if raw.len() > most {
        return Err(ContentError::Field { field: "layer", reason: "more layers than this effect takes" });
    }
    raw.into_iter().map(|raw| effect(raw, false)).collect()
}

fn effect(raw: RawEffect, needs_on: bool) -> Result<EffectProfile, ContentError> {
    let on = match (raw.on.as_deref(), needs_on) {
        (Some("start"), true) => EffectOn::Start,
        (Some("burn"), true) => EffectOn::Burn,
        (Some("pop"), true) => EffectOn::Pop,
        (None, false) => EffectOn::Burn,
        (_, true) => return Err(ContentError::Field { field: "effect.on", reason: "start, burn or pop" }),
        (_, false) => return Err(ContentError::Field { field: "layer.on", reason: "a layer has no `on`" }),
    };
    let additive = match raw.blend.as_deref() {
        None | Some("additive") => true,
        Some("alpha") => false,
        Some(_) => return Err(ContentError::Field { field: "effect.blend", reason: "additive or alpha" }),
    };
    let grid = raw.grid.unwrap_or(0);
    if grid == 1 || grid > 16 {
        return Err(ContentError::Field { field: "effect.grid", reason: "0, or 2 to 16" });
    }
    if !is_png_path(&raw.texture) {
        return Err(ContentError::Field {
            field: "effect.texture",
            reason: "a relative .PNG path of letters, digits, _ - . /",
        });
    }
    let keys = raw.keys.unwrap_or_else(|| vec![0.0, 1.0 / 3.0, 2.0 / 3.0, 1.0]);
    let count = keys.len();
    if !(2..=MAX_KEYS).contains(&count) {
        return Err(ContentError::Field { field: "effect.keys", reason: "2 to 16 numbers" });
    }
    let rising = keys.windows(2).all(|pair| pair[0] <= pair[1]);
    if !rising || keys.iter().any(|k| !k.is_finite() || !(0.0..=1.0).contains(k)) {
        return Err(ContentError::Field { field: "effect.keys", reason: "rising numbers from 0 to 1" });
    }
    let size = raw.size.unwrap_or_else(|| vec![0.2; count]);
    if size.len() != count || size.iter().any(|s| !s.is_finite() || !(0.0..=20.0).contains(s)) {
        return Err(ContentError::Field { field: "effect.size", reason: "a number from 0 to 20 metres for each key" });
    }
    let colors = raw.colors.unwrap_or_else(|| vec![[255; 4]; count]);
    if colors.len() != count {
        return Err(ContentError::Field { field: "effect.colors", reason: "a colour for each key" });
    }
    Ok(EffectProfile {
        on,
        texture: raw.texture,
        additive,
        grid,
        fps: raw.fps.unwrap_or(24),
        random_start: raw.random_start.unwrap_or(false),
        rate: within(raw.rate, 20.0, 0.0, 500.0, "effect.rate")?,
        rate_per_speed: within(raw.rate_per_speed, 0.0, 0.0, 100.0, "effect.rate_per_speed")?,
        life: within(raw.life, 0.3, 0.01, 10.0, "effect.life")?,
        life_variance: within(raw.life_variance, 0.0, 0.0, 1.0, "effect.life_variance")?,
        speed: within(raw.speed, 0.0, -50.0, 50.0, "effect.speed")?,
        speed_variance: within(raw.speed_variance, 0.0, 0.0, 50.0, "effect.speed_variance")?,
        spread: within(raw.spread, 0.0, 0.0, 180.0, "effect.spread")?,
        lift: within(raw.lift, 0.0, -50.0, 50.0, "effect.lift")?,
        drag: within(raw.drag, 0.0, 0.0, 50.0, "effect.drag")?,
        damping: within(raw.damping, 0.0, 0.0, 1.0, "effect.damping")?,
        inherit: within(raw.inherit, 0.0, 0.0, 1.0, "effect.inherit")?,
        inherit_variance: within(raw.inherit_variance, 0.0, 0.0, 1.0, "effect.inherit_variance")?,
        angle_range: within(raw.angle_range, 0.0, 0.0, 360.0, "effect.angle_range")?,
        keys,
        size,
        colors,
        spin: within(raw.spin, 0.0, 0.0, 3600.0, "effect.spin")?,
        duration: within(raw.duration, 0.0, 0.0, 10.0, "effect.duration")?,
    })
}

#[cfg(test)]
mod tests {
    use super::super::parse_nitrous_profile;
    use super::*;

    const EFFECT: &str = r#"
format = 1
[flame]
replace_burn = true
[[effect]]
on = "burn"
texture = "NITROUS/MW2012/GLOW.PNG"
rate = 30.0
life = 0.2
size = [0.1, 0.3, 0.3, 0.1]
colors = [[1, 2, 3, 0], [1, 2, 3, 255], [1, 2, 3, 255], [1, 2, 3, 0]]
[[effect]]
on = "start"
texture = "NITROUS/MW2012/FLAME.PNG"
blend = "alpha"
grid = 4
duration = 0.8
"#;

    #[test]
    fn effects_parse_in_file_order_with_defaults() {
        let profile = parse_nitrous_profile(EFFECT).unwrap();
        assert!(profile.flame.unwrap().replace_burn);
        assert!(!profile.flame.unwrap().replace_pop);
        assert_eq!(profile.effects.len(), 2);
        let glow = &profile.effects[0];
        assert_eq!((glow.on, glow.additive, glow.grid, glow.rate, glow.life), (EffectOn::Burn, true, 0, 30.0, 0.2));
        assert_eq!(glow.colors[1], [1, 2, 3, 255]);
        let flame = &profile.effects[1];
        assert_eq!(
            (flame.on, flame.additive, flame.grid, flame.fps, flame.duration),
            (EffectOn::Start, false, 4, 24, 0.8)
        );
        assert_eq!(flame.size, vec![0.2; 4], "the size a profile leaves out");
        assert_eq!(flame.keys, vec![0.0, 1.0 / 3.0, 2.0 / 3.0, 1.0]);
    }

    #[test]
    fn any_number_of_keys_from_two_to_sixteen_may_be_used_when_every_list_has_one_value_for_each() {
        let keyed = |keys: &str, size: &str, colors: &str| {
            parse_nitrous_profile(&format!(
                "format = 1
[[effect]]
on = \"burn\"
texture = \"A.PNG\"
keys = {keys}
size = {size}
colors = {colors}
spin = 90.0"
            ))
        };
        let seven = keyed(
            "[0.0, 0.1, 0.2, 0.3, 0.5, 0.9, 1.0]",
            "[0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7]",
            "[[1,2,3,4],[1,2,3,4],[1,2,3,4],[1,2,3,4],[1,2,3,4],[1,2,3,4],[1,2,3,4]]",
        )
        .unwrap();
        assert_eq!((seven.effects[0].keys.len(), seven.effects[0].spin), (7, 90.0));
        assert!(keyed("[0.0, 1.0]", "[0.1]", "[[1,2,3,4],[1,2,3,4]]").is_err(), "one size too few");
        assert!(keyed("[0.0, 1.0]", "[0.1, 0.1]", "[[1,2,3,4]]").is_err(), "one colour too few");
        assert!(keyed("[0.5]", "[0.1]", "[[1,2,3,4]]").is_err(), "a single key is no track");
    }

    #[test]
    fn a_bad_effect_is_refused() {
        let bad = |extra: &str| {
            parse_nitrous_profile(&format!("format = 1\n[[effect]]\non = \"burn\"\ntexture = \"A.PNG\"\n{extra}"))
        };
        assert!(bad("").is_ok());
        assert!(matches!(bad("grid = 17"), Err(ContentError::Field { field: "effect.grid", .. })));
        assert!(matches!(bad("blend = \"x\""), Err(ContentError::Field { field: "effect.blend", .. })));
        assert!(matches!(bad("rate = 9999.0"), Err(ContentError::Field { field: "effect.rate", .. })));
        assert!(matches!(bad("keys = [0.0, 0.5, 0.4, 1.0]"), Err(ContentError::Field { field: "effect.keys", .. })));
        assert!(matches!(bad("damping = 1.5"), Err(ContentError::Field { field: "effect.damping", .. })));
        assert!(bad("damping = 0.75").is_ok());
        assert!(matches!(bad("life_variance = 2.0"), Err(ContentError::Field { field: "effect.life_variance", .. })));
        assert!(matches!(bad("size = [0.1, 0.1, 0.1, 99.0]"), Err(ContentError::Field { field: "effect.size", .. })));
        let wrong_on = "format = 1\n[[effect]]\non = \"sometimes\"\ntexture = \"A.PNG\"";
        assert!(matches!(parse_nitrous_profile(wrong_on), Err(ContentError::Field { field: "effect.on", .. })));
        let wrong_file = "format = 1\n[[effect]]\non = \"burn\"\ntexture = \"../A.PNG\"";
        assert!(matches!(parse_nitrous_profile(wrong_file), Err(ContentError::Field { field: "effect.texture", .. })));
        let too_many = "[[effect]]\non = \"burn\"\ntexture = \"A.PNG\"\n".repeat(17);
        assert!(matches!(parse_nitrous_profile(&format!("format = 1\n{too_many}")), Err(ContentError::Field { .. })));
    }
}
