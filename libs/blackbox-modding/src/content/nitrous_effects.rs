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
//! grid = 0                      # 2, 4, 8 or 16 cells per side for a flipbook, 0 for the whole picture
//! fps = 24                      # flipbook frames a second
//! rate = 20.0                   # particles a second
//! life = 0.3                    # seconds
//! speed = 1.0                   # metres a second along the pipe's axis
//! size = [0.1, 0.3, 0.3, 0.1]   # full width in metres at four equal steps of the life
//! colors = [[255, 255, 255, 0], [255, 255, 255, 255], [255, 255, 255, 255], [255, 255, 255, 0]]
//! ```

use serde::Deserialize;

use super::ContentError;
use super::engine_sound::is_png_path;

/// The most effects a profile may have.
pub const MAX_EFFECTS: usize = 8;

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
    /// 0 for the whole picture, else 2, 4, 8 or 16 cells per side, played as a flipbook.
    pub grid: u8,
    pub fps: u8,
    pub random_start: bool,
    /// Particles a second, and seconds of life with its variance.
    pub rate: f32,
    pub life: f32,
    pub life_variance: f32,
    /// Metres a second along the pipe's axis, with its variance, and the cone's full angle in degrees.
    pub speed: f32,
    pub speed_variance: f32,
    pub spread: f32,
    /// Metres a second squared the particles rise (negative falls).
    pub lift: f32,
    pub drag: f32,
    /// The share of the car's velocity a particle takes at birth, 0 to 1.
    pub inherit: f32,
    /// Full width in metres at four equal steps of the life, and the colour there.
    pub size: [f32; 4],
    pub colors: [[u8; 4]; 4],
    /// Seconds a `start` effect plays; unused by the others.
    pub duration: f32,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct RawEffect {
    on: String,
    texture: String,
    blend: Option<String>,
    grid: Option<u8>,
    fps: Option<u8>,
    random_start: Option<bool>,
    rate: Option<f32>,
    life: Option<f32>,
    life_variance: Option<f32>,
    speed: Option<f32>,
    speed_variance: Option<f32>,
    spread: Option<f32>,
    lift: Option<f32>,
    drag: Option<f32>,
    inherit: Option<f32>,
    size: Option<[f32; 4]>,
    colors: Option<[[u8; 4]; 4]>,
    duration: Option<f32>,
}

fn within(value: Option<f32>, default: f32, low: f32, high: f32, field: &'static str) -> Result<f32, ContentError> {
    let value = value.unwrap_or(default);
    if !value.is_finite() || value < low || value > high {
        return Err(ContentError::Field { field, reason: "a number outside the range this field takes" });
    }
    Ok(value)
}

/// Check the effects of a profile.
pub(super) fn check(raw: Vec<RawEffect>) -> Result<Vec<EffectProfile>, ContentError> {
    if raw.len() > MAX_EFFECTS {
        return Err(ContentError::Field { field: "effect", reason: "a profile has at most 8 effects" });
    }
    raw.into_iter().map(effect).collect()
}

fn effect(raw: RawEffect) -> Result<EffectProfile, ContentError> {
    let on = match raw.on.as_str() {
        "start" => EffectOn::Start,
        "burn" => EffectOn::Burn,
        "pop" => EffectOn::Pop,
        _ => return Err(ContentError::Field { field: "effect.on", reason: "start, burn or pop" }),
    };
    let additive = match raw.blend.as_deref() {
        None | Some("additive") => true,
        Some("alpha") => false,
        Some(_) => return Err(ContentError::Field { field: "effect.blend", reason: "additive or alpha" }),
    };
    let grid = raw.grid.unwrap_or(0);
    if !matches!(grid, 0 | 2 | 4 | 8 | 16) {
        return Err(ContentError::Field { field: "effect.grid", reason: "0, 2, 4, 8 or 16" });
    }
    if !is_png_path(&raw.texture) {
        return Err(ContentError::Field {
            field: "effect.texture",
            reason: "a relative .PNG path of letters, digits, _ - . /",
        });
    }
    let size = raw.size.unwrap_or([0.2; 4]);
    if size.iter().any(|s| !s.is_finite() || !(0.0..=20.0).contains(s)) {
        return Err(ContentError::Field { field: "effect.size", reason: "four numbers from 0 to 20 metres" });
    }
    Ok(EffectProfile {
        on,
        texture: raw.texture,
        additive,
        grid,
        fps: raw.fps.unwrap_or(24),
        random_start: raw.random_start.unwrap_or(false),
        rate: within(raw.rate, 20.0, 0.0, 500.0, "effect.rate")?,
        life: within(raw.life, 0.3, 0.01, 10.0, "effect.life")?,
        life_variance: within(raw.life_variance, 0.0, 0.0, 10.0, "effect.life_variance")?,
        speed: within(raw.speed, 0.0, -50.0, 50.0, "effect.speed")?,
        speed_variance: within(raw.speed_variance, 0.0, 0.0, 50.0, "effect.speed_variance")?,
        spread: within(raw.spread, 0.0, 0.0, 180.0, "effect.spread")?,
        lift: within(raw.lift, 0.0, -50.0, 50.0, "effect.lift")?,
        drag: within(raw.drag, 0.0, 0.0, 50.0, "effect.drag")?,
        inherit: within(raw.inherit, 0.0, 0.0, 1.0, "effect.inherit")?,
        size,
        colors: raw.colors.unwrap_or([[255; 4]; 4]),
        duration: within(raw.duration, 0.5, 0.0, 10.0, "effect.duration")?,
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
        assert_eq!(flame.size, [0.2; 4], "the size a profile leaves out");
    }

    #[test]
    fn a_bad_effect_is_refused() {
        let bad = |extra: &str| {
            parse_nitrous_profile(&format!("format = 1\n[[effect]]\non = \"burn\"\ntexture = \"A.PNG\"\n{extra}"))
        };
        assert!(bad("").is_ok());
        assert!(matches!(bad("grid = 3"), Err(ContentError::Field { field: "effect.grid", .. })));
        assert!(matches!(bad("blend = \"x\""), Err(ContentError::Field { field: "effect.blend", .. })));
        assert!(matches!(bad("rate = 9999.0"), Err(ContentError::Field { field: "effect.rate", .. })));
        assert!(matches!(bad("size = [0.1, 0.1, 0.1, 99.0]"), Err(ContentError::Field { field: "effect.size", .. })));
        let wrong_on = "format = 1\n[[effect]]\non = \"sometimes\"\ntexture = \"A.PNG\"";
        assert!(matches!(parse_nitrous_profile(wrong_on), Err(ContentError::Field { field: "effect.on", .. })));
        let wrong_file = "format = 1\n[[effect]]\non = \"burn\"\ntexture = \"../A.PNG\"";
        assert!(matches!(parse_nitrous_profile(wrong_file), Err(ContentError::Field { field: "effect.texture", .. })));
        let nine = "[[effect]]\non = \"burn\"\ntexture = \"A.PNG\"\n".repeat(9);
        assert!(matches!(parse_nitrous_profile(&format!("format = 1\n{nine}")), Err(ContentError::Field { .. })));
    }
}
