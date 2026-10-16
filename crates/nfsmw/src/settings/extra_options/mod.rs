//! The Extra Options settings file (`NFSMWExtraOptionsSettings.ini`, the format of the NFS:MW Extra
//! Options mod), read as the lowest settings layer. Only the options this port implements are
//! applied; the rest are logged once so nothing is silently dropped. Spec: `docs/specs/extra-options.md`.

mod breaker;
mod ini;
mod pursuit;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_breaker;
#[cfg(test)]
mod tests_pursuit;

pub use ini::{entries, number, switch};

use std::path::Path;
use std::sync::OnceLock;

use super::{Partial, Percent, Settings};

/// The mod's default `WorldAnimationSpeed`, which is the original rate in its own unit.
const MOD_ANIMATION_DEFAULT: f32 = 45.0;

/// The file's name, next to the config file (the mod's own name for it).
pub const FILE_NAME: &str = "NFSMWExtraOptionsSettings.ini";

/// The settings layer from the file at `path`; a missing or unreadable file is no layer.
pub fn read(path: &Path) -> Partial {
    let Ok(text) = std::fs::read_to_string(path) else { return Partial::default() };
    let parsed = ExtraOptions::parse(&text);
    if !parsed.unsupported.is_empty() {
        log::info!("extra options not implemented, ignored: {}", parsed.unsupported.join(", "));
    }
    parsed.options.layer()
}

/// The options from the file next to the config file, read once. The Speedbreaker and the pursuit take their values
/// from here, not from a settings layer. A missing file, or no config folder, leaves every option unset.
pub(crate) fn active() -> ExtraOptions {
    static ACTIVE: OnceLock<ExtraOptions> = OnceLock::new();
    *ACTIVE.get_or_init(|| read_active().unwrap_or_default())
}

fn read_active() -> Option<ExtraOptions> {
    let path = Settings::config_path()?.parent()?.join(FILE_NAME);
    let text = std::fs::read_to_string(path).ok()?;
    Some(ExtraOptions::parse(&text).options)
}

/// The options this port applies. `None` means the file left it unset.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct ExtraOptions {
    /// `[Misc] SkipMovies`: skip the cutscenes at start-up, like `skip_intro`.
    pub skip_movies: Option<bool>,
    /// `[Weather] AlwaysRain`: rain on every track.
    pub always_rain: Option<bool>,
    /// `[Weather] GeneralRainAmount`, as a percentage of the original amount, 0 to 100.
    pub rain_amount: Option<Percent>,
    /// `[Gameplay] GameSpeed`: the physics clock multiplier (1 is the original speed).
    pub game_speed: Option<f32>,
    /// `[Gameplay] WorldAnimationSpeed`: the multiplier of the world animation clock.
    pub animation_speed: Option<f32>,
    /// `[Gameplay] CarScale`: the player car's model size (1 is the original).
    pub car_scale: Option<f32>,
    /// `[Gameplay] NoRevLimiter`: lift the rev limiter.
    pub no_rev_limiter: Option<bool>,
    /// `[Gameplay] ShowHUDOnLookBack`: keep the HUD up while looking back.
    pub show_hud_on_look_back: Option<bool>,
    /// `[Gameplay] ShowTurnSignals`: show turn signals on cars while steering.
    pub turn_signals: Option<bool>,
    /// `[Hotkeys] Headlights`: a Windows virtual-key code.
    pub headlights_key: Option<u32>,
    /// `[Misc] EnableSound` and `EnableMusic`: `Some(false)` mutes everything or the music.
    pub sound: Option<bool>,
    pub music: Option<bool>,
    /// `[Gameplay] SBRechargeTime`: seconds for a Speedbreaker recharge to fill the meter.
    pub sb_recharge_seconds: Option<f32>,
    /// `[Gameplay] SBRechargeSpeedLimit`: the speed in mph above which the Speedbreaker recharges.
    pub sb_recharge_mph: Option<f32>,
    /// `[Gameplay] SBMassMultiplier`: the car's collision mass scale while the Speedbreaker is active.
    pub sb_mass_multiplier: Option<f32>,
    /// `[Pursuit] HeatLevelOverride`: force the heat range of every chase.
    pub heat_override: Option<bool>,
    /// `[Pursuit] MinimumHeatLevel` and `MaximumHeatLevel`: the forced heat range, when the override is on.
    pub heat_min: Option<f32>,
    pub heat_max: Option<f32>,
    /// `[Pursuit] SpeedingLimit`: the speed (m/s) over which the Speeding infraction is recorded.
    pub speeding_limit_mps: Option<f32>,
    /// `[Pursuit] RecklessDrivingLimit`: the speed (m/s) over which the Reckless Driving infraction is recorded.
    pub reckless_limit_mps: Option<f32>,
}

