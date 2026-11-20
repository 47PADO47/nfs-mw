//! The mod's run-time settings (parameter-mapping.md section 7) and the `config.toml` that sets their first values.
//!
//! The player's choices live in `bb.settings` and `bb.menus`; `config.toml` (read through `bb.config`) only gives the first
//! values and the `substeps` override for testing. A config with an unknown key or a number out of range is rejected whole,
//! with one line in the log, and the defaults stand.

use serde::Deserialize;

use crate::presets::Compound;

/// The lowest grip gain the menu offers.
pub const GRIP_GAIN_MIN: f64 = 0.5;
/// The highest grip gain the menu offers.
pub const GRIP_GAIN_MAX: f64 = 1.5;
/// The most substeps a setting may ask for.
pub const SUBSTEPS_MAX: u32 = 24;

/// The `preset` setting: pick the compound by the car's grip, or force one for every car.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PresetChoice {
    /// The front axle's grip level picks the compound.
    #[default]
    Auto,
    /// This compound for every car.
    Forced(Compound),
}

impl PresetChoice {
    /// The names a setting or a command may use: `auto` and the five compounds.
    pub const NAMES: [&'static str; 6] = ["auto", "economy", "road", "sport", "semislick", "slick"];

    /// The choice a name says.
    pub fn parse(name: &str) -> Option<PresetChoice> {
        if name == "auto" {
            return Some(PresetChoice::Auto);
        }
        Compound::parse(name).map(PresetChoice::Forced)
    }

    /// The name of the choice.
    pub fn name(self) -> &'static str {
        match self {
            PresetChoice::Auto => "auto",
            PresetChoice::Forced(compound) => compound.name(),
        }
    }

    /// The compound the mapping is forced to, if any.
    pub fn forced(self) -> Option<Compound> {
        match self {
            PresetChoice::Auto => None,
            PresetChoice::Forced(compound) => Some(compound),
        }
    }
}

/// The values of the run-time settings.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Settings {
    /// The master switch.
    pub enabled: bool,
    /// The compound choice.
    pub preset: PresetChoice,
    /// Multiplies the grip of every tyre; 1 is as tuned.
    pub grip_gain: f64,
    /// 0 is automatic, 1 to 24 forces the number of substeps per tick.
    pub substeps: u32,
}

impl Default for Settings {
    fn default() -> Self {
        Settings { enabled: false, preset: PresetChoice::Auto, grip_gain: 1.0, substeps: 0 }
    }
}

/// The shape of `config.toml`.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawConfig {
    enabled: Option<bool>,
    preset: Option<String>,
    grip_gain: Option<f64>,
    substeps: Option<u32>,
}

/// The settings a config text gives, over the defaults. The error is one line naming the key.
pub fn parse_config(text: &str) -> Result<Settings, String> {
    let raw: RawConfig = toml::from_str(text).map_err(|error| error.message().to_owned())?;
    let mut settings = Settings::default();
    if let Some(enabled) = raw.enabled {
        settings.enabled = enabled;
    }
    if let Some(name) = raw.preset {
        settings.preset = PresetChoice::parse(&name)
            .ok_or_else(|| format!("preset = \"{name}\" is not one of {}", PresetChoice::NAMES.join(", ")))?;
    }
    if let Some(gain) = raw.grip_gain {
        if !gain.is_finite() || !(GRIP_GAIN_MIN..=GRIP_GAIN_MAX).contains(&gain) {
            return Err(format!("grip_gain = {gain} is outside {GRIP_GAIN_MIN} to {GRIP_GAIN_MAX}"));
        }
        settings.grip_gain = gain;
    }
    if let Some(substeps) = raw.substeps {
        if substeps > SUBSTEPS_MAX {
            return Err(format!("substeps = {substeps} is outside 0 to {SUBSTEPS_MAX}"));
        }
        settings.substeps = substeps;
    }
    Ok(settings)
}

/// Keep a grip gain from a slider or a command inside its range; a number that is not finite is 1.
pub fn clamp_grip_gain(value: f64) -> f64 {
    if !value.is_finite() {
        return 1.0;
    }
    value.clamp(GRIP_GAIN_MIN, GRIP_GAIN_MAX)
}
