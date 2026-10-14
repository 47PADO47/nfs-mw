//! The chase in the sound: each frame the world's chase reaches the music and the speech. The music follows it through
//! [`MusicInput`]; the speech gets its context, a new chase opens with the speech event that starts a pursuit, and the
//! edges of the chase (a sighting lost or regained, the heat jumping, the search's time running out) are said as their
//! speech events. The air-support switches of the config (`chase/air_support.rs`) gate the groups that have a switch.
//!
//! Not wired, for lack of state in [`PursuitFeed`] (the feed is built by `scenes/world/ai/police/world.rs`, which is
//! not this module's). Each line below needs the state named; add it to the feed, then add a cue in `chase/cues.rs`.
//! - Helicopter lines (`SpottedAgain` `spotted` 214, `Reacquire` `helilostvisual` 169, `SearchPattern` heli call,
//!   `HoldOn`, `KillShot`, `DeadHeliSilence`, `AttackMissed`): the helicopter's presence, its line of sight and its
//!   health or fuel. The feed has only the cops' sight.
//! - `RamLines` `intenttoram` 113 and `AttackMissed`: a ram intent and the outcome of a ram attempt (no ram state).
//! - `VehicleReport` `vehiclereport` 155 and `EscapedCar` `spotterwanted` 216: the first sighting's car description and
//!   a memory of the cars that got away (neither is in the feed).
//! - `BeenHit` `collisionworld` 98 family: the player's collisions (no hit state).
//! - `RoadblockCall` and the strategy lines: the roadblock state (none in the feed).
//! - `WeatherReport` `weatherreport` 219 and `RainFlag`: the rain intensity, which the weather model holds in the
//!   scene, not the feed (`weather::Weather::rain`).
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
mod tests;

use cues::Cue;
pub(crate) use cues::Cues;

pub(crate) use air_support::AirSupport;

use super::{Audio, MusicInput, PursuitFeed};

/// The speech event a chase opens with: number 61 of the speech tables (`initpursuit` in
/// `docs/specs/ai-pursuit-speech-events.md`), whose collection is named `setup_initpursuit` in the data.
const OPENING_LINE: &str = "setup_initpursuit";

/// The speech event of each cue, by its collection name (numbers in `docs/specs/ai-pursuit-speech-events.md`):
/// `lostvisual` is 106, `regainvisual` 107, `heatjump` 220, `disptimeexpired` 180.
fn cue_line(cue: Cue) -> &'static str {
    match cue {
        Cue::Lost => "lostvisual",
        Cue::Regained => "regainvisual",
        Cue::HeatJump => "heatjump",
        Cue::TimeExpired => "disptimeexpired",
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
            self.say_line(OPENING_LINE);
        }
        // The cues are tracked whether or not they are said: a switched-off group still keeps its edges.
        let allowed: Vec<Cue> = cues.into_iter().filter(|cue| self.air_support.allows(*cue)).collect();
        for cue in allowed {
            self.say_line(cue_line(cue));
        }
    }

    /// Ask for a named pursuit line (speaker 0, any voice) and log what became of it.
    fn say_line(&mut self, name: &str) {
        match self.say_named(name, 0) {
            Ok(outcome) => log::info!("pursuit: {name} {outcome:?}"),
            Err(e) => log::warn!("pursuit: no {name}: {e}"),
        }
    }
}
