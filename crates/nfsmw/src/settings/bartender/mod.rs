//! The pursuit tunables, read from `NFSMWBartenderSettings.ini` next to the config file: the cop cap and the cop spawn
//! limit, heat growth, the bust rules, the escape's heat loss and the infraction fines. The reader is `ini-tuning`; this
//! file holds the NFS:MW schema, its ranges and the vanilla values (in [`vanilla`]).
//!
//! The file is our own schema. Its keys are not the keys of the Bartender mod, whose names and defaults were not read
//! (`docs/mods/nfsmwbartender.md` section 4), so a Bartender file does not load here.
//!
//! Layout:
//! - `[Cops] MaxCops`: the most cop cars chasing at once.
//! - `[Heat1]` to `[Heat10]`: per heat level `CopCap` (the cop count the wave wants; without it the install's
//!   `FullEngagementCopCount` of the level), `TimePerLevel` (seconds of heat growth) and `BustSpeed` (km/h).
//! - `[Heat] EvadeAdjust`: the share of the heat a chase keeps after an escape.
//! - `[Fines]`: the fine of each infraction, one key each (`Speeding`, `Racing`, `RecklessDriving`, `Assault`,
//!   `HitAndRun`, `DamageToProperty`, `ResistingArrest`, `OffRoad`).
//! - `[Busted] Timeout`, `Reach`, `HeightGate`: the busted timer's constants (the flashing reach is FairBusts' setting).
//!
//! Rules: a missing file is vanilla throughout; a value outside its range is clamped to the nearest bound; a malformed
//! value is vanilla. Each is logged, so the game still starts.

use std::path::Path;

use blackbox_pursuit::Fines;
use ini_tuning::{Entry, Range, Read, entries, read_levels, read_number};

pub mod vanilla;

/// The file's name, next to the config file (our own name for it).
pub const FILE_NAME: &str = "NFSMWBartenderSettings.ini";

/// The heat levels the per-level keys cover.
pub const LEVELS: usize = 10;

/// One heat level's tunables.
#[derive(Debug, Clone, PartialEq)]
pub struct Level {
    /// The cop cars the wave wants at this level. `None` keeps the install's row value (`FullEngagementCopCount`).
    pub cop_cap: Option<u32>,
    /// Seconds of heat growth per level, while the chase is at this level.
    pub time_per_level: f32,
    /// The speed under which the player is busted at this level, km/h.
    pub bust_speed_kph: f32,
}

/// The busted timer's constants that the file sets.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BustedRules {
    /// Seconds of the timer past which the player is busted.
    pub timeout: f32,
    /// Horizontal reach of a height-matched cop, metres, while the player is not flashing.
    pub reach: f32,
    /// Maximum height difference, metres, for a cop to count.
    pub height_gate: f32,
}

/// Every pursuit tunable the file sets. The defaults are the vanilla values.
#[derive(Debug, Clone, PartialEq)]
pub struct BartenderSettings {
    pub max_cops: u32,
    /// The heat levels 1 to 10.
    pub levels: [Level; LEVELS],
    /// The share of the heat a chase keeps after an escape.
    pub evade_heat_adjust: f32,
    pub fines: Fines,
    pub busted: BustedRules,
}

impl Default for Level {
    fn default() -> Self {
        Self { cop_cap: None, time_per_level: vanilla::TIME_PER_LEVEL[0], bust_speed_kph: vanilla::BUST_SPEED_KPH[0] }
    }
}

impl Default for BustedRules {
    fn default() -> Self {
        Self { timeout: vanilla::BUST_TIMEOUT, reach: vanilla::BUST_REACH, height_gate: vanilla::HEIGHT_GATE }
    }
}

impl Default for BartenderSettings {
    fn default() -> Self {
        Self {
            max_cops: vanilla::MAX_COPS,
            levels: std::array::from_fn(|i| Level {
                cop_cap: None,
                time_per_level: vanilla::TIME_PER_LEVEL[i],
                bust_speed_kph: vanilla::BUST_SPEED_KPH[i],
            }),
            evade_heat_adjust: vanilla::EVADE_HEAT_ADJUST,
            fines: Fines::new(vanilla::FINES),
            busted: BustedRules::default(),
        }
    }
}

impl BartenderSettings {
    /// The tunables of heat `level`, clamped to `1..=10`.
    pub fn level(&self, level: u32) -> &Level {
        &self.levels[(level as usize).clamp(1, LEVELS) - 1]
    }

    /// Seconds of heat growth per level at `level`.
    pub fn time_per_level(&self, level: u32) -> f32 {
        self.level(level).time_per_level
    }

    /// The speed under which the player is busted at `level`, metres per second.
    pub fn bust_speed_mps(&self, level: u32) -> f32 {
        self.level(level).bust_speed_kph / KPH_PER_MPS
    }

