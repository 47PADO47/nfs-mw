//! The chase in the sound: each frame the world's chase reaches the music and the speech. The music follows it through
//! [`MusicInput`]; the speech gets its context, a new chase opens with the speech event that starts a pursuit, and the
//! edges of the chase (a sighting lost or regained, the heat jumping) are said as their speech events.
//!
//! Not wired yet, for lack of state in the feed: the roadblock calls and the strategy lines (no roadblock state
//! reaches the audio), a cop hit or rammed (no hit state), the busted line and the dead-heli or kill lines (their state
//! is built by the sibling epics). TODO: add them as cues in `chase/cues.rs` once their state reaches [`PursuitFeed`].

mod cues;
#[cfg(test)]
mod tests;

use cues::Cue;
pub(crate) use cues::Cues;

use super::{Audio, MusicInput, PursuitFeed};

/// The speech event a chase opens with: number 61 of the speech tables (`initpursuit` in
/// `docs/specs/ai-pursuit-speech-events.md`), whose collection is named `setup_initpursuit` in the data.
const OPENING_LINE: &str = "setup_initpursuit";

/// The speech event of each cue, by its collection name (numbers in `docs/specs/ai-pursuit-speech-events.md`):
/// `lostvisual` is 106, `regainvisual` 107, `heatjump` 220.
fn cue_line(cue: Cue) -> &'static str {
    match cue {
        Cue::Lost => "lostvisual",
        Cue::Regained => "regainvisual",
        Cue::HeatJump => "heatjump",
    }
}

impl Audio {
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
        for cue in cues {
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
