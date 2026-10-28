//! Jukebox previews do not consume the radio's playlist, history or user's on/off setting.

use super::{NowPlaying, Player, Playing, Radio};

pub(super) struct Snapshot {
    playing: Option<Playing>,
    now: Option<NowPlaying>,
    paused: bool,
}

impl Radio {
    pub(super) fn preview(&mut self, song: usize, music: &mut impl Player) -> Result<(), String> {
        if self.held {
            return Err("EA Trax is held for pursuit music".into());
        }
        let selected = self.songs.get(song).ok_or_else(|| format!("no song {song}"))?;
        self.backend.chain(selected.event)?;
        if self.preview.is_none() {
            if let Some(playing) = &self.playing {
                playing.handle.set_paused(true);
            }
            self.preview = Some(Snapshot { playing: self.playing.take(), now: self.now.take(), paused: self.paused });
        }
        self.stop();
        if let Err(error) = self.start(song, music) {
            self.end_preview();
            return Err(error);
        }
        Ok(())
    }

    pub(super) fn end_preview(&mut self) {
        let Some(snapshot) = self.preview.take() else { return };
        self.stop();
        if !self.enabled {
            return;
        }
        self.playing = snapshot.playing;
        self.now = snapshot.now;
        self.paused = snapshot.paused;
        if let Some(playing) = &self.playing {
            playing.handle.set_paused(self.paused);
        }
    }

    /// A preview that ends waits for another explicit Preview press; it never starts a different song.
    pub(super) fn update_preview(&mut self) {
        if self.playing.as_ref().is_some_and(|playing| playing.handle.finished()) {
            self.stop();
            return;
        }
        if let (Some(playing), Some(now)) = (&self.playing, &mut self.now) {
            now.elapsed_secs = playing.handle.position_secs();
        }
    }
}
