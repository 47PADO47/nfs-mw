//! FairBusts: the busted timer's reset penalty, read from `NFSMWFairBustsSettings.ini` next to the config file. Stock
//! (the install's `kResetPenaltyBlock`, verified in `docs/provenance/fairbusts-reset.md`) lets a manual reset flash the
//! player: a cop within 90 m busts a slow or fast player at four times the fill, with no speed gate. FairBusts moves
//! that back to the normal rules: the normal reach, the speed gate and a fill multiplier the file sets.
//!
//! Keys (section `[General]`, the names of the NFSMWFairBusts mod):
//! - `Enabled`: `0` leaves the stock penalty.
//! - `ResetAutoBust`: `0` gives the flashing player the normal reach (15 m) and the speed gate; `1` keeps the stock reach
//!   (90 m) and skips the gate, so the instant bust of stock returns.
//! - `ResetBustMultiplier`: the flashing fill rate, 0 to 64 (stock 4.0).
//!
//! Rules: a missing file is the default (enabled, normal reach); a malformed switch or number is its default; a number
//! outside its range is clamped. Each is logged.

use std::path::Path;

use ini_tuning::{Entry, Range, Read, entries, last, read_number, switch};

/// The file's name, next to the config file (the mod's own name for it).
pub const FILE_NAME: &str = "NFSMWFairBustsSettings.ini";

/// The section the keys live in.
const SECTION: &str = "General";

/// The stock flashing reach, metres (the reset-range load of `kResetPenaltyBlock`, 90.0).
pub const STOCK_RESET_REACH: f32 = 90.0;

/// The stock flashing fill multiplier (the reset-rate scale of `kResetPenaltyBlock`, 4.0).
pub const STOCK_RESET_MULTIPLIER: f32 = 4.0;

/// The fill multiplier the file asks for by default. Stock is 4.0 and the rule is kept at that value.
const DEFAULT_MULTIPLIER: f32 = 4.0;

/// The range of the fill multiplier (the mod's clamp).
const MULTIPLIER: Range = Range { min: 0.0, max: 64.0 };

/// What the file sets.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FairBustsSettings {
    pub enabled: bool,
    pub reset_auto_bust: bool,
    pub reset_bust_multiplier: f32,
}

/// The reset penalty the busted timer applies while the player is flashing (after a manual reset).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ResetPenalty {
    /// Horizontal reach of a height-matched cop, metres, while flashing.
    pub reach: f32,
    /// Whether the speed gate still applies while flashing.
    pub speed_gate: bool,
    /// The fill multiplier while flashing.
    pub multiplier: f32,
}

impl ResetPenalty {
    /// The install's penalty: 90 m, no speed gate, four times the fill.
    pub const STOCK: ResetPenalty =
        ResetPenalty { reach: STOCK_RESET_REACH, speed_gate: false, multiplier: STOCK_RESET_MULTIPLIER };
}

impl Default for FairBustsSettings {
    fn default() -> Self {
        Self { enabled: true, reset_auto_bust: false, reset_bust_multiplier: DEFAULT_MULTIPLIER }
    }
}

impl FairBustsSettings {
    /// The penalty these settings give. `normal_reach` is the pursuit's normal reach (the Bartender file's `Reach`,
    /// 15 m vanilla), which a fair reset uses. A disabled mod is the stock penalty whatever the other keys say.
    pub fn penalty(&self, normal_reach: f32) -> ResetPenalty {
        if !self.enabled {
            return ResetPenalty::STOCK;
        }
        match self.reset_auto_bust {
            true => {
                ResetPenalty { reach: STOCK_RESET_REACH, speed_gate: false, multiplier: self.reset_bust_multiplier }
            }
            false => ResetPenalty { reach: normal_reach, speed_gate: true, multiplier: self.reset_bust_multiplier },
        }
    }
}

/// The settings in the file next to the config file. A missing file is the default.
pub fn load() -> FairBustsSettings {
    match crate::settings::Settings::config_path().as_deref().and_then(Path::parent) {
        Some(dir) => read(&dir.join(FILE_NAME)),
        None => FairBustsSettings::default(),
    }
}

/// The settings in the file at `path`. A missing file is the default.
pub fn read(path: &Path) -> FairBustsSettings {
    match std::fs::read_to_string(path) {
        Ok(text) => parse(&text),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => FairBustsSettings::default(),
        Err(e) => {
            log::warn!("cannot read {}: {e}", path.display());
            FairBustsSettings::default()
        }
    }
}

/// The settings from the text of the file.
pub fn parse(text: &str) -> FairBustsSettings {
    let e = entries(text);
    let d = FairBustsSettings::default();
    FairBustsSettings {
        enabled: flag(&e, "Enabled", d.enabled),
        reset_auto_bust: flag(&e, "ResetAutoBust", d.reset_auto_bust),
        reset_bust_multiplier: number(&e, "ResetBustMultiplier", d.reset_bust_multiplier),
    }
}

/// A switch key, or `default` when absent or not a switch.
fn flag(e: &[Entry], key: &str, default: bool) -> bool {
    let Some(entry) = last(e, SECTION, key) else { return default };
    match switch(&entry.value) {
        Some(value) => value,
        None => {
            log::warn!("fairbusts: {SECTION}.{key} is not a switch, using {default}");
            default
        }
    }
}

/// A number key clamped to [`MULTIPLIER`], or `default` when absent or malformed.
fn number(e: &[Entry], key: &str, default: f32) -> f32 {
    let read = read_number(e, SECTION, key, MULTIPLIER);
    match read {
        Read::Clamped(v) => log::warn!("fairbusts: {SECTION}.{key} is out of range, using {v}"),
        Read::Malformed => log::warn!("fairbusts: {SECTION}.{key} is not a number, using {default}"),
        Read::Absent | Read::Value(_) => {}
    }
    read.value().unwrap_or(default)
}

#[cfg(test)]
#[path = "fairbusts_tests.rs"]
mod tests;
