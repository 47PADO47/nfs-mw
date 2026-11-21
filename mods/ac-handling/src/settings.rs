//! The mod's run-time settings (parameter-mapping.md section 7 and the units of the later specs) and the `config.toml` that sets
//! their first values.
//!
//! The player's choices live in `bb.settings` and `bb.menus`; `config.toml` (read through `bb.config`) only gives the first
//! values and the `substeps` override for testing. A config with an unknown key or a number out of range is rejected whole,
//! with one line in the log, and the defaults stand.

use serde::Deserialize;

use crate::aids::{Levels, MAX_CLICKS, MAX_ESC_LEVEL, MAX_LEVEL};
use crate::presets::Compound;
use crate::steering::Device;

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

/// One optional unit: follow the preset's default, or the player's choice.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum UnitMode {
    /// The default of the car's preset (and tuning file).
    #[default]
    Preset,
    /// Always on.
    On,
    /// Always off.
    Off,
}

impl UnitMode {
    /// The names a setting or a command may use.
    pub const NAMES: [&'static str; 3] = ["preset", "on", "off"];

    /// The mode a name says.
    pub fn parse(name: &str) -> Option<UnitMode> {
        match name {
            "preset" | "default" => Some(UnitMode::Preset),
            "on" => Some(UnitMode::On),
            "off" => Some(UnitMode::Off),
            _ => None,
        }
    }

    /// The name of the mode.
    pub fn name(self) -> &'static str {
        match self {
            UnitMode::Preset => "preset",
            UnitMode::On => "on",
            UnitMode::Off => "off",
        }
    }

    /// Whether the unit is on, given the preset's default.
    pub fn resolve(self, preset_default: bool) -> bool {
        match self {
            UnitMode::Preset => preset_default,
            UnitMode::On => true,
            UnitMode::Off => false,
        }
    }
}

/// The four optional units.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Unit {
    /// The suspension hook.
    Suspension,
    /// Traction control, ABS, stability control, brake bias (the controls hook).
    Aids,
    /// The steering model (the controls hook with the steering flag).
    Steering,
    /// Tyre temperature, pressure and wear.
    Thermal,
}

impl Unit {
    /// Every unit.
    pub const ALL: [Unit; 4] = [Unit::Suspension, Unit::Aids, Unit::Steering, Unit::Thermal];

    /// The name used in settings and commands.
    pub fn name(self) -> &'static str {
        match self {
            Unit::Suspension => "suspension",
            Unit::Aids => "aids",
            Unit::Steering => "steering",
            Unit::Thermal => "thermal",
        }
    }

    /// The unit a name says.
    pub fn parse(name: &str) -> Option<Unit> {
        Unit::ALL.into_iter().find(|unit| unit.name() == name)
    }
}

/// The mode of each optional unit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct UnitModes {
    /// The suspension unit.
    pub suspension: UnitMode,
    /// The aids unit.
    pub aids: UnitMode,
    /// The steering unit.
    pub steering: UnitMode,
    /// The thermal unit.
    pub thermal: UnitMode,
}

impl UnitModes {
    /// The mode of `unit`.
    pub fn get(&self, unit: Unit) -> UnitMode {
        match unit {
            Unit::Suspension => self.suspension,
            Unit::Aids => self.aids,
            Unit::Steering => self.steering,
            Unit::Thermal => self.thermal,
        }
    }

    /// Set the mode of `unit`.
    pub fn set(&mut self, unit: Unit, mode: UnitMode) {
        match unit {
            Unit::Suspension => self.suspension = mode,
            Unit::Aids => self.aids = mode,
            Unit::Steering => self.steering = mode,
            Unit::Thermal => self.thermal = mode,
        }
    }
}

/// The levels the player set; `None` follows the preset's starting level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct LevelSettings {
    /// Traction control.
    pub tc: Option<u32>,
    /// ABS.
    pub abs: Option<u32>,
    /// Stability control.
    pub esc: Option<u32>,
    /// Brake bias clicks.
    pub bias_clicks: Option<i32>,
}

