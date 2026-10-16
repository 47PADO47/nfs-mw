//! The player's own tracks on the radio (`docs/specs/custom-music.md`).
//!
//! [`Library`] holds the tracks of the folders and playlists the config names; [`Queue`] orders one list's tracks;
//! [`choose_source`] decides whether the next track is an EA Trax song or a custom one; [`open`] plays a file as blocks
//! of frames; [`caption`] gives the title and artist the HUD shows. [`Custom`] keeps these for the radio.

mod decode;
mod label;
mod library;
mod queue;
mod rotation;
#[cfg(test)]
pub(crate) mod test_files;

use std::path::PathBuf;

pub use decode::open;
pub use label::{Caption, caption};
pub use library::Library;
pub use queue::Queue;
pub use rotation::{Source, choose as choose_source};

use crate::audio::radio::Mode;
use crate::settings::CustomMode;

/// One audio file of the library.
#[derive(Debug, Clone, PartialEq)]
pub struct CustomTrack {
    pub path: PathBuf,
    /// The title a playlist gave the file (`#EXTINF`), when one did.
    pub title: Option<String>,
    /// The artist a playlist gave the file, when one did.
    pub artist: Option<String>,
    /// The length a playlist gave, in seconds.
    pub length_secs: Option<f32>,
}

/// The custom side of the radio: the library, one queue per list, the source last chosen, and how the two are mixed.
#[derive(Debug, Default)]
pub struct Custom {
    library: Library,
    /// One queue per list, in the radio's list order (front end, then in game).
    queues: [Queue; 2],
    /// The source of the last track picked fresh (not from the history); the mix alternates from it.
    last: Option<Source>,
    mode: CustomMode,
    /// The fade-out of a custom track stopped before its end, in seconds.
    fade_secs: f32,
}

impl Custom {
    pub fn new(library: Library, mode: CustomMode, fade_secs: f32) -> Self {
        let queues = [Queue::new(library.list(0).len()), Queue::new(library.list(1).len())];
        Self { library, queues, last: None, mode, fade_secs }
    }

    /// Whether the library has no tracks (the feature is off, or nothing was found).
    pub fn is_empty(&self) -> bool {
        self.library.is_empty()
    }

    /// The tracks of list `list` (0 front end, 1 in game): track numbers in list order.
    pub fn list(&self, list: usize) -> &[usize] {
        self.library.list(list)
    }

    pub fn track(&self, n: usize) -> Option<&CustomTrack> {
        self.library.tracks.get(n)
    }

    pub fn track_count(&self) -> usize {
        self.library.tracks.len()
    }

    pub fn mode(&self) -> CustomMode {
        self.mode
    }

    pub fn fade_secs(&self) -> f32 {
        self.fade_secs
    }

    /// The source of the next track of list `list`, when a fresh pick is made. `has_trax` says whether the EA Trax
    /// list of the same context has songs.
    pub fn choose(&mut self, list: usize, has_trax: bool) -> Option<Source> {
        let has_custom = !self.library.list(list).is_empty();
        let source = choose_source(self.mode, has_trax, has_custom, self.last)?;
        self.last = Some(source);
        Some(source)
    }

    /// The next custom track of list `list`, as a track number, in the radio's order.
    pub fn next_track(&mut self, list: usize, mode: Mode, random: &mut dyn FnMut(usize) -> usize) -> Option<usize> {
        let position = self.queues[list].next(mode, random)?;
        self.library.list(list).get(position).copied()
    }
}