/// The file's keys that name an option this port does not implement yet, in file order.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Parsed {
    pub options: ExtraOptions,
    pub unsupported: Vec<String>,
}

impl ExtraOptions {
    /// Read the options from the text of a settings file.
    pub fn parse(text: &str) -> Parsed {
        let mut parsed = Parsed::default();
        for entry in entries(text) {
            let options = &mut parsed.options;
            match (entry.section.as_str(), entry.key.as_str()) {
                ("Misc", "SkipMovies") => options.skip_movies = switch(&entry.value),
                ("Misc", "EnableSound") => options.sound = switch(&entry.value),
                ("Misc", "EnableMusic") => options.music = switch(&entry.value),
                ("Weather", "AlwaysRain") => options.always_rain = switch(&entry.value),
                ("Weather", "GeneralRainAmount") => options.rain_amount = rain_percent(&entry.value),
                ("Gameplay", "GameSpeed") => options.game_speed = number(&entry.value),
                ("Gameplay", "WorldAnimationSpeed") => {
                    options.animation_speed = number(&entry.value).map(|speed| speed / MOD_ANIMATION_DEFAULT)
                }
                ("Gameplay", "NoRevLimiter") => options.no_rev_limiter = switch(&entry.value),
                ("Gameplay", "ShowHUDOnLookBack") => options.show_hud_on_look_back = switch(&entry.value),
                ("Gameplay", "ShowTurnSignals") => options.turn_signals = switch(&entry.value),
                ("Hotkeys", "Headlights") => options.headlights_key = number(&entry.value).map(|code| code as u32),
                ("Gameplay", "CarScale") => options.car_scale = number(&entry.value),
                ("Gameplay", "SBRechargeTime") => options.sb_recharge_seconds = number(&entry.value),
                ("Gameplay", "SBRechargeSpeedLimit") => options.sb_recharge_mph = number(&entry.value),
                ("Gameplay", "SBMassMultiplier") => options.sb_mass_multiplier = number(&entry.value),
                ("Pursuit", "HeatLevelOverride") => options.heat_override = switch(&entry.value),
                ("Pursuit", "MinimumHeatLevel") => options.heat_min = number(&entry.value),
                ("Pursuit", "MaximumHeatLevel") => options.heat_max = number(&entry.value),
                ("Pursuit", "SpeedingLimit") => options.speeding_limit_mps = number(&entry.value),
                ("Pursuit", "RecklessDrivingLimit") => options.reckless_limit_mps = number(&entry.value),
                _ => parsed.unsupported.push(format!("{}.{}", entry.section, entry.key)),
            }
        }
        parsed
    }

    /// The settings layer these options form: unset fields fall through to the defaults.
    pub fn layer(&self) -> Partial {
        Partial {
            skip_intro: self.skip_movies,
            always_rain: self.always_rain,
            rain_amount: self.rain_amount,
            game_speed: self.game_speed,
            animation_speed: self.animation_speed,
            car_scale: self.car_scale,
            no_rev_limiter: self.no_rev_limiter,
            show_hud_on_look_back: self.show_hud_on_look_back,
            turn_signals: self.turn_signals,
            headlights_key: self.headlights_key,
            master_volume: (self.sound == Some(false)).then_some(Percent(0)),
            music_volume: (self.music == Some(false)).then_some(Percent(0)),
            ..Partial::default()
        }
    }
}

/// The mod's rain multiplier (1 is the original amount) as a percentage; values above 1 clamp to 100.
fn rain_percent(value: &str) -> Option<Percent> {
    let amount = number(value)?;
    Some(Percent((amount * 100.0).round().clamp(0.0, 100.0) as u8))
}
