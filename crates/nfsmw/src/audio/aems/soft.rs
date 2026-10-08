//! A software mixer for the tests: plays the voices of the sample layer into a buffer, so the layer can be measured
//! (and written to a WAV) without an audio device.

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

use kira::Frame;
use kira::sound::static_sound::StaticSoundData;
use kira::sound::{EndPosition, PlaybackPosition};

use super::host::{Sampler, Voice, VoiceStatus};
use crate::audio::Audio;

/// The output rate of the mix.
pub const RATE: f64 = 48_000.0;

struct State {
    frames: Arc<[Frame]>,
    source_rate: f64,
    /// Position in source frames.
    position: f64,
    gain: f32,
    rate: f32,
    paused: bool,
    stopped: bool,
    /// Loop region in source frames.
    looping: Option<(f64, f64)>,
}

impl State {
    fn done(&self) -> bool {
        self.stopped || (self.looping.is_none() && self.position >= self.frames.len() as f64)
    }
}

struct Handle(Rc<RefCell<State>>);

impl Voice for Handle {
    fn set_gain(&mut self, gain: f32) {
        self.0.borrow_mut().gain = gain;
    }
    fn set_rate(&mut self, rate: f32) {
        self.0.borrow_mut().rate = rate;
    }
    fn pause(&mut self) {
        self.0.borrow_mut().paused = true;
    }
    fn resume(&mut self) {
        self.0.borrow_mut().paused = false;
    }
    fn stop(&mut self) {
        self.0.borrow_mut().stopped = true;
    }
    fn status(&self) -> VoiceStatus {
        let s = self.0.borrow();
        let elapsed = s.position / s.source_rate;
        let remaining =
            if s.looping.is_some() { 0.0 } else { (s.frames.len() as f64 / s.source_rate - elapsed).max(0.0) };
        VoiceStatus {
            playing: !s.done(),
            elapsed_ms: (elapsed * 1000.0) as i32,
            remaining_ms: (remaining * 1000.0) as i32,
        }
    }
}

impl Drop for Handle {
    fn drop(&mut self) {
        self.0.borrow_mut().stopped = true;
    }
}

/// The voices started so far.
#[derive(Default)]
pub struct Soft {
    voices: Vec<Rc<RefCell<State>>>,
    /// How many voices were ever started.
    pub started: usize,
}

fn seconds(position: PlaybackPosition, rate: f64) -> f64 {
    match position {
        PlaybackPosition::Seconds(s) => s,
        PlaybackPosition::Samples(n) => n as f64 / rate,
    }
}

impl Soft {
    /// Mixes `out.len()` samples (mono, from the left channel) of every voice into `out`.
    pub fn render(&mut self, out: &mut [f32]) {
        self.voices.retain(|v| !v.borrow().done());
        for voice in &self.voices {
            let mut s = voice.borrow_mut();
            if s.paused {
                continue;
            }
            for sample in out.iter_mut() {
                if s.done() {
                    break;
                }
                let (i, frac) = (s.position as usize, (s.position - s.position.floor()) as f32);
                let a = s.frames.get(i).map_or(0.0, |f| f.left);
                let b = s.frames.get(i + 1).map_or(a, |f| f.left);
                *sample += (a + (b - a) * frac) * s.gain;
                s.position += f64::from(s.rate) * s.source_rate / RATE;
                if let Some((start, end)) = s.looping
                    && s.position >= end
                {
                    s.position = start + (s.position - end) % (end - start).max(1.0);
                }
            }
        }
    }
}

/// The real bank sounds, played into a [`Soft`] mix.
pub struct SoftSampler<'a> {
    pub audio: &'a mut Audio,
    pub mix: &'a mut Soft,
}

impl Sampler for SoftSampler<'_> {
    fn sound(&mut self, bank: &str, index: usize) -> Result<StaticSoundData, String> {
        self.audio.bank_sound(bank, index)
    }

    fn start(&mut self, data: StaticSoundData, gain: f32, rate: f32) -> Option<Box<dyn Voice>> {
        let source_rate = f64::from(data.sample_rate);
        let looping = data.settings.loop_region.map(|region| {
            let start = seconds(region.start, source_rate) * source_rate;
            let end = match region.end {
                EndPosition::EndOfAudio => data.frames.len() as f64,
                EndPosition::Custom(p) => seconds(p, source_rate) * source_rate,
            };
            (start, end)
        });
        let state = State {
            frames: data.frames,
            source_rate,
            position: 0.0,
            gain,
            rate,
            paused: false,
            stopped: false,
            looping,
        };
        let state = Rc::new(RefCell::new(state));
        self.mix.voices.push(state.clone());
        self.mix.started += 1;
        Some(Box::new(Handle(state)))
    }
}
