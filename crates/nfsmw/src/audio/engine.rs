//! The engine voice: a `kira` sound that runs one or two Ginsu synthesisers (the accelerate and the
//! decelerate loop) and resamples their output to the device. The game thread sets the mix through an
//! [`EngineHandle`]; the audio thread reads it once per callback.

use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use super::filter::{CutoffHandle, VoiceFilter};
use blackbox_ginsu::{GinsuData, GinsuSynth, SynthParams};
use kira::Frame;
use kira::info::Info;
use kira::sound::{Sound, SoundData};

/// Samples a synthesiser renders at once.
const CHUNK: usize = 256;

/// What one loop should sound like right now.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LoopMix {
    /// Target frequency in the loop's units (engine "RPM"), not below the loop's own lowest frequency when the
    /// controller knows it; a request outside the recording's range is clamped by the synthesiser.
    pub frequency: f32,
    /// Linear volume.
    pub volume: f32,
    /// Playback rate multiplier (1 = natural).
    pub pitch: f32,
}

impl Default for LoopMix {
    fn default() -> Self {
        Self { frequency: 0.0, volume: 0.0, pitch: 1.0 }
    }
}

/// What the engine should sound like right now, from the sound mix controller: one drive per loop. The
/// controller decides how they relate (the original hands both loops the same frequency and pitch).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EngineMix {
    pub accel: LoopMix,
    pub decel: LoopMix,
    pub low_pass_hz: f32,
    pub azimuth: u16,
    pub dry: f32,
}

impl Default for EngineMix {
    fn default() -> Self {
        Self::shared(0.0, 1.0, 0.0, 0.0)
    }
}

impl EngineMix {
    /// Both loops at one frequency and pitch, with their own volumes.
    pub fn shared(frequency: f32, pitch: f32, accel_volume: f32, decel_volume: f32) -> Self {
        Self {
            accel: LoopMix { frequency, volume: accel_volume, pitch },
            decel: LoopMix { frequency, volume: decel_volume, pitch },
            low_pass_hz: 25_000.0,
            azimuth: 0,
            dry: 1.0,
        }
    }
}

struct Cell(AtomicU32);

impl Cell {
    fn new(v: f32) -> Self {
        Self(AtomicU32::new(v.to_bits()))
    }
    fn get(&self) -> f32 {
        f32::from_bits(self.0.load(Ordering::Relaxed))
    }
    fn set(&self, v: f32) {
        self.0.store(v.to_bits(), Ordering::Relaxed);
    }
}

#[derive(Clone, Copy)]
struct Request {
    mix: EngineMix,
    revision: u64,
}

struct Requests {
    latest: Request,
}

struct Control {
    request: Mutex<Requests>,
    frequency: Cell,
    closed: AtomicBool,
    consumed_revision: AtomicU64,
    wet_sender: Mutex<Option<super::wet::Sender>>,
    wet_gain: Cell,
}

/// The game's end of a playing engine voice. Dropping it ends the voice.
pub struct EngineHandle(Arc<Control>);

impl EngineHandle {
    pub(super) fn set_wet_sender(&self, sender: super::wet::Sender) {
        if let Ok(mut slot) = self.0.wet_sender.lock() {
            *slot = Some(sender);
        }
    }
    pub(super) fn set_wet_gain(&self, gain: f32) {
        self.0.wet_gain.set(gain.clamp(0.0, 1.0));
    }

    pub fn set(&self, mix: EngineMix) {
        if let Ok(mut request) = self.0.request.lock() {
            let next = Request { mix, revision: request.latest.revision.wrapping_add(1) };
            request.latest = next;
        }
    }

    pub fn set_tick(&self, mix: EngineMix, revision: u64) {
        if let Ok(mut request) = self.0.request.lock() {
            let next = Request { mix, revision };
            request.latest = next;
        }
    }

    /// Latest completed synthesis frequency, before device-rate/dynamic-pitch resampling.
    pub fn consumed_revision(&self) -> Option<u64> {
        let revision = self.0.consumed_revision.load(Ordering::Relaxed);
        (revision != u64::MAX).then_some(revision)
    }
    pub fn current_frequency(&self) -> f32 {
        self.0.frequency.get()
    }
}

impl Drop for EngineHandle {
    fn drop(&mut self) {
        self.0.closed.store(true, Ordering::Relaxed);
    }
}

/// The loops of one engine, ready to play.
pub struct EngineVoice {
    pub accel: Arc<GinsuData>,
    pub decel: Option<Arc<GinsuData>>,
    pub start: EngineMix,
}

impl SoundData for EngineVoice {
    type Error = blackbox_ginsu::Error;
    type Handle = EngineHandle;

    fn into_sound(self) -> Result<(Box<dyn Sound>, Self::Handle), Self::Error> {
        let accel = Stream::new(self.accel, self.start.accel.frequency)?;
        let decel = self.decel.map(|d| Stream::new(d, self.start.decel.frequency)).transpose()?;
        let request = Request { mix: self.start, revision: 0 };
        let control = Arc::new(Control {
            request: Mutex::new(Requests { latest: request }),
            frequency: Cell::new(accel.synth.current_frequency()),
            closed: AtomicBool::new(false),
            consumed_revision: AtomicU64::new(u64::MAX),
            wet_sender: Mutex::new(None),
            wet_gain: Cell::new(0.0),
        });
        let (filter, filter_handle) = VoiceFilter::new(false);
        Ok((
            Box::new(EngineSound {
                control: control.clone(),
                request,
                accel,
                decel,
                revision: None,
                filter,
                filter_handle,
                control_left: 0.0,
                release_left: 0.12,
                wet_sender: None,
            }),
            EngineHandle(control),
        ))
    }
}

