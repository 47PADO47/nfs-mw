//! The engine voice: a `kira` sound that runs one or two Ginsu synthesisers (the accelerate and the
//! decelerate loop) and resamples their output to the device. The game thread sets the mix through an
//! [`EngineHandle`]; the audio thread reads it once per callback.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

use blackbox_ginsu::{GinsuData, GinsuSynth, SynthParams};
use kira::Frame;
use kira::info::Info;
use kira::sound::{Sound, SoundData};

/// Samples a synthesiser renders at once.
const CHUNK: usize = 256;

/// What the engine should sound like right now, from the sound mix controller.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EngineMix {
    /// Target frequency in the loops' units (engine "RPM").
    pub frequency: f32,
    /// Linear volume of the accelerate and the decelerate loop.
    pub accel_volume: f32,
    pub decel_volume: f32,
    /// Playback rate multiplier (1 = natural).
    pub pitch: f32,
}

impl Default for EngineMix {
    fn default() -> Self {
        Self { frequency: 0.0, accel_volume: 0.0, decel_volume: 0.0, pitch: 1.0 }
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

struct Control {
    frequency: Cell,
    accel_volume: Cell,
    decel_volume: Cell,
    pitch: Cell,
    closed: AtomicBool,
}

/// The game's end of a playing engine voice. Dropping it ends the voice.
pub struct EngineHandle(Arc<Control>);

impl EngineHandle {
    pub fn set(&self, mix: EngineMix) {
        let c = &self.0;
        c.frequency.set(mix.frequency);
        c.accel_volume.set(mix.accel_volume);
        c.decel_volume.set(mix.decel_volume);
        c.pitch.set(mix.pitch);
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
        let control = Arc::new(Control {
            frequency: Cell::new(self.start.frequency),
            accel_volume: Cell::new(self.start.accel_volume),
            decel_volume: Cell::new(self.start.decel_volume),
            pitch: Cell::new(self.start.pitch),
            closed: AtomicBool::new(false),
        });
        let start = self.start.frequency;
        let accel = Stream::new(self.accel, start)?;
        let decel = self.decel.map(|d| Stream::new(d, start)).transpose()?;
        Ok((Box::new(EngineSound { control: control.clone(), accel, decel }), EngineHandle(control)))
    }
}

/// One loop's synthesiser and the linear resampler that brings it to the device rate.
struct Stream {
    synth: GinsuSynth,
    data: Arc<GinsuData>,
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
        Ok(Self { synth, data, rate, buffer: [0.0; CHUNK], index: CHUNK, previous: 0.0, next: 0.0, position: 0.0 })
    }

    /// The next output sample at `dt` seconds per output frame.
    fn sample(&mut self, frequency: f32, volume: f32, pitch: f32, dt: f64) -> f32 {
        // A request below the loop's lowest frequency plays the lowest one at a lower rate.
        let (frequency, ratio) = self.data.tables().clamp_frequency(frequency);
        self.position += self.rate * f64::from(ratio * pitch) * dt;
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
}

/// A finite, sane value in `lo..=hi` (NaN counts as `lo`).
fn clean(v: f32, lo: f32, hi: f32) -> f32 {
    if v.is_nan() { lo } else { v.clamp(lo, hi) }
}

impl Sound for EngineSound {
    fn process(&mut self, out: &mut [Frame], dt: f64, _info: &Info) {
        let c = &self.control;
        let frequency = clean(c.frequency.get(), 0.0, 100_000.0);
        let (accel, decel) = (clean(c.accel_volume.get(), 0.0, 4.0), clean(c.decel_volume.get(), 0.0, 4.0));
        let pitch = clean(c.pitch.get(), 0.05, 8.0);
        for frame in out.iter_mut() {
            let mut sample = self.accel.sample(frequency, accel, pitch, dt);
            if let Some(decel_stream) = &mut self.decel {
                sample += decel_stream.sample(frequency, decel, pitch, dt);
            }
            *frame = Frame::from_mono(sample);
        }
    }

    fn finished(&self) -> bool {
        self.control.closed.load(Ordering::Relaxed)
    }
}

#[cfg(test)]
mod tests {
    use blackbox_ginsu::GinsuTables;

    use super::*;

    const RATE: u32 = 32_000;

    /// A loop whose recording is a sine rising from 100 to 400 Hz, like a rev sweep, with the tables built
    /// from it: cycle starts at every zero turn and 50 frequency segments.
    fn data() -> Arc<GinsuData> {
        let n = (RATE as usize) * 2;
        let (mut pcm, mut phase) = (Vec::with_capacity(n), 0.0f32);
        let (mut cycle_pos, mut freq_at) = (vec![0u32], Vec::with_capacity(n));
        for i in 0..n {
            let hz = 100.0 + 300.0 * i as f32 / n as f32;
            let before = phase;
            phase += hz / RATE as f32;
            if phase.floor() > before.floor() {
                cycle_pos.push(i as u32);
            }
            freq_at.push(hz * 120.0);
            pcm.push((phase * std::f32::consts::TAU).sin() * 0.5);
        }
        let (lo, hi) = (freq_at[0], *freq_at.last().unwrap());
        let freq_pos = (0..=50)
            .map(|s| {
                let f = lo + (hi - lo) * s as f32 / 50.0;
                freq_at.iter().position(|&x| x >= f).unwrap_or(n - 1) as u32
            })
            .collect();
        let tables = GinsuTables::new(lo, hi, RATE, n as u32, freq_pos, cycle_pos).expect("tables");
        Arc::new(GinsuData::new(tables, pcm).expect("data"))
    }

    fn render(voice: EngineVoice, mixes: &[(EngineMix, usize)], out_rate: f64) -> Vec<f32> {
        let (mut sound, handle) = voice.into_sound().unwrap();
        let info = kira::info::MockInfoBuilder::new().build();
        let mut out = Vec::new();
        for &(mix, frames) in mixes {
            handle.set(mix);
            let mut block = vec![Frame::ZERO; frames];
            sound.process(&mut block, 1.0 / out_rate, &info);
            out.extend(block.iter().map(|f| f.left));
        }
        out
    }

    fn rising_zero_crossings_hz(samples: &[f32], rate: f64) -> f64 {
        let crossings = samples.windows(2).filter(|w| w[0] <= 0.0 && w[1] > 0.0).count();
        crossings as f64 / (samples.len() as f64 / rate)
    }

    fn mix(hz: f32, pitch: f32) -> EngineMix {
        EngineMix { frequency: hz * 120.0, accel_volume: 1.0, decel_volume: 0.0, pitch }
    }

    #[test]
    fn the_pitch_follows_the_requested_frequency_at_the_device_rate() {
        let d = data();
        for hz in [150.0f32, 250.0, 350.0] {
            let voice = EngineVoice { accel: d.clone(), decel: None, start: EngineMix::default() };
            let samples = render(voice, &[(mix(hz, 1.0), 48_000)], 48_000.0);
            let measured = rising_zero_crossings_hz(&samples[24_000..], 48_000.0);
            assert!((measured - f64::from(hz)).abs() < f64::from(hz) * 0.1, "asked {hz} Hz, heard {measured:.1} Hz");
        }
    }

    #[test]
    fn a_playback_rate_shifts_the_pitch() {
        let voice = EngineVoice { accel: data(), decel: None, start: EngineMix::default() };
        let samples = render(voice, &[(mix(200.0, 1.5), 48_000)], 48_000.0);
        let measured = rising_zero_crossings_hz(&samples[24_000..], 48_000.0);
        assert!((measured - 300.0).abs() < 30.0, "{measured}");
    }

    #[test]
    fn zero_volume_is_silence_and_bad_numbers_do_not_poison_the_output() {
        let d = data();
        let voice = EngineVoice { accel: d.clone(), decel: Some(d), start: EngineMix::default() };
        let quiet = EngineMix { accel_volume: 0.0, ..mix(200.0, 1.0) };
        let bad = EngineMix { frequency: f32::NAN, accel_volume: f32::INFINITY, decel_volume: -3.0, pitch: f32::NAN };
        let out = render(voice, &[(quiet, 8_000), (bad, 8_000)], 48_000.0);
        assert!(out[2_000..8_000].iter().all(|s| s.abs() < 1e-3), "silent at zero volume");
        assert!(out.iter().all(|s| s.is_finite() && s.abs() <= 8.0));
    }

    #[test]
    fn dropping_the_handle_finishes_the_voice() {
        let voice = EngineVoice { accel: data(), decel: None, start: EngineMix::default() };
        let (sound, handle) = voice.into_sound().unwrap();
        assert!(!sound.finished());
        drop(handle);
        assert!(sound.finished());
    }
}
