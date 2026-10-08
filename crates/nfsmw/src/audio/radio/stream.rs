//! A `kira` sound that plays blocks of frames as they arrive from a decoder thread.
//!
//! The decoder thread sends blocks through a bounded channel; the audio thread takes them with `try_recv`, so
//! it never waits. Playback resamples from the stream's rate to the device's with linear interpolation. When
//! the sender is gone and every block is used, the sound is finished. [`StreamHandle`] reports the position and
//! the end to the game thread, and fades the sound out when it is dropped.

use std::convert::Infallible;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::mpsc::{Receiver, TryRecvError};

use kira::Frame;
use kira::info::Info;
use kira::sound::{Sound, SoundData};

/// A block of stereo frames.
pub type Block = Vec<Frame>;

/// How long the fade-out of a stopped sound lasts, in seconds.
const FADE_OUT: f32 = 0.08;

/// What the audio thread and the game thread share.
#[derive(Default)]
struct Shared {
    /// Asked by the game thread: fade out and finish.
    stop: AtomicBool,
    /// Set by the audio thread once every block has been played.
    ended: AtomicBool,
    /// Frames of the stream played so far.
    played: AtomicU64,
    /// Times the audio thread found no block ready while it needed one.
    underruns: AtomicU32,
}

/// The game's end of a playing stream. Dropping it fades the sound out.
pub struct StreamHandle {
    shared: Arc<Shared>,
    sample_rate: u32,
}

impl StreamHandle {
    /// The stream played to its end (or was stopped and has faded out).
    pub fn finished(&self) -> bool {
        self.shared.ended.load(Ordering::Relaxed)
    }

    /// Seconds of the stream played so far.
    pub fn position_secs(&self) -> f32 {
        self.shared.played.load(Ordering::Relaxed) as f32 / self.sample_rate.max(1) as f32
    }

    /// How often playback ran dry because the decoder was late.
    pub fn underruns(&self) -> u32 {
        self.shared.underruns.load(Ordering::Relaxed)
    }
}

impl Drop for StreamHandle {
    fn drop(&mut self) {
        self.shared.stop.store(true, Ordering::Relaxed);
    }
}

/// The blocks to play and their sample rate.
pub struct StreamData {
    pub blocks: Receiver<Block>,
    pub sample_rate: u32,
}

impl SoundData for StreamData {
    type Error = Infallible;
    type Handle = StreamHandle;

    fn into_sound(self) -> Result<(Box<dyn Sound>, StreamHandle), Infallible> {
        let shared = Arc::new(Shared::default());
        let handle = StreamHandle { shared: shared.clone(), sample_rate: self.sample_rate };
        let sound = StreamSound {
            shared,
            blocks: self.blocks,
            rate: f64::from(self.sample_rate.max(1)),
            block: Vec::new(),
            index: 0,
            previous: Frame::ZERO,
            next: Frame::ZERO,
            position: 1.0,
            gain: 1.0,
            drained: false,
        };
        Ok((Box::new(sound), handle))
    }
}

struct StreamSound {
    shared: Arc<Shared>,
    blocks: Receiver<Block>,
    rate: f64,
    block: Block,
    index: usize,
    previous: Frame,
    next: Frame,
    /// Fractional source position between `previous` and `next`; at 1 or more a new frame is needed.
    position: f64,
    gain: f32,
    /// The sender is gone and every block is used.
    drained: bool,
}

impl StreamSound {
    /// The next frame of the stream; `None` when none is ready yet (the decoder is late) or the stream is over.
    fn pull(&mut self) -> Option<Frame> {
        while self.index >= self.block.len() {
            match self.blocks.try_recv() {
                Ok(block) => {
                    self.block = block;
                    self.index = 0;
                }
                Err(TryRecvError::Empty) => {
                    self.shared.underruns.fetch_add(1, Ordering::Relaxed);
                    return None;
                }
                Err(TryRecvError::Disconnected) => {
                    self.drained = true;
                    return None;
                }
            }
        }
        let frame = self.block[self.index];
        self.index += 1;
        self.shared.played.fetch_add(1, Ordering::Relaxed);
        Some(frame)
    }
}

impl Sound for StreamSound {
    fn process(&mut self, out: &mut [Frame], dt: f64, _info: &Info) {
        let step = self.rate * dt;
        let stopping = self.shared.stop.load(Ordering::Relaxed);
        let fade = if stopping { dt as f32 / FADE_OUT } else { 0.0 };
        for frame in out.iter_mut() {
            *frame = Frame::ZERO;
            if self.shared.ended.load(Ordering::Relaxed) {
                continue;
            }
            self.position += step;
            while self.position >= 1.0 {
                let Some(next) = self.pull() else { break };
                self.position -= 1.0;
                self.previous = self.next;
                self.next = next;
            }
            if self.drained {
                self.shared.ended.store(true, Ordering::Relaxed);
                continue;
            }
            if self.position >= 1.0 {
                // Starved: wait for the decoder instead of playing on without sound.
                self.position = 1.0;
                continue;
            }
            self.gain = (self.gain - fade).max(0.0);
            let t = self.position as f32;
            let left = self.previous.left + (self.next.left - self.previous.left) * t;
            let right = self.previous.right + (self.next.right - self.previous.right) * t;
            *frame = Frame::new(left * self.gain, right * self.gain);
            if self.gain <= 0.0 {
                self.shared.ended.store(true, Ordering::Relaxed);
            }
        }
    }

    fn finished(&self) -> bool {
        self.shared.ended.load(Ordering::Relaxed)
    }
}