/// One loop's synthesiser and the linear resampler that brings it to the device rate.
struct Stream {
    synth: GinsuSynth,
    rate: f64,
    buffer: [f32; CHUNK],
    index: usize,
    previous: f32,
    next: f32,
    position: f64,
}

impl Stream {
    fn new(data: Arc<GinsuData>, start: f32) -> Result<Self, blackbox_ginsu::Error> {
        let synth = GinsuSynth::new(data.clone(), start.max(data.tables().min_frequency()))?;
        let rate = f64::from(data.tables().sample_rate());
        Ok(Self { synth, rate, buffer: [0.0; CHUNK], index: CHUNK, previous: 0.0, next: 0.0, position: 0.0 })
    }

    /// The next output sample at `dt` seconds per output frame.
    fn sample(&mut self, drive: LoopMix, dt: f64) -> f32 {
        let LoopMix { frequency, volume, pitch } = drive;
        self.position += self.rate * f64::from(pitch) * dt;
        while self.position >= 1.0 {
            self.position -= 1.0;
            self.previous = self.next;
            if self.index == CHUNK {
                self.synth.render(&SynthParams::new(frequency).with_volume(volume), &mut self.buffer);
                self.index = 0;
            }
            self.next = self.buffer[self.index];
            self.index += 1;
        }
        self.previous + (self.next - self.previous) * self.position as f32
    }
}

struct EngineSound {
    control: Arc<Control>,
    accel: Stream,
    decel: Option<Stream>,
    request: Request,
    revision: Option<u64>,
    filter: VoiceFilter,
    filter_handle: CutoffHandle,
    control_left: f64,
    release_left: f64,
    wet_sender: Option<super::wet::Sender>,
}

/// A finite, sane value in `lo..=hi` (NaN counts as `lo`).
fn clean(v: f32, lo: f32, hi: f32) -> f32 {
    if v.is_nan() { lo } else { v.clamp(lo, hi) }
}

impl Sound for EngineSound {
    fn process(&mut self, out: &mut [Frame], dt: f64, info: &Info) {
        if !dt.is_finite() || dt <= 0.0 {
            out.fill(Frame::ZERO);
            return;
        }
        let sane = |mix: LoopMix| LoopMix {
            frequency: clean(mix.frequency, 0.0, 100_000.0),
            volume: clean(mix.volume, 0.0, 4.0),
            pitch: clean(mix.pitch, 0.05, 8.0),
        };
        if self.wet_sender.is_none()
            && let Ok(sender) = self.control.wet_sender.try_lock()
        {
            self.wet_sender = sender.clone();
        }
        let mut written = 0;
        while written < out.len() {
            if self.control_left <= dt * 0.001 {
                // Retarget from the latest coherent controller state; never accumulate permanent latency after a stalled callback.
                if let Ok(requests) = self.control.request.try_lock() {
                    self.request = requests.latest;
                }
                self.control_left += 1.0 / 60.0;
            }
            let (accel, decel) = (sane(self.request.mix.accel), sane(self.request.mix.decel));
            if self.revision != Some(self.request.revision) {
                self.accel.synth.update_frequency(accel.frequency, 60.0);
                if let Some(stream) = &mut self.decel {
                    stream.synth.update_frequency(decel.frequency, 60.0);
                }
                self.revision = Some(self.request.revision);
                self.control.consumed_revision.store(self.request.revision, Ordering::Relaxed);
            }
            let take = ((self.control_left / dt - 1e-9).ceil() as usize).max(1).min(out.len() - written).min(256);
            let chunk = &mut out[written..written + take];
            let closed = self.control.closed.load(Ordering::Relaxed);
            let pan = (f32::from(self.request.mix.azimuth) / 65536.0 * std::f32::consts::TAU).sin();
            let dry = clean(self.request.mix.dry, 0.0, 1.0);
            let left_gain = dry * (1.0 - pan).sqrt();
            let right_gain = dry * (1.0 + pan).sqrt();
            for frame in chunk.iter_mut() {
                let mut sample = self.accel.sample(accel, dt);
                if let Some(stream) = &mut self.decel {
                    sample += stream.sample(decel, dt);
                }
                if closed {
                    self.release_left = (self.release_left - dt).max(0.0);
                    sample *= (self.release_left / 0.12) as f32;
                }
                // Constant-power pan normalized to the established centre gain.
                *frame = Frame::from_mono(sample);
            }
            let cutoff = clean(self.request.mix.low_pass_hz, 0.0, 25_000.0);
            self.filter_handle.set(cutoff);
            self.filter.process(chunk, dt, info);
            let mut wet = [0.0_f32; 256];
            let wet_gain = self.control.wet_gain.get();
            for (i, frame) in chunk.iter_mut().enumerate() {
                wet[i] = (frame.left + frame.right) * 0.5 * wet_gain;
                frame.left *= left_gain;
                frame.right *= right_gain;
            }
            if let Some(sender) = &self.wet_sender {
                sender.mono(written, &wet[..chunk.len()]);
            }
            self.control.frequency.set(self.accel.synth.current_frequency());
            self.control_left = (self.control_left - take as f64 * dt).max(0.0);
            written += take;
        }
    }

    fn finished(&self) -> bool {
        self.control.closed.load(Ordering::Relaxed) && self.release_left <= 0.0
    }
}

#[cfg(test)]
mod tests;
