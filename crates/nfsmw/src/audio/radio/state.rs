//! What the radio exposes about the song on the air.

use nfsmw_data::music::Song;

use super::stream::StreamHandle;

/// The song on the air, as the HUD shows it (artist and title; the original also shows the album).
#[derive(Debug, Clone, PartialEq)]
pub struct NowPlaying {
    /// Index in the song list.
    pub song: usize,
    pub artist: String,
    pub title: String,
    pub album: String,
    /// Seconds played so far (refreshed every frame).
    pub elapsed_secs: f32,
    /// Length of the song (the sum of the stored stream lengths).
    pub length_secs: f32,
    /// The song is held (the radio is paused).
    pub paused: bool,
    /// Changes whenever the radio has something new to announce: a song starts, it is paused or resumed.
    pub serial: u32,
}

impl NowPlaying {
    pub(super) fn new(index: usize, song: &Song, length_secs: f32, serial: u32) -> Self {
        Self {
            song: index,
            artist: song.artist.clone(),
            title: song.title.clone(),
            album: song.album.clone(),
            elapsed_secs: 0.0,
            length_secs,
            paused: false,
            serial,
        }
    }

    /// `m:ss of m:ss`, for the HUD.
    pub fn progress(&self) -> String {
        format!("{} / {}", clock(self.elapsed_secs), clock(self.length_secs))
    }

    /// `artist - title`.
    pub fn label(&self) -> String {
        format!("{} - {}", self.artist, self.title)
    }
}

/// The stream that is playing; dropping it fades it out.
pub(super) struct Playing {
    pub handle: StreamHandle,
}

/// `m:ss`.
pub fn clock(secs: f32) -> String {
    let whole = secs.max(0.0) as u32;
    format!("{}:{:02}", whole / 60, whole % 60)
}
