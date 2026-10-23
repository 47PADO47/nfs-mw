//! The picture of a movie, made off the frame thread: a worker reads the file, decodes the VP6 frames and turns them into
//! RGBA, and sends them ahead of the clock through a small bounded queue. The frame thread only takes what is due.

use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, Sender, SyncSender, TryRecvError, channel, sync_channel};
use std::thread;

use anyhow::{Context, Result, anyhow};
use blackbox_movie::{Demuxer, MovieHeader, Packet, Timeline, Vp6Decoder, YuvFrame};

/// Messages the worker may run ahead of the frame thread: the header, then the pictures that wait for their time.
const AHEAD: usize = 3;
/// The read buffer of the movie file. A story movie is over 100 MB, so it is read in blocks and never loaded whole.
const READ_BUFFER: usize = 1 << 20;

/// One decoded picture: frame `index` of the movie, RGBA8.
pub(super) struct Picture {
    pub index: u32,
    pub rgba: Vec<u8>,
}

/// What the worker sends.
pub(super) enum Message {
    /// The file is open and its header is read: always the first message.
    Opened(MovieHeader),
    /// The next picture, in order. Pictures before the start offset are not sent.
    Frame(Picture),
    /// The movie cannot be played (a damaged file, a decoding error). The worker stops after it.
    Failed(String),
}

/// The frame thread's end of the video worker. Dropping it stops the worker.
pub(super) struct VideoStream {
    messages: Receiver<Message>,
    /// Pictures the frame thread did not show, handed back for the worker to fill again.
    recycle: Sender<Vec<u8>>,
}

impl VideoStream {
    /// Starts the worker on the movie file at `path`, which decodes from the frame due at `start` seconds. Opening the
    /// file and reading its header happen on the worker; the first [`Message`] reports them.
    pub fn spawn(path: PathBuf, start: f64) -> Result<Self> {
        let (tx, messages) = sync_channel(AHEAD);
        let (recycle, spare) = channel();
        thread::Builder::new()
            .name("movie-video".into())
            .spawn(move || run(&path, start, &tx, &spare))
            .context("starting the video decoder")?;
        Ok(Self { messages, recycle })
    }

    /// The next message, if the worker has one ready. `Disconnected` means the worker is done.
    pub fn try_next(&self) -> Result<Message, TryRecvError> {
        self.messages.try_recv()
    }

    /// Gives a picture that was not shown back to the worker, so the next one needs no new allocation.
    pub fn give_back(&self, rgba: Vec<u8>) {
        // The worker may be gone already: then the buffer is simply freed.
        let _ = self.recycle.send(rgba);
    }
}

/// The worker: decodes the movie into `tx` and ends with it, with a [`Message::Failed`] first on an error.
fn run(path: &Path, start: f64, tx: &SyncSender<Message>, spare: &Receiver<Vec<u8>>) {
    let Err(e) = decode(path, start, tx, spare) else { return };
    // A receiver that is gone has no use for the reason.
    let _ = tx.send(Message::Failed(format!("{e:#}")));
}

/// Decodes every frame from the one due at `start`, in order, and sends it. A full queue makes this wait, which keeps
/// the worker a few frames ahead of the clock and no more. Returns quietly when the receiver is dropped.
fn decode(path: &Path, start: f64, tx: &SyncSender<Message>, spare: &Receiver<Vec<u8>>) -> Result<()> {
    let file = File::open(path).with_context(|| format!("opening {}", path.display()))?;
    let mut demuxer = Demuxer::new(BufReader::with_capacity(READ_BUFFER, file)).context("reading the movie")?;
    let header = *demuxer.header();
    let mut decoder = Vp6Decoder::new(header.width, header.height).map_err(|e| anyhow!("the video decoder: {e}"))?;
    if tx.send(Message::Opened(header)).is_err() {
        return Ok(());
    }
    // Frames before this one are decoded, because later frames predict from them, and not sent.
    let first = Timeline::from_header(&header).frame_at(start).unwrap_or(0);
    let mut yuv = YuvFrame::default();
    while let Some(packet) = demuxer.next_packet().map_err(|e| anyhow!("reading a frame: {e}"))? {
        let Packet::Video(video) = packet else { continue };
        decoder.decode_into(&video.data, &mut yuv).map_err(|e| anyhow!("decoding a frame: {e}"))?;
        if video.index < first {
            continue;
        }
        let mut rgba = spare.try_recv().unwrap_or_default();
        rgba.resize(yuv.rgba_len(), 0);
        yuv.to_rgba(&mut rgba);
        if tx.send(Message::Frame(Picture { index: video.index, rgba })).is_err() {
            return Ok(());
        }
    }
    Ok(())
}
