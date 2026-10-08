//! The two things the radio talks to, as traits so the player logic can be tested without a sound device or the
//! 533 MB music file.

use std::path::PathBuf;
use std::sync::Arc;

use ea_audio::mus::graph::Graph;
use ea_audio::mus::{Chain, Mpf};
use kira::track::TrackHandle;

use super::feeder::{self, Started};
use super::stream::{StreamData, StreamHandle};

/// Where songs come from: the music map and the stream file.
pub trait Backend: Send {
    /// The chain of streams of the song whose start event is `event`; an error when the map has no such song.
    fn chain(&self, event: u32) -> Result<Arc<Chain>, String>;

    /// Start decoding a chain.
    fn open(&self, chain: &Arc<Chain>) -> Result<Started, String>;
}

/// Where the sound goes: the music track of the mixer.
pub trait Player {
    fn play(&mut self, data: StreamData) -> Result<StreamHandle, String>;
}

impl Player for TrackHandle {
    fn play(&mut self, data: StreamData) -> Result<StreamHandle, String> {
        TrackHandle::play(self, data).map_err(|e| e.to_string())
    }
}

/// The install's `MW_Music.mpf` and `MW_Music.mus`.
pub struct Files {
    graph: Graph,
    mpf: Arc<Mpf>,
    mus: PathBuf,
}

/// The control value a song is walked with. A song's nodes cover the whole range, so any value does.
const CONTROL: u8 = 64;

impl Files {
    pub fn new(mpf_bytes: &[u8], mus: PathBuf) -> Result<Self, String> {
        let mpf = Mpf::parse(mpf_bytes).map_err(|e| format!("MW_Music.mpf: {e}"))?;
        let graph = Graph::parse(mpf_bytes).map_err(|e| format!("MW_Music.mpf: {e}"))?;
        Ok(Self { graph, mpf: Arc::new(mpf), mus })
    }
}

impl Backend for Files {
    fn chain(&self, event: u32) -> Result<Arc<Chain>, String> {
        let chain = self.mpf.song_chain(&self.graph, event, CONTROL).map_err(|e| e.to_string())?;
        Ok(Arc::new(chain))
    }

    fn open(&self, chain: &Arc<Chain>) -> Result<Started, String> {
        feeder::start(&self.mus, self.mpf.clone(), chain.clone())
    }
}
