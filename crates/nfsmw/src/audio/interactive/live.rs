//! The decoder thread of a pursuit track: follows the music graph one bar at a time with the control value the
//! game writes, reads each bar's stream from the `.mus` file and sends the frames on.
//!
//! Unlike the radio, whose songs are walked to their end before they start, the next bar is chosen when the
//! previous bar has been decoded, with the control value of that moment. The thread runs a few blocks ahead of
//! the player, so a change of the control value is heard after the buffered blocks (about a second) and the rest
//! of the bar on the air.

use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::mpsc::{SyncSender, sync_channel};
use std::thread;

use ea_audio::Pcm;
use ea_audio::mus::Mpf;
use ea_audio::mus::graph::{Advance, Cursor, Graph};

use crate::audio::pcm;
use crate::audio::radio::feeder::{FileSource, Started};
use crate::audio::radio::stream::Block;

/// Blocks the thread may run ahead of playback (about 0.2 s each): few, so the control value acts soon.
pub const AHEAD: usize = 5;

/// Decodes one stream of the map to the player. Returns `Ok(false)` when the player has gone.
pub type Play<'a> = dyn FnMut(u32, &SyncSender<Block>) -> Result<bool, String> + 'a;

/// The first audio node from `cursor`, or why there is none.
fn first_stream(graph: &Graph, cursor: &mut Cursor, value: u8) -> Result<u32, String> {
    match cursor.advance(graph, value) {
        Advance::Audio { stream, .. } => Ok(stream),
        Advance::End(end) => Err(format!("the track starts at nothing to play ({end:?})")),
    }
}

/// Follow the graph from `first`, the stream the cursor has just reached, until the track ends or the player is
/// gone. Each step reads the control value afresh.
pub fn feed(
    graph: &Graph,
    mut cursor: Cursor,
    first: u32,
    control: &AtomicU8,
    tx: &SyncSender<Block>,
    play: &mut Play<'_>,
) {
    let mut stream = first;
    loop {
        match play(stream, tx) {
            Ok(true) => {}
            Ok(false) => return,
            Err(e) => return log::warn!("music: stopped in stream {stream}: {e}"),
        }
        match cursor.advance(graph, control.load(Ordering::Relaxed)) {
            Advance::Audio { stream: next, fired, .. } => {
                if !fired.is_empty() {
                    log::debug!("music: passed events {fired:06X?} (not run)");
                }
                stream = next;
            }
            Advance::End(end) => return log::debug!("music: the pursuit track ended ({end:?})"),
        }
    }
}

/// Reads streams of the `.mus` file, all of one format.
struct FileDecoder {
    mpf: Arc<Mpf>,
    source: FileSource,
    sample_rate: u32,
    channels: u16,
}

impl FileDecoder {
    fn play(&self, stream: u32, tx: &SyncSender<Block>) -> Result<bool, String> {
        let mut reader = self.mpf.open(&self.source, stream as usize).map_err(|e| e.to_string())?;
        let header = reader.header();
        if (header.sample_rate, header.channels) != (self.sample_rate, self.channels) {
            return Err("the streams of a track differ in rate or channels".into());
        }
        let mut samples = Vec::new();
        loop {
            samples.clear();
            match reader.next_chunk(&mut samples).map_err(|e| e.to_string())? {
                Some(_) => {}
                None => return Ok(true),
            }
            let chunk = Pcm {
                sample_rate: self.sample_rate,
                channels: self.channels,
                samples: std::mem::take(&mut samples),
                loop_range: None,
            };
            if tx.send(pcm::frames(&chunk)).is_err() {
                return Ok(false);
            }
            samples = chunk.samples;
        }
    }
}

/// Start the track that enters the graph at `start`, steered by `control`, on a new thread. The first bar is
/// chosen and opened here, so a missing file or a start node that plays nothing is an error now. The thread ends
/// with the track, on a decoding error (logged), or when the receiver is dropped.
pub fn start(
    mus: &Path,
    mpf: Arc<Mpf>,
    graph: Arc<Graph>,
    start: usize,
    control: Arc<AtomicU8>,
) -> Result<Started, String> {
    let source = FileSource::open(mus).map_err(|e| format!("{}: {e}", mus.display()))?;
    let mut cursor = Cursor::new(&graph, start).map_err(|e| e.to_string())?;
    let first = first_stream(&graph, &mut cursor, control.load(Ordering::Relaxed))?;
    let (sample_rate, channels) = {
        let reader = mpf.open(&source, first as usize).map_err(|e| e.to_string())?;
        (reader.header().sample_rate, reader.header().channels)
    };
    let (tx, blocks) = sync_channel(AHEAD);
    let decoder = FileDecoder { mpf, source, sample_rate, channels };
    thread::Builder::new()
        .name("music-pursuit".into())
        .spawn(move || feed(&graph, cursor, first, &control, &tx, &mut |s, tx| decoder.play(s, tx)))
        .map_err(|e| format!("cannot start the decoder: {e}"))?;
    Ok(Started { sample_rate, blocks })
}