    /// The cop cap the file sets at `level`, if any.
    pub fn cop_cap(&self, level: u32) -> Option<u32> {
        self.level(level).cop_cap
    }
}

/// Kilometres per hour in one metre per second.
const KPH_PER_MPS: f32 = 3.6;

/// Cop cars.
const COPS: Range = Range { min: 1.0, max: 64.0 };
/// Seconds of heat growth. The floor keeps a zero rate from dividing by zero.
const SECONDS: Range = Range { min: 1.0, max: 100_000.0 };
/// Km/h.
const KPH: Range = Range { min: 0.0, max: 200.0 };
/// A share of the heat.
const SHARE: Range = Range { min: 0.0, max: 2.0 };
/// A fine, in dollars.
const FINE: Range = Range { min: 0.0, max: 1_000_000.0 };
/// The busted timer's seconds.
const TIMEOUT: Range = Range { min: 0.1, max: 60.0 };
/// Metres of reach.
const METRES: Range = Range { min: 1.0, max: 500.0 };
/// Metres of height.
const HEIGHT: Range = Range { min: 0.1, max: 10.0 };

/// The infraction keys of `[Fines]`, in `Infraction::ALL` order.
const FINE_KEYS: [&str; 8] =
    ["Speeding", "Racing", "RecklessDriving", "Assault", "HitAndRun", "DamageToProperty", "ResistingArrest", "OffRoad"];

/// The settings in the file next to the config file. A missing file is the vanilla tuning.
pub fn load() -> BartenderSettings {
    match crate::settings::Settings::config_path().as_deref().and_then(Path::parent) {
        Some(dir) => read(&dir.join(FILE_NAME)),
        None => BartenderSettings::default(),
    }
}

/// The settings in the file at `path`. A missing file is the vanilla tuning.
pub fn read(path: &Path) -> BartenderSettings {
    match std::fs::read_to_string(path) {
        Ok(text) => parse(&text),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => BartenderSettings::default(),
        Err(e) => {
            log::warn!("cannot read {}: {e}", path.display());
            BartenderSettings::default()
        }
    }
}

/// The tunables from the text of the file.
pub fn parse(text: &str) -> BartenderSettings {
    let e = entries(text);
    let d = BartenderSettings::default();
    let caps = read_levels::<LEVELS>(&e, "Heat", "CopCap", COPS);
    let times = read_levels::<LEVELS>(&e, "Heat", "TimePerLevel", SECONDS);
    let speeds = read_levels::<LEVELS>(&e, "Heat", "BustSpeed", KPH);
    let levels = std::array::from_fn(|i| {
        let section = ini_tuning::section_name("Heat", i + 1);
        Level {
            cop_cap: note(&section, "CopCap", caps[i]).map(|v| v.round() as u32),
            time_per_level: note(&section, "TimePerLevel", times[i]).unwrap_or(d.levels[i].time_per_level),
            bust_speed_kph: note(&section, "BustSpeed", speeds[i]).unwrap_or(d.levels[i].bust_speed_kph),
        }
    });
    BartenderSettings {
        max_cops: pick(&e, "Cops", "MaxCops", COPS, d.max_cops as f32).round() as u32,
        levels,
        evade_heat_adjust: pick(&e, "Heat", "EvadeAdjust", SHARE, d.evade_heat_adjust),
        fines: Fines::new(fines(&e, &d.fines)),
        busted: BustedRules {
            timeout: pick(&e, "Busted", "Timeout", TIMEOUT, d.busted.timeout),
            reach: pick(&e, "Busted", "Reach", METRES, d.busted.reach),
            height_gate: pick(&e, "Busted", "HeightGate", HEIGHT, d.busted.height_gate),
        },
    }
}

/// The fines of `[Fines]`, one key per infraction; an absent or bad key keeps its vanilla fine.
fn fines(e: &[Entry], default: &Fines) -> [u32; 8] {
    std::array::from_fn(|i| {
        let read = read_number(e, "Fines", FINE_KEYS[i], FINE);
        note("Fines", FINE_KEYS[i], read).map_or(default.of(blackbox_pursuit::Infraction::ALL[i]), |v| v.round() as u32)
    })
}

/// A key read through `range`, or `default` when absent or malformed.
fn pick(e: &[Entry], section: &str, key: &str, range: Range, default: f32) -> f32 {
    let read = read_number(e, section, key, range);
    note(section, key, read).unwrap_or(default)
}

/// The usable value of `read`, logging what the file got wrong. `None` when absent or malformed.
fn note(section: &str, key: &str, read: Read) -> Option<f32> {
    match read {
        Read::Clamped(v) => log::warn!("bartender: {section}.{key} is out of range, using {v}"),
        Read::Malformed => log::warn!("bartender: {section}.{key} is not a number, using the vanilla value"),
        Read::Absent | Read::Value(_) => {}
    }
    read.value()
}

#[cfg(test)]
mod tests;
