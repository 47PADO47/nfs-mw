//! What the player can do to the radio: pause and resume, go to the next song, go back to the previous one.
//!
//! The original has no radio dial and no "previous song" (spec `docs/specs/music-graph.md` section 7); the history
//! and the pause are the rewrite's own. The history holds the songs played in this session, so `previous` goes back
//! through them and `next` goes forward through them again before the play list picks anything new.

use super::{Player, Radio};

/// A request from the keyboard, the pad or the console.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Control {
    /// Pause the song on the air, resume a paused one, or start one when none plays.
    Toggle,
    /// The next song.
    Next,
    /// The previous song (or the start of this one).
    Previous,
}

/// Songs the history keeps.
const HISTORY_LIMIT: usize = 32;
/// `previous` restarts the song on the air when it has played longer than this (in seconds); within it, it goes back.
const RESTART_AFTER: f32 = 3.0;

impl Radio {
    /// Do what `control` asks, with `music` to start songs on. Returns what to tell the player.
    pub fn apply(&mut self, control: Control, music: &mut impl Player) -> Result<String, String> {
        match control {
            Control::Toggle => self.toggle(music),
            Control::Next => self.skip(music).map(|title| format!("next: {title}")),
            Control::Previous => self.previous(music).map(|title| format!("previous: {title}")),
        }
    }

    /// Note that song `n` began by choice or by the play list: whatever the player had gone back over is forgotten.
    pub(super) fn record(&mut self, n: usize) {
        self.history.truncate(self.cursor);
        self.history.push(n);
        if self.history.len() > HISTORY_LIMIT {
            self.history.remove(0);
        }
        self.cursor = self.history.len();
    }

    /// Hold the song on the air where it is.
    pub fn pause(&mut self) -> Result<String, String> {
        if !self.enabled {
            return Err("the radio is off (radio on)".into());
        }
        let Some(playing) = &self.playing else { return Err("no song is on the air".into()) };
        if self.paused {
            return Ok("is already paused".into());
        }
        playing.handle.set_paused(true);
        self.paused = true;
        self.announce();
        Ok("paused".into())
    }

    /// Let a paused song go on.
    pub fn resume(&mut self) -> Result<String, String> {
        let Some(playing) = &self.playing else { return Err("no song is on the air".into()) };
        if !self.paused {
            return Ok("is not paused".into());
        }
        playing.handle.set_paused(false);
        self.paused = false;
        self.announce();
        Ok("resumed".into())
    }

    /// Pause, resume, or (with no song on the air) start one now.
    pub fn toggle(&mut self, music: &mut impl Player) -> Result<String, String> {
        if !self.enabled {
            return Err("the radio is off (radio on)".into());
        }
        if self.paused {
            return self.resume();
        }
        if self.playing.is_some() {
            return self.pause();
        }
        self.skip(music).map(|title| format!("playing {title}"))
    }

    /// Go back to the song before the one on the air; when this one has played for a while (or nothing is before
    /// it), start it again. Returns the title.
    pub fn previous(&mut self, music: &mut impl Player) -> Result<String, String> {
        if !self.enabled {
            return Err("the radio is off (radio on)".into());
        }
        if self.cursor == 0 {
            return Err("no song has played yet".into());
        }
        let played = self.now.as_ref().map_or(f32::MAX, |now| now.elapsed_secs);
        let cursor = if self.cursor >= 2 && played <= RESTART_AFTER { self.cursor - 1 } else { self.cursor };
        let n = self.history[cursor - 1];
        self.broken = None;
        self.manual = !self.driving;
        self.stop();
        self.start(n, music)?;
        self.cursor = cursor;
        Ok(self.songs[n].title.clone())
    }

    /// Show the pause state of the song on the air to the HUD (a new `serial` makes it announce itself again).
    fn announce(&mut self) {
        self.serial = self.serial.wrapping_add(1);
        let (paused, serial) = (self.paused, self.serial);
        let Some(now) = &mut self.now else { return };
        now.paused = paused;
        now.serial = serial;
    }
}
