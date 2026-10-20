//! The chase in the sound: each frame the world's chase reaches the music and the speech. The music follows it through
//! [`MusicInput`]; the speech gets its context, and the chase radio (`chase/radio`) runs the original's `SoundAI`
//! speech machine against it: the opening exchange, the strategy commentary and backup requests, the lost-contact
//! lines, the observer's remarks, heat jumps and the arrest bullhorn, each worded by the parameters the machine fills.
//! The air-support switches of the config (`chase/air_support.rs`) gate the lines that have a switch.
//!
//! The machine itself is `blackbox_pursuit::speech` (pure, tested without a game); this module is its adapter: the feed
//! (`chase/feed.rs`, filled by `scenes/world/ai/police/speech_feed.rs`), the snapshot, the game's tables (voices,
//! battalions, road names) and the mapping of each line to its speech event and parameters. Spec:
//! `docs/specs/ai-pursuit-speech.md`; the rows that are wired are in `docs/pursuit-system.md`, "Police radio".
//!
//! Not wired, for lack of state in the world (each needs the state named; add it to [`feed::SpeechFeed`], then to the
//! snapshot in `radio/snapshot.rs`):
//! - The free-roam 911 call (`disp911report`): the machine and the adapter work (a test drives them), but the world
//!   gives no feed outside a chase, so no free-roam frame reaches them; `Audio::take_911_call` hands the call's
//!   `Lifetime911` to the world once something calls it.
//! - Notices the world does not raise: a deliberate hit on a cop with its contact (the adapter infers one from the
//!   assault infraction), a crash against the world, a jump or a roll (`Notice::PlayerHitWorld`, `Airborne`, the up
//!   vector), the road alignment and brake of the car (the u-turn and hard-braking remarks), a passing cop
//!   (`BlewByCop`), a tunnel, a gas station blast, a cop hitting traffic or a wall.
//! - The roadblock's outcomes (`RoadblockDodged`, `RoadblockHit`, `RoadblockPosition`) and the helicopter joining one;
//!   the roadblock and helicopter agent's work.
//! - The backup's arrival time (`backup_eta`) and the heavy support requests (`BackupRequested`, `BackupDenied`).
//! - The off-road places of the speech map, AI racers, the weather's effect on bail-outs beyond the rain edge.
//! - `RainFlag` (the radio's rain line): no speech collection of that name is in `/speed.exe`, so no event to ask for.
//! - `NoSpeechDropoff`: the speech dispatcher's own drop rule (`speech/dispatcher.rs`), not a chase cue.
//! - The helicopter's `HoldOn`, `DeadHeliSilence`, `AttackMissed`: no event the machine asks for.

mod air_support;
#[cfg(test)]
mod air_support_tests;
mod cues;
mod data;
pub mod feed;
#[cfg(test)]
mod line_tests;
pub mod radio;

use cues::Cue;
pub(crate) use cues::Cues;

pub(crate) use air_support::AirSupport;
pub(crate) use data::ChaseData;
pub(crate) use radio::ChaseRadio;

use blackbox_pursuit::EndReason;

use super::{Audio, MusicInput, PursuitFeed};

/// The speech event of the weather cue, by the name the attribute database gives it (number 219, `weatherreport`).
fn cue_line(cue: Cue) -> &'static str {
    match cue {
        Cue::Rain => "anytimeevents_weatherreport",
    }
}

/// Stable installed speech ID of the weather cue; the label above abbreviates the actual collection name.
fn cue_id(cue: Cue) -> u32 {
    match cue {
        Cue::Rain => 219,
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

    /// Once per frame: `chase` is the world's chase, `None` while no cops chase, `ended` is how the last chase ended
    /// (`EndReason::None` before any) and `dt` the time since the last frame. The music follows the chase, the speech
    /// gets its context, the chase radio runs and the weather cue is said.
    pub fn follow_chase(&mut self, music: &mut MusicInput, chase: Option<&PursuitFeed>, ended: EndReason, dt: f32) {
        music.follow(chase);
        match chase {
            Some(chase) => self.set_speech_context(chase.heat as i32, chase.player_mph, chase.secs),
            None => self.set_speech_context(0, 0.0, 0.0),
        }
        self.run_chase_radio(chase, ended, dt);
        let Some(cue) = self.chase_cues.step(chase) else { return };
        if self.air_support.allows(cue) {
            self.say_line(cue_line(cue), cue_id(cue));
        }
    }

    /// Runs the speech machine for one frame and sends what it asks for to the speech, loading the speech when there is
    /// something to say.
    fn run_chase_radio(&mut self, chase: Option<&PursuitFeed>, ended: EndReason, dt: f32) {
        let level = self.chase_level(chase.map_or(1, |c| c.heat));
        let out = {
            let dispatcher = self.speech.loaded_mut().map(|speech| speech.dispatcher_mut());
            self.chase_radio.run(dt, chase, ended, &level, dispatcher)
        };
        if out.is_empty() {
            return;
        }
        if !self.speech.audible() {
            return;
        }
        if let Err(e) = self.speech() {
            log::debug!("chase radio: {e}");
            return;
        }
        let air = self.air_support;
        let Audio { speech, chase_radio, .. } = self;
        if let Some(speech) = speech.loaded_mut() {
            chase_radio.apply(out, speech, &air);
        }
    }

    /// The speech fields of the heat level, reading the database (and the speech tuning, once) on first use. Without the
    /// database the machine keeps the install's tuning and the level has no roadblocks and no 911.
    fn chase_level(&mut self, heat: u32) -> nfsmw_data::speech_chase::LevelSpeech {
        let Ok(db) = self.database() else { return Default::default() };
        if let Some(tune) = self.chase_data.take_new_tune(&db) {
            self.chase_radio.set_tune(&tune);
        }
        self.chase_data.level(&db, heat)
    }

    /// The 911 call that ended: the seconds the cops must keep away, once. The world takes it when it models 911 calls.
    #[allow(dead_code)] // the world has no 911 calls yet
    pub fn take_911_call(&mut self) -> Option<f32> {
        self.chase_radio.take_911_call()
    }

    /// The chase radio's state, for the console's `speech flow`.
    pub fn chase_flow_status(&self) -> String {
        self.chase_radio.status()
    }

    /// Says the weather cue's line (speaker 0, any voice) and logs what became of it.
    fn say_line(&mut self, name: &str, id: u32) {
        match self.say(id, 0) {
            Ok(outcome) => log::info!("pursuit: {name} {outcome:?}"),
            Err(e) => log::warn!("pursuit: no {name}: {e}"),
        }
    }
}
