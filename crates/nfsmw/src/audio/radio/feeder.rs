//! The decoder thread: reads a song's chain of streams from the `.mus` file and sends the frames on.

use std::fs::File;
use std::path::Path;
use std::sync::Arc;
use std::sync::mpsc::{Receiver, SyncSender, sync_channel};
use std::thread;

use ea_audio::mus::{Chain, ChainReader, Mpf};
use ea_audio::{Pcm, ReadAt};

use super::stream::Block;
use crate::audio::pcm;

/// Blocks the thread may run ahead of playback. A block is one `SCDl` block, about 0.2 s of music.
const AHEAD: usize = 12;

/// A file read with positioned reads, so the 533 MB music file is never loaded.
pub struct FileSource {
    file: File,
    len: u64,
}

impl FileSource {
    pub fn open(path: &Path) -> std::io::Result<Self> {
        let file = File::open(path)?;
        let len = file.metadata()?.len();
        Ok(Self { file, len })
    }
}

impl ReadAt for FileSource {
    fn len(&self) -> u64 {
        self.len
    }

    fn read_at(&self, offset: u64, buf: &mut [u8]) -> usize {
        let mut done = 0;
        while done < buf.len() {
            #[cfg(windows)]
            let n = std::os::windows::fs::FileExt::seek_read(&self.file, &mut buf[done..], offset + done as u64);
            #[cfg(unix)]
            let n = std::os::unix::fs::FileExt::read_at(&self.file, &mut buf[done..], offset + done as u64);
            match n {
                Ok(0) | Err(_) => break,
                Ok(n) => done += n,
            }
        }
        done
    }
}

/// What the player needs to know about a chain before it starts.
pub struct Started {
    pub sample_rate: u32,
    pub blocks: Receiver<Block>,
}

/// Start decoding `chain` from the file at `mus` on a new thread. The format of the first stream is read here, so
/// a missing file or a damaged first stream is an error now rather than silence later. The thread ends when the
/// chain is done, on a decoding error (logged), or when the receiver is dropped.
pub fn start(mus: &Path, mpf: Arc<Mpf>, chain: Arc<Chain>) -> Result<Started, String> {
    let source = FileSource::open(mus).map_err(|e| format!("{}: {e}", mus.display()))?;
    let sample_rate = ChainReader::new(&mpf, &source, &chain).map_err(|e| e.to_string())?.sample_rate();
    let (tx, blocks) = sync_channel(AHEAD);
    thread::Builder::new()
        .name("radio-decoder".into())
        .spawn(move || feed(&mpf, &source, &chain, &tx))
        .map_err(|e| format!("cannot start the decoder: {e}"))?;
    Ok(Started { sample_rate, blocks })
}

/// Decode the whole chain into `tx`; returns when it is done or the receiver has gone.
pub fn feed<S: ReadAt + ?Sized>(mpf: &Mpf, source: &S, chain: &Chain, tx: &SyncSender<Block>) {
    let mut reader = match ChainReader::new(mpf, source, chain) {
        Ok(reader) => reader,
        Err(e) => return log::warn!("radio: {e}"),
    };
    let (sample_rate, channels) = (reader.sample_rate(), reader.channels());
    let mut samples = Vec::new();
    loop {
        samples.clear();
        match reader.next_chunk(&mut samples) {
            Ok(Some(_)) => {}
            Ok(None) => return,
            Err(e) => return log::warn!("radio: stopped in stream {}: {e}", reader.segment().stream),
        }
        let chunk = Pcm { sample_rate, channels, samples: std::mem::take(&mut samples), loop_range: None };
        if tx.send(pcm::frames(&chunk)).is_err() {
            return;
        }
        samples = chunk.samples;
    }
}
