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
mod custom;
pub(in crate::audio) mod feeder;
mod glue;
pub mod input;
mod playlist;
mod preview;
mod state;
pub(in crate::audio) mod stream;
#[cfg(test)]
mod tests;

use std::path::PathBuf;

use nfsmw_data::music::Song;

pub use backend::{Backend, Files, Player};
pub use commands::command;
pub use controls::Control;
pub use custom::Library;
pub use glue::RadioSlot;
pub use playlist::{Mode, Playlist, Random};
pub use state::NowPlaying;
use state::Playing;
use stream::StreamData;

use crate::settings::CustomMusic;
use crate::settings::eatrax::Jukebox;
use custom::{Custom, CustomTrack, Source};

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

/// One track of the radio's history: an EA Trax song (its index in the song list) or a custom track (its number in
/// the library).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Entry {
    Song(usize),
    Custom(usize),
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
    /// A jukebox preview pauses the prior song without consuming its playlist/history position.
    preview: Option<preview::Snapshot>,
    /// The song on the air is held (paused); cleared whenever it stops.
    paused: bool,
    /// The interactive soundtrack owns the music transport this frame.
    held: bool,
    /// The tracks played this session, oldest first, so `previous` can go back and `next` forward again.
    history: Vec<Entry>,
    /// How many entries of `history` are behind and including the track on the air; the next one is `history[cursor]`.
    cursor: usize,
    /// The player's own tracks (`custom/`); empty unless the config turns them on.
    custom: Custom,
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
            preview: None,
            paused: false,
            held: false,
            history: Vec::new(),
            cursor: 0,
            custom: Custom::default(),
            serial: 0,
            broken: None,
            now: None,
        }
    }

    /// Uses the jukebox's table: each list plays the songs of its mode in the table's order, and the order flag picks
    /// random or sequential play. Songs the backend has no chain for never play (`docs/specs/eatrax-jukebox.md`, 11).
    pub fn set_jukebox(&mut self, jukebox: &Jukebox) {
        let known: Vec<bool> = (0..self.songs.len()).map(|n| self.backend.chain(self.songs[n].event).is_ok()).collect();
        let list = |index: usize| {
            let order = jukebox.order(index);
            let enabled: Vec<usize> =
                order.iter().copied().filter(|&n| known.get(n).copied().unwrap_or(false)).collect();
            Playlist::with_order(enabled, order)
        };
        self.lists = [list(0), list(1)];
        self.mode = if jukebox.random { Mode::Shuffle } else { Mode::Ordered };
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

    /// Start the next track: the one after the track on the air in the history when the player went back, else the
    /// one the play lists name. `None` when no track may play in this context.
    ///
    /// A custom track that will not open is skipped for the next one (at most once per custom track); an EA Trax song
    /// that will not start is an error, as before.
    pub fn start_next(&mut self, music: &mut impl Player) -> Result<Option<Entry>, String> {
        if let Some(&entry) = self.history.get(self.cursor) {
            self.start_entry(entry, music)?;
            self.cursor += 1;
            return Ok(Some(entry));
        }
        let mut attempts = 0;
        loop {
            let Some(entry) = self.pick_fresh() else { return Ok(None) };
            match self.start_entry(entry, music) {
                Ok(()) => {
                    self.record(entry);
                    return Ok(Some(entry));
                }
                Err(e) => {
                    attempts += 1;
                    let custom = matches!(entry, Entry::Custom(_));
                    if !custom || attempts > self.custom.track_count() {
                        return Err(e);
                    }
                    log::warn!("custom music: {e}; trying the next track");
                }
            }
        }
    }

    /// The next track the play lists choose (not from the history), and advances the list it came from.
    fn pick_fresh(&mut self) -> Option<Entry> {
        let list = self.context().index();
        let has_trax = !self.lists[list].is_empty();
        match self.custom.choose(list, has_trax)? {
            Source::Trax => {
                let mode = self.mode;
                let random = &mut self.random;
                self.lists[list].next(mode, &mut |count| random.below(count)).map(Entry::Song)
            }
            Source::Custom => {
                let mode = self.mode;
                let random = &mut self.random;
                self.custom.next_track(list, mode, &mut |count| random.below(count)).map(Entry::Custom)
            }
        }
    }

    /// Start a track of either kind.
    fn start_entry(&mut self, entry: Entry, music: &mut impl Player) -> Result<(), String> {
        match entry {
            Entry::Song(n) => self.start(n, music),
            Entry::Custom(n) => self.start_custom(n, music),
        }
    }

    /// Start custom track `n` on `music`, replacing the one on the air. Its fade is the config's `fade_ms`.
    fn start_custom(&mut self, n: usize, music: &mut impl Player) -> Result<(), String> {
        let track: CustomTrack = self.custom.track(n).ok_or_else(|| format!("no custom track {n}"))?.clone();
        let opened = custom::open(&track.path)?;
        let caption = custom::caption(&track, &opened.tags);
        let length_secs = opened.length_secs.or(track.length_secs).unwrap_or(0.0);
        let handle = music.play(StreamData { blocks: opened.blocks, sample_rate: opened.sample_rate })?;
        handle.set_fade_out(self.custom.fade_secs());
        log::info!("radio: {} - {} (custom, {:.1} s)", caption.artist, caption.title, length_secs);
        self.serial = self.serial.wrapping_add(1);
        self.now = Some(NowPlaying::custom(&caption, length_secs, self.serial));
        self.paused = false;
        self.playing = Some(Playing { handle });
        Ok(())
    }

    /// The title of the track on the air, for the console's answers.
    fn on_air_title(&self) -> String {
        self.now.as_ref().map(|now| now.title.clone()).unwrap_or_default()
    }

    /// Give the radio the player's own tracks. `config` says the mode, the starting order and the fade. An empty
    /// library (the feature is off, or nothing was found) leaves the radio as it was.
    pub fn set_custom(&mut self, config: &CustomMusic, library: Library) {
        if library.is_empty() {
            return;
        }
        self.custom = Custom::new(library, config.mode, config.fade_ms as f32 / 1000.0);
        self.mode = if config.shuffle { Mode::Shuffle } else { Mode::Ordered };
    }

    /// Once per frame: tell the radio whether the game is being played (driving, or paused in the game) and
    /// whether music can be heard. It starts the next song when the game begins and when a song ends, and stops
    /// when the game ends (unless the player started it by hand). A song on the air goes on when the music volume
    /// is turned to zero (it is silent, not skipped), so moving the slider does not lose it; only a new song waits
    /// for the music to be audible.
    #[cfg(test)]
    pub fn update(&mut self, driving: bool, audible: bool, music: &mut impl Player) {
        self.update_held(driving, false, false, audible, music);
    }

    /// `update`, with `held`: a pursuit has the songs' place for now (its music plays instead). That is not the game
    /// ending, so the play list and the history stay. The song on the air stops, unless the player paused it: a held
    /// paused song stays where it is and goes on only when the player resumes it. `menus` is true while the main
    /// menu is up: the radio plays there too, from the front-end list.
    pub fn update_held(&mut self, driving: bool, menus: bool, held: bool, audible: bool, music: &mut impl Player) {
        if self.preview.is_some() {
            self.update_preview();
            return;
        }
        self.held = held;
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
        if !(self.enabled && (driving || menus || self.manual)) {
            return self.stop();
        }
        if held {
            if !self.paused {
                self.stop();
            }
            return;
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
        self.transport_available()?;
        if !self.enabled {
            return Err("the radio is off (radio on)".into());
        }
        self.broken = None;
        self.manual = !self.driving;
        self.stop();
        self.start_next(music)?.ok_or_else(|| format!("no songs in the {} list", self.context().name()))?;
        Ok(self.on_air_title())
    }
}
