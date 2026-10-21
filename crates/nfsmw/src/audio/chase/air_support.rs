//! The air-support line groups as switches: which of the pursuit's helicopter, pilot and radio lines may be said.
//! The keys are those of the RevisedAirSupport mod (`[Pilot]` and `[Radio]` sections, the mod's key names); every
//! one defaults to on. The file is `NFSMWAirSupportSettings.ini` next to the config file, read with `ini-tuning`
//! (`[Section]` headers, `Key = value` lines, `//` comments). A missing file is all on; a malformed value is on.
//!
//! The lines the radio machine asks for are gated by [`AirSupport::allows_line`], the weather cue of `cues.rs` by
//! [`AirSupport::allows`]. The other switches are read and held here so the config surface is complete; their TODOs are
//! in `chase.rs`.

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
    /// Whether the weather cue may be said (the `WeatherReport` switch).
    pub fn allows(&self, cue: Cue) -> bool {
        match cue {
            Cue::Rain => self.weather_report,
        }
    }

    /// Whether the radio machine may ask for `line`: the lines that have a switch obey it (the same readings of the mod's
    /// groups as [`AirSupport::allows`]); every other line is always allowed.
    pub fn allows_line(&self, line: blackbox_pursuit::speech::Line) -> bool {
        use blackbox_pursuit::speech::Line;
        match line {
            Line::DispTimeExpired => self.search_pattern,
            Line::WeatherReport => self.weather_report,
            Line::CallForRoadblock => self.roadblock_call,
            Line::CollisionWorld | Line::CollWorldCivi | Line::CollWorldSpin => self.been_hit,
            Line::HeliLostVisual => self.reacquire,
            Line::Spotted => self.spotted_again,
            Line::UnitDisabled => self.kill_shot,
            Line::IntentToRam => self.ram_lines,
            _ => true,
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
