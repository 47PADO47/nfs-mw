//! The radio: licensed songs played gaplessly from `MW_Music.mus` through the music volume group.
//!
//! A song is a chain of streams of the music graph (`ea-audio::mus`); [`feeder`] decodes the chain on a thread
//! and [`stream`] plays the blocks. [`Playlist`] picks the song, by the original's rules. [`Radio`] starts the next
//! song when one ends, when driving begins and on `radio next`. Spec: `docs/specs/music-graph.md`.
//!
//! Nothing here has been listened to; see the spec for what is inferred.

mod backend;
mod commands;
mod controls;
pub(in crate::audio) mod feeder;
mod glue;
pub mod input;
mod playlist;
mod state;
pub(in crate::audio) mod stream;
#[cfg(test)]
mod tests;

use std::path::PathBuf;

use nfsmw_data::music::Song;

pub use backend::{Backend, Files, Player};
pub use commands::command;
pub use controls::Control;
pub use glue::RadioSlot;
pub use playlist::{Mode, Playlist, Random};
pub use state::NowPlaying;
use state::Playing;
use stream::StreamData;

/// Which play list is in use.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Context {
    /// The menus (the front end does not start the radio yet; `radio next` outside the game uses it).
    FrontEnd,
    /// Driving.
    InGame,
}

impl Context {
    fn index(self) -> usize {
        match self {
            Self::FrontEnd => 0,
            Self::InGame => 1,
        }
    }

    fn name(self) -> &'static str {
        match self {
            Self::FrontEnd => "front-end",
            Self::InGame => "in-game",
        }
    }
}

pub struct Radio {
    songs: Vec<Song>,
    backend: Box<dyn Backend>,
    /// Front-end and in-game play lists.
    lists: [Playlist; 2],
    mode: Mode,
    random: Random,
    enabled: bool,
    driving: bool,
    /// Started by the player outside the game: plays on until `radio off` or driving starts.
    manual: bool,
    playing: Option<Playing>,
    /// The song on the air is held (paused); cleared whenever it stops.
    paused: bool,
    /// The songs played this session, oldest first, so `previous` can go back and `next` forward again.
    history: Vec<usize>,
    /// How many entries of `history` are behind and including the song on the air; the next one is `history[cursor]`.
    cursor: usize,
    /// Counts what the HUD should announce (a song starting, a pause, a resume).
    serial: u32,
    /// Why songs stopped starting (cleared by the next command), so the failure is not retried every frame.
    broken: Option<String>,
    /// The song on the air, for the HUD.
    pub now: Option<NowPlaying>,
}

impl Radio {
    /// A radio over the install: `songs`, the map in `mpf_bytes` and the streams in the file `mus`.
    pub fn from_files(songs: Vec<Song>, mpf_bytes: &[u8], mus: PathBuf) -> Result<Self, String> {
        Ok(Self::new(songs, Box::new(Files::new(mpf_bytes, mus)?)))
    }

    /// A radio over `songs` whose streams come from `backend`. Songs the backend has no chain for never play.
    pub fn new(songs: Vec<Song>, backend: Box<dyn Backend>) -> Self {
        let known = |n: &usize| backend.chain(songs[*n].event).is_ok();
        let list =
            |wanted: fn(&Song) -> bool| Playlist::new((0..songs.len()).filter(known).filter(|&n| wanted(&songs[n])));
        let lists = [list(|s| s.playability.front_end()), list(|s| s.playability.in_game())];
        Self {
            songs,
            backend,
            lists,
            mode: Mode::default(),
            random: Random::from_clock(),
            enabled: true,
            driving: false,
            manual: false,
            playing: None,
            paused: false,
            history: Vec::new(),
            cursor: 0,
            serial: 0,
            broken: None,
            now: None,
        }
    }

    #[cfg(test)]
    pub fn songs(&self) -> &[Song] {
        &self.songs
    }

