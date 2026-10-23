//! The sound of a movie, decoded off the frame thread. The picture waits for it (see `MoviePlayback`), so the two start
//! together.

use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, TryRecvError, sync_channel};
use std::thread;

use anyhow::{Context, Result, anyhow};
use blackbox_movie::{Demuxer, Packet};
use ea_audio::Pcm;
use ea_audio::schl::{StreamDecoder, header};

/// The read buffer of the movie file (see `player::READ_BUFFER`).
const READ_BUFFER: usize = 1 << 20;

/// The sound, or `None` for a movie without any, or the reason it cannot be decoded.
pub(super) type Decoded = Result<Option<Pcm>, String>;

/// The frame thread's end of the sound decoder. Dropping it makes the decoder give up.
pub(super) struct SoundDecoder {
    result: Receiver<Decoded>,
    cancel: Arc<AtomicBool>,
}

impl SoundDecoder {
    /// Starts decoding the sound of the movie file at `path` on a worker.
    pub fn spawn(path: PathBuf) -> Result<Self> {
        let (tx, result) = sync_channel(1);
        let cancel = Arc::new(AtomicBool::new(false));
        let stop = cancel.clone();
        thread::Builder::new()
            .name("movie-sound".into())
            .spawn(move || {
                let decoded = decode(&path, &stop).map_err(|e| format!("{e:#}"));
                // A receiver that is gone has no use for the sound.
                let _ = tx.send(decoded);
            })
            .context("starting the sound decoder")?;
        Ok(Self { result, cancel })
    }

    /// The decoded sound, once the worker is done with it.
    pub fn try_take(&self) -> Result<Decoded, TryRecvError> {
        self.result.try_recv()
    }
}

impl Drop for SoundDecoder {
    fn drop(&mut self) {
        self.cancel.store(true, Ordering::Relaxed);
    }
}

/// The sound of the movie, decoded: the interleaved samples of every audio block. The file is read in blocks, and
/// only the audio is kept.
fn decode(path: &Path, cancel: &AtomicBool) -> Result<Option<Pcm>> {
    let _trace = blackbox_trace::scope("Movie: decode the sound");
    let file = File::open(path).with_context(|| format!("opening {}", path.display()))?;
    let mut demuxer = Demuxer::new(BufReader::with_capacity(READ_BUFFER, file)).context("reading the movie")?;
    let Some(raw) = demuxer.audio_header().map(<[u8]>::to_vec) else { return Ok(None) };
    let stream = header::parse(&raw).map_err(|e| anyhow!("the movie's audio header: {e}"))?;
    let mut decoder = StreamDecoder::new(&stream).map_err(|e| anyhow!("the movie's audio: {e}"))?;
    let mut samples = Vec::new();
    while let Some(packet) = demuxer.next_packet().map_err(|e| anyhow!("reading the movie: {e}"))? {
        if cancel.load(Ordering::Relaxed) {
            return Ok(None);
        }
        let Packet::Audio(block) = packet else { continue };
        decoder.decode_block(&block.data, &mut samples).map_err(|e| anyhow!("decoding the movie's audio: {e}"))?;
    }
    Ok(Some(Pcm { sample_rate: stream.sample_rate, channels: stream.channels, samples, loop_range: None }))
}
