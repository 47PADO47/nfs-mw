//! A nitrous effect profile: what a mod's nitrous variant looks and sounds like while the nitrous burns
//! (`docs/modding/nitrous-variants.md`).
//!
//! The file is `NITROUS/<value id>/profile.toml` in the mod's `files/` folder, where `<value id>` is the id of the
//! variant the mod adds to the `nitrous_variant` option (`bb.settings`). The sound files it names are 16-bit PCM WAV in the
//! same folder tree. Everything is optional; an effect the profile leaves out is the game's own.
//!
//! ```toml
//! format = 1
//!
//! [audio]                       # sounds; `replace_original` silences the game's own nitrous sounds while this plays
//! start = "NITROUS/MW2012/START.WAV"   # played once when the nitrous lights
//! loop = "NITROUS/MW2012/LOOP.WAV"     # played over the whole burn, looping
//! end = "NITROUS/MW2012/END.WAV"       # played once when it stops
//! start_gain = 1.0
//! loop_gain = 1.0
//! end_gain = 1.0
//! loop_fade_in = 0.1            # seconds
//! loop_fade_out = 0.25
//! replace_original = true
//!
//! [flame]                       # the tail pipe flames, drawn from the game's emitters with this tint
//! tint = [0.4, 0.6, 1.0]        # multiplies the red, green and blue of the flame's particles
//! gain = 1.0                    # multiplies their alpha
//! replace_burn = false          # the game's own blue flame is not drawn while an `on = "burn"` effect plays
//! replace_pop = false           # the game's blow-off and backfire flame is not drawn while an `on = "pop"` effect plays
//!
//! [[effect]]                    # a particle emitter at every tail pipe, see `nitrous_effects`
//! on = "burn"                   # "start", "burn" or "pop"
//! texture = "NITROUS/MW2012/GLOW.PNG"
//!
//! [screen]                     # screen and camera effects that follow the burn, see `nitrous_screen`
//! vignette = 0.2                # strength 0 to 1 at full effect
//! radial_blur = 0.3             # strength 0 to 1 at full effect
//! fov = 8.0                     # degrees added to the chase camera's field of view
//! fade_in = 0.25                # seconds to reach full effect
//! fade_out = 1.0                # seconds to leave it
//! ```

use serde::Deserialize;

use super::ContentError;
use super::engine_sound::is_sound_path;
use super::nitrous_effects::{self as effects, EffectProfile, RawEffect};

pub use super::variant::PROFILE_FILE;
use super::variant::{self, NITROUS};

#[path = "nitrous_screen.rs"]
mod screen;
pub use screen::ScreenProfile;

/// The folder under a mod's `files/` that holds the variants' files.
pub const NITROUS_FOLDER: &str = NITROUS.folder;

/// The path of the profile of variant `value` in the install's layout (the overlay of the mod's `files/` folder).
pub fn profile_path(value: &str) -> String {
    NITROUS.profile_path(value)
}

/// The sounds of a variant.
#[derive(Debug, Clone, PartialEq)]
pub struct AudioProfile {
    pub start: Option<String>,
    pub looped: Option<String>,
    pub end: Option<String>,
    pub start_gain: f32,
    pub loop_gain: f32,
    pub end_gain: f32,
    /// Seconds the loop takes to reach full level, and to die away after the burn.
    pub loop_fade_in: f32,
    pub loop_fade_out: f32,
    /// The game's own nitrous sounds are silent while the profile plays.
    pub replace_original: bool,
}

/// The tint of the pipe flames.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FlameProfile {
    pub tint: [f32; 3],
    pub gain: f32,
    /// The game's blue nitrous flame is not drawn while a `burn` effect plays.
    pub replace_burn: bool,
    /// The game's blow-off and backfire flame is not drawn while a `pop` effect plays.
    pub replace_pop: bool,
}