    #[cfg(test)]
    pub fn enabled(&self) -> bool {
        self.enabled
    }

    fn context(&self) -> Context {
        if self.driving { Context::InGame } else { Context::FrontEnd }
    }

    /// Choose ordered or shuffled play from the next song on.
    pub fn set_mode(&mut self, mode: Mode) {
        self.mode = mode;
    }

    /// Turn the radio on or off; off stops the song at once.
    pub fn set_enabled(&mut self, on: bool) {
        self.enabled = on;
        self.broken = None;
        if !on {
            self.stop();
        }
    }

    /// Stop the song on the air (it fades out over a few milliseconds).
    pub fn stop(&mut self) {
        self.playing = None;
        self.paused = false;
        self.now = None;
    }

    /// Start song `n` on `music`, replacing the one on the air.
    pub fn start(&mut self, n: usize, music: &mut impl Player) -> Result<(), String> {
        let song = self.songs.get(n).ok_or_else(|| format!("no song {n}"))?;
        let chain = self.backend.chain(song.event)?;
        let started = self.backend.open(&chain)?;
        let handle = music.play(StreamData { blocks: started.blocks, sample_rate: started.sample_rate })?;
        log::info!(
            "radio: {} - {} ({} streams, {:.1} s)",
            song.artist,
            song.title,
            chain.segments.len(),
            chain.total_secs()
        );
        self.serial = self.serial.wrapping_add(1);
        self.now = Some(NowPlaying::new(n, song, chain.total_secs() as f32, self.serial));
        self.paused = false;
        self.playing = Some(Playing { handle });
        Ok(())
    }

    /// Start the next song: the one after the song on the air in the history when the player went back, else the
    /// one the play list names. `None` when no song may play in this context.
    pub fn start_next(&mut self, music: &mut impl Player) -> Result<Option<usize>, String> {
        if let Some(&n) = self.history.get(self.cursor) {
            self.start(n, music)?;
            self.cursor += 1;
            return Ok(Some(n));
        }
        let list = self.context().index();
        let mode = self.mode;
        let random = &mut self.random;
        let Some(n) = self.lists[list].next(mode, &mut |count| random.below(count)) else { return Ok(None) };
        self.start(n, music)?;
        self.record(n);
        Ok(Some(n))
    }

    /// Once per frame: tell the radio whether the game is being played (driving, or paused in the game) and
    /// whether music can be heard. It starts the next song when the game begins and when a song ends, and stops
    /// when the game ends (unless the player started it by hand). A song on the air goes on when the music volume
    /// is turned to zero (it is silent, not skipped), so moving the slider does not lose it; only a new song waits
    /// for the music to be audible.
    pub fn update(&mut self, driving: bool, audible: bool, music: &mut impl Player) {
        if driving != self.driving {
            self.driving = driving;
            self.manual = false;
            self.history.clear();
            self.cursor = 0;
            self.stop();
        }
        if self.playing.as_ref().is_some_and(|p| p.handle.finished()) {
            self.stop();
        }
        if !(self.enabled && (driving || self.manual)) {
            return self.stop();
        }
        if self.playing.is_none() && audible && self.broken.is_none() {
            let started = self.start_next(music);
            if let Err(e) = started {
                log::warn!("radio: {e}");
                self.broken = Some(e);
            }
        }
        let (Some(playing), Some(now)) = (&self.playing, &mut self.now) else { return };
        now.elapsed_secs = playing.handle.position_secs();
    }

    /// `radio next`: stop the song and start the next one now, in the game or not. Returns the title.
    pub fn skip(&mut self, music: &mut impl Player) -> Result<String, String> {
        if !self.enabled {
            return Err("the radio is off (radio on)".into());
        }
        self.broken = None;
        self.manual = !self.driving;
        self.stop();
        let n = self.start_next(music)?.ok_or_else(|| format!("no songs in the {} list", self.context().name()))?;
        Ok(self.songs[n].title.clone())
    }
}