impl LevelSettings {
    /// The effective levels over the preset's starting ones.
    pub fn over(&self, preset: Levels) -> Levels {
        Levels {
            tc: self.tc.unwrap_or(preset.tc),
            abs: self.abs.unwrap_or(preset.abs),
            esc: self.esc.unwrap_or(preset.esc),
            bias_clicks: self.bias_clicks.unwrap_or(preset.bias_clicks),
        }
        .clamped()
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
    /// The optional units.
    pub units: UnitModes,
    /// The aids' levels.
    pub levels: LevelSettings,
    /// The steering device.
    pub device: Device,
    /// The speed sensitivity of the pad's lock, 0 to 1; `None` is the tuning's value.
    pub steer_sens: Option<f64>,
    /// The force-feedback gain; `None` is the tuning's value.
    pub ffb_gain: Option<f64>,
    /// The force-feedback curve exponent; `None` is the tuning's value.
    pub ffb_curve: Option<f64>,
    /// The tyres start warm (as if in blankets) instead of at the ambient temperature.
    pub warm_start: bool,
    /// Show the tyre temperatures on the HUD now and then.
    pub hud: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            enabled: false,
            preset: PresetChoice::Auto,
            grip_gain: 1.0,
            substeps: 0,
            units: UnitModes::default(),
            levels: LevelSettings::default(),
            device: Device::Pad,
            steer_sens: None,
            ffb_gain: None,
            ffb_curve: None,
            warm_start: false,
            hud: false,
        }
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
    suspension: Option<String>,
    aids: Option<String>,
    steering: Option<String>,
    thermal: Option<String>,
    warm_start: Option<bool>,
    hud: Option<bool>,
    device: Option<String>,
    tc: Option<u32>,
    abs: Option<u32>,
    esc: Option<u32>,
    brake_bias_clicks: Option<i32>,
}

/// One unit mode of a config text.
fn unit_mode(name: &str, value: Option<String>) -> Result<Option<UnitMode>, String> {
    let Some(value) = value else {
        return Ok(None);
    };
    UnitMode::parse(&value)
        .map(Some)
        .ok_or_else(|| format!("{name} = \"{value}\" is not one of {}", UnitMode::NAMES.join(", ")))
}

/// A level of a config text inside its range.
fn level(name: &str, value: Option<u32>, max: u32) -> Result<Option<u32>, String> {
    match value {
        Some(found) if found > max => Err(format!("{name} = {found} is outside 0 to {max}")),
        other => Ok(other),
    }
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
    for (unit, value) in [
        (Unit::Suspension, raw.suspension),
        (Unit::Aids, raw.aids),
        (Unit::Steering, raw.steering),
        (Unit::Thermal, raw.thermal),
    ] {
        if let Some(mode) = unit_mode(unit.name(), value)? {
            settings.units.set(unit, mode);
        }
    }
    if let Some(name) = raw.device {
        settings.device = Device::parse(&name).ok_or_else(|| format!("device = \"{name}\" is not pad or wheel"))?;
    }
    settings.warm_start = raw.warm_start.unwrap_or(false);
    settings.hud = raw.hud.unwrap_or(false);
    settings.levels.tc = level("tc", raw.tc, MAX_LEVEL)?;
    settings.levels.abs = level("abs", raw.abs, MAX_LEVEL)?;
    settings.levels.esc = level("esc", raw.esc, MAX_ESC_LEVEL)?;
    if let Some(clicks) = raw.brake_bias_clicks {
        if clicks.abs() > MAX_CLICKS {
            return Err(format!("brake_bias_clicks = {clicks} is outside -{MAX_CLICKS} to {MAX_CLICKS}"));
        }
        settings.levels.bias_clicks = Some(clicks);
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
