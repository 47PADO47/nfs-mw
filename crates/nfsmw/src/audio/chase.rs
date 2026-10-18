//! The chase in the sound: each frame the world's chase reaches the music and the speech. The music follows it through
//! [`MusicInput`]; the speech gets its context, a new chase opens with the speech event that starts a pursuit, and the
//! edges of the chase (a sighting lost or regained, the heat jumping, the search's time running out) are said as their
//! speech events. The air-support switches of the config (`chase/air_support.rs`) gate the groups that have a switch.
//!
//! Wired from the feed's air-support state (each an edge in `chase/cues.rs`, gated by its air-support switch):
//! `WeatherReport` `weatherreport` 219 (rain rising), `RoadblockCall` `callforrb` 76, `BeenHit` `collisionworld` 98,
//! `Reacquire` `helilostvisual` 169 and `SpottedAgain` `spotted` 214 (both with a helicopter in the chase), and
//! `KillShot` `unitdisabled` 164 (a cop wrecked).
//!
//! Not wired, for lack of state in [`PursuitFeed`] (the feed is built by `scenes/world/ai/police/world.rs`, which is
//! not this module's). Each line below needs the state named; add it to the feed, then add a cue in `chase/cues.rs`.
//! - Helicopter lines not yet wired (`SearchPattern` heli call, `HoldOn`, `DeadHeliSilence`): the helicopter's line of
//!   sight, its health or fuel. The feed has only the helicopter's presence.
//! - `RamLines` `intenttoram` 113 and `AttackMissed`: a ram intent and the outcome of a ram attempt (no ram state).
//! - `VehicleReport` `vehiclereport` 155 and `EscapedCar` `spotterwanted` 216: the first sighting's car description and
//!   a memory of the cars that got away (neither is in the feed).
//! - The strategy lines: the strategy state (none in the feed).
//! - `RainFlag` (the radio's rain line): no speech collection of that name is in `/speed.exe`, so no event to ask for.
//! - `Answer911` `disp911report` 102 and `unit911reply` 103: the 911 counters (the feed has none).
//! - `FastAirRequest` and `FastAirReminder`: the air-support request state (none in the feed).
//! - `NoSpeechDropoff`: the speech dispatcher's own drop rule (`speech/dispatcher.rs`), not a chase cue.
//! - `LostSuspect` `lostsuspect` 108 (said when the pursuit terminates with no cop in view): the termination reason
//!   is not in the feed, which simply stops.

mod air_support;
#[cfg(test)]
mod air_support_tests;
mod cues;
#[cfg(test)]
mod installed_tests;
#[cfg(test)]
mod line_tests;
#[cfg(test)]
mod tests;

use cues::Cue;
pub(crate) use cues::Cues;

pub(crate) use air_support::AirSupport;

use super::{Audio, MusicInput, PursuitFeed};

/// The speech event a chase opens with: number 61 of the speech tables (`initpursuit` in
/// `docs/specs/ai-pursuit-speech-events.md`), whose collection is named `setup_initpursuit` in the data.
const OPENING_LINE: &str = "setup_initpursuit";
const OPENING_ID: u32 = 61;

/// The speech event of each cue, by its collection name (numbers in `docs/specs/ai-pursuit-speech-events.md`):
/// `lostvisual` is 106, `regainvisual` 107, `heatjump` 220, `disptimeexpired` 180, `weatherreport` 219, `callforrb` 76,
/// `collisionworld` 98, `helilostvisual` 169, `spotted` 214, `unitdisabled` 164.
fn cue_line(cue: Cue) -> &'static str {
    match cue {
        Cue::Lost => "lostvisual",
        Cue::Regained => "regainvisual",
        Cue::HeatJump => "heatjump",
        Cue::TimeExpired => "disptimeexpired",
        Cue::Rain => "weatherreport",
        Cue::RoadblockCalled => "callforrb",
        Cue::BeenHit => "collisionworld",
        Cue::HeliLost => "helilostvisual",
        Cue::HeliSpotted => "spotted",
        Cue::CopDown => "unitdisabled",
    }
}

/// Stable installed speech IDs; the labels above abbreviate some actual collection names.
fn cue_id(cue: Cue) -> u32 {
    match cue {
        Cue::Lost => 106,
        Cue::Regained => 107,
        Cue::HeatJump => 220,
        Cue::TimeExpired => 180,
        Cue::Rain => 219,
        Cue::RoadblockCalled => 76,
        Cue::BeenHit => 98,
        Cue::HeliLost => 169,
        Cue::HeliSpotted => 214,
        Cue::CopDown => 164,
    }
}

/// The air-support switches from the config file next to the settings. Read once, when the audio is made.
pub(crate) fn load_air_support() -> AirSupport {
    match crate::settings::Settings::config_path().as_deref().and_then(std::path::Path::parent) {
        Some(dir) => air_support::read(&dir.join(air_support::FILE_NAME)),
        None => AirSupport::default(),
    }
}

impl Audio {
    /// The air-support switches the chase obeys from now on.
    pub fn set_air_support(&mut self, air_support: AirSupport) {
        self.air_support = air_support;
    }

    /// Once per frame: `chase` is the world's chase, `None` while no cops chase. The music follows it, and the
    /// speech gets its context; a chase that has just begun says its opening line, and its cues are said.
    pub fn follow_chase(&mut self, music: &mut MusicInput, chase: Option<&PursuitFeed>) {
        let started = music.follow(chase);
        let cues = self.chase_cues.step(chase);
        let Some(chase) = chase else {
            self.set_speech_context(0, 0.0, 0.0);
            return;
        };
        self.set_speech_context(chase.heat as i32, chase.player_mph, chase.secs);
        if started {
            self.say_line(OPENING_LINE, OPENING_ID);
        }
        // The cues are tracked whether or not they are said: a switched-off group still keeps its edges.
        let allowed: Vec<Cue> = cues.into_iter().filter(|cue| self.air_support.allows(*cue)).collect();
        for cue in allowed {
            self.say_line(cue_line(cue), cue_id(cue));
        }
    }

    /// Ask for a named pursuit line (speaker 0, any voice) and log what became of it.
    fn say_line(&mut self, name: &str, id: u32) {
        match self.say(id, 0) {
            Ok(outcome) => log::info!("pursuit: {name} {outcome:?}"),
            Err(e) => log::warn!("pursuit: no {name}: {e}"),
        }
    }
}
