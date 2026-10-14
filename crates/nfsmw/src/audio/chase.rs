//! The chase in the sound: each frame the world's chase reaches the music and the speech. The music follows it through
//! [`MusicInput`]; the speech gets its context, and a new chase opens with the speech event that starts a pursuit.

use super::{Audio, MusicInput, PursuitFeed};

/// The speech event a chase opens with: number 61 of the speech tables (`initpursuit` in
/// `docs/specs/ai-pursuit-speech-events.md`), whose collection is named `setup_initpursuit` in the data.
const OPENING_LINE: &str = "setup_initpursuit";

impl Audio {
    /// Once per frame: `chase` is the world's chase, `None` while no cops chase. The music follows it, and the
    /// speech gets its context; a chase that has just begun says its opening line.
    pub fn follow_chase(&mut self, music: &mut MusicInput, chase: Option<&PursuitFeed>) {
        let started = music.follow(chase);
        let Some(chase) = chase else {
            self.set_speech_context(0, 0.0, 0.0);
            return;
        };
        self.set_speech_context(chase.heat as i32, chase.player_mph, chase.secs);
        if !started {
            return;
        }
        match self.say_named(OPENING_LINE, 0) {
            Ok(outcome) => log::info!("pursuit: the opening line {outcome:?}"),
            Err(e) => log::warn!("pursuit: no opening line: {e}"),
        }
    }
}