/// A variant's effects.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct NitrousProfile {
    pub audio: Option<AudioProfile>,
    pub flame: Option<FlameProfile>,
    pub screen: Option<ScreenProfile>,
    /// The particle effects, in file order (`nitrous_effects`).
    pub effects: Vec<EffectProfile>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Raw {
    format: u32,
    audio: Option<RawAudio>,
    flame: Option<RawFlame>,
    screen: Option<screen::RawScreen>,
    #[serde(default)]
    effect: Vec<RawEffect>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawAudio {
    start: Option<String>,
    #[serde(rename = "loop")]
    looped: Option<String>,
    end: Option<String>,
    start_gain: Option<f32>,
    loop_gain: Option<f32>,
    end_gain: Option<f32>,
    loop_fade_in: Option<f32>,
    loop_fade_out: Option<f32>,
    replace_original: Option<bool>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawFlame {
    tint: Option<[f32; 3]>,
    gain: Option<f32>,
    replace_burn: Option<bool>,
    replace_pop: Option<bool>,
}

const GAIN_RANGE: std::ops::RangeInclusive<f32> = 0.0..=4.0;
const FADE_RANGE: std::ops::RangeInclusive<f32> = 0.0..=10.0;
const UNIT_RANGE: std::ops::RangeInclusive<f32> = 0.0..=1.0;

/// Parse and check a profile. A number out of range, a sound path that is not a `.WAV` path, a key the schema does not
/// know or another format number is refused with the reason.
pub fn parse(text: &str) -> Result<NitrousProfile, ContentError> {
    let raw: Raw = toml::from_str(text).map_err(|error| ContentError::Parse(error.to_string()))?;
    variant::check_format(raw.format)?;
    Ok(NitrousProfile {
        audio: raw.audio.map(audio).transpose()?,
        flame: raw.flame.map(flame).transpose()?,
        screen: raw.screen.map(screen::parse).transpose()?,
        effects: effects::check(raw.effect)?,
    })
}

fn within(
    value: Option<f32>,
    default: f32,
    range: &std::ops::RangeInclusive<f32>,
    field: &'static str,
) -> Result<f32, ContentError> {
    let value = value.unwrap_or(default);
    if !value.is_finite() || !range.contains(&value) {
        return Err(ContentError::Field { field, reason: "a number outside the range this field takes" });
    }
    Ok(value)
}

fn sound(path: Option<String>, field: &'static str) -> Result<Option<String>, ContentError> {
    let Some(path) = path else { return Ok(None) };
    if !is_sound_path(&path) {
        return Err(ContentError::Field {
            field,
            reason: "a sound is a relative .WAV path of letters, digits, _ - . /",
        });
    }
    Ok(Some(path))
}

fn audio(raw: RawAudio) -> Result<AudioProfile, ContentError> {
    let profile = AudioProfile {
        start: sound(raw.start, "audio.start")?,
        looped: sound(raw.looped, "audio.loop")?,
        end: sound(raw.end, "audio.end")?,
        start_gain: within(raw.start_gain, 1.0, &GAIN_RANGE, "audio.start_gain")?,
        loop_gain: within(raw.loop_gain, 1.0, &GAIN_RANGE, "audio.loop_gain")?,
        end_gain: within(raw.end_gain, 1.0, &GAIN_RANGE, "audio.end_gain")?,
        loop_fade_in: within(raw.loop_fade_in, 0.1, &FADE_RANGE, "audio.loop_fade_in")?,
        loop_fade_out: within(raw.loop_fade_out, 0.25, &FADE_RANGE, "audio.loop_fade_out")?,
        replace_original: raw.replace_original.unwrap_or(true),
    };
    if profile.start.is_none() && profile.looped.is_none() && profile.end.is_none() {
        return Err(ContentError::Field { field: "audio", reason: "names no sound" });
    }
    Ok(profile)
}

fn flame(raw: RawFlame) -> Result<FlameProfile, ContentError> {
    let tint = raw.tint.unwrap_or([1.0; 3]);
    if tint.iter().any(|c| !c.is_finite() || !GAIN_RANGE.contains(c)) {
        return Err(ContentError::Field { field: "flame.tint", reason: "three numbers from 0 to 4" });
    }
    Ok(FlameProfile {
        tint,
        gain: within(raw.gain, 1.0, &GAIN_RANGE, "flame.gain")?,
        replace_burn: raw.replace_burn.unwrap_or(false),
        replace_pop: raw.replace_pop.unwrap_or(false),
    })
}

/// Follows a burn: a level that rises to 1 over `fade_in` seconds while the nitrous burns and falls to 0 over `fade_out`
/// after, and the two edges. Pure, so the host and its tests step it the same way.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct BurnFollower {
    level: f32,
    burning: bool,
}

/// What one [`BurnFollower::step`] saw.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BurnStep {
    /// The level after the step, 0 to 1.
    pub level: f32,
    /// The burn began in this step.
    pub started: bool,
    /// The burn stopped in this step.
    pub ended: bool,
}

impl BurnFollower {
    /// The level now, 0 to 1.
    pub fn level(&self) -> f32 {
        self.level
    }

    /// Whether the last step saw the nitrous burning.
    pub fn burning(&self) -> bool {
        self.burning
    }

    /// Advance by `dt` seconds with the nitrous `burning` or not. A fade of 0 seconds is instant.
    pub fn step(&mut self, burning: bool, fade_in: f32, fade_out: f32, dt: f32) -> BurnStep {
        let started = burning && !self.burning;
        let ended = !burning && self.burning;
        self.burning = burning;
        let dt = dt.max(0.0);
        self.level = match burning {
            true => ramp(self.level, 1.0, fade_in, dt),
            false => ramp(self.level, 0.0, fade_out, dt),
        };
        BurnStep { level: self.level, started, ended }
    }
}

/// `level` moved towards `target` by `dt` over a fade of `seconds` (a whole 0 to 1 sweep takes that long).
fn ramp(level: f32, target: f32, seconds: f32, dt: f32) -> f32 {
    if seconds <= 0.0 {
        return target;
    }
    let step = dt / seconds;
    match target > level {
        true => (level + step).min(target),
        false => (level - step).max(target),
    }
}

#[cfg(test)]
#[path = "nitrous_fx_tests.rs"]
mod tests;
