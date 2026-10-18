//! The air-support line groups as switches: which of the pursuit's helicopter, pilot and radio lines may be said.
//! The keys are those of the RevisedAirSupport mod (`[Pilot]` and `[Radio]` sections, the mod's key names); every
//! one defaults to on. The file is `NFSMWAirSupportSettings.ini` next to the config file, read with `ini-tuning`
//! (`[Section]` headers, `Key = value` lines, `//` comments). A missing file is all on; a malformed value is on.
//!
//! The groups that are wired in `cues.rs` are gated by [`AirSupport::allows`]. The other switches are read and held
//! here so the config surface is complete; their TODOs are in `chase.rs`.

use std::path::Path;

use ini_tuning::{Entry, entries, last, switch};

use super::cues::Cue;

/// The file's name, next to the config file.
pub const FILE_NAME: &str = "NFSMWAirSupportSettings.ini";

/// Declares [`AirSupport`]: one bool per key, all defaulting to on, and the parser that reads each key from its
/// section.
macro_rules! air_support {
    ($( [$section:literal] $field:ident = $key:literal; )*) => {
        /// The line groups the chase may say. Each field is one key of the mod's config.
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub struct AirSupport {
            $(
                #[doc = concat!("The `[", $section, "] ", $key, "` switch.")]
                pub $field: bool,
            )*
        }

        impl Default for AirSupport {
            fn default() -> Self {
                Self { $($field: true,)* }
            }
        }

        impl AirSupport {
            fn from_entries(e: &[Entry]) -> Self {
                Self { $($field: flag(e, $section, $key),)* }
            }
        }
    };
}

air_support! {
    ["Pilot"] ram_lines = "RamLines";
    ["Pilot"] search_pattern = "SearchPattern";
    ["Pilot"] spotted_again = "SpottedAgain";
    ["Pilot"] vehicle_report = "VehicleReport";
    ["Pilot"] escaped_car = "EscapedCar";
    ["Pilot"] reacquire = "Reacquire";
    ["Pilot"] been_hit = "BeenHit";
    ["Pilot"] kill_shot = "KillShot";
    ["Pilot"] dead_heli_silence = "DeadHeliSilence";
    ["Pilot"] roadblock_call = "RoadblockCall";
    ["Pilot"] weather_report = "WeatherReport";
    ["Pilot"] answer_911 = "Answer911";
    ["Pilot"] hold_on = "HoldOn";
    ["Pilot"] attack_missed = "AttackMissed";
    ["Radio"] rain_flag = "RainFlag";
    ["Radio"] fast_air_request = "FastAirRequest";
    ["Radio"] fast_air_reminder = "FastAirReminder";
    ["Radio"] no_speech_dropoff = "NoSpeechDropoff";
}

impl AirSupport {
    /// Whether a cue may be said. The base chase lines (lost, regained, heat jump) are never switched off here.
    /// `TimeExpired` is the dispatch's "time expired" at the end of a lost search; it belongs to the mod's
    /// `SearchPattern` group, which the mod describes as the call when the chase cools down. That mapping is our
    /// reading of the mod's description, to be checked in play. `CopDown` as `KillShot` is also a guess: the mod's
    /// kill-shot group may be the helicopter's kill line, not the cop-down one.
    pub fn allows(&self, cue: Cue) -> bool {
        match cue {
            Cue::Lost | Cue::Regained | Cue::HeatJump | Cue::Escaped => true,
            Cue::TimeExpired => self.search_pattern,
            Cue::Rain => self.weather_report,
            Cue::RoadblockCalled => self.roadblock_call,
            Cue::BeenHit => self.been_hit,
            Cue::HeliLost => self.reacquire,
            Cue::HeliSpotted => self.spotted_again,
            Cue::CopDown => self.kill_shot,
        }
    }
}

/// The switch of `section.key`: its last line, or on when absent or not a switch.
fn flag(e: &[Entry], section: &str, key: &str) -> bool {
    let Some(entry) = last(e, section, key) else {
        return true;
    };
    match switch(&entry.value) {
        Some(on) => on,
        None => {
            log::warn!("air support: [{section}] {key} = {:?} is not a switch; left on", entry.value);
            true
        }
    }
}

/// The switches of the text of the config file.
pub fn parse(text: &str) -> AirSupport {
    AirSupport::from_entries(&entries(text))
}

/// The switches in the file at `path`. A missing file is all on.
pub fn read(path: &Path) -> AirSupport {
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(e) => {
            if e.kind() != std::io::ErrorKind::NotFound {
                log::warn!("cannot read {}: {e}", path.display());
            }
            return AirSupport::default();
        }
    };
    parse(&text)
}
