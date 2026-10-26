//! A mod car's engine: loops that play all the time and are mixed by engine speed (docs/modding/car-sound.md,
//! "Engine: `[engine_sound]`"). They are the car's own recordings (`[engine_sound]`), or, for a car that gives none,
//! loops the game synthesises (`blackbox_carsound::synth`). A mod car has no Ginsu voice: this is its whole engine.
//!
//! A layer is loudest when the engine turns at its own rpm and fades out, equal power, into its neighbour's; each is
//! played at the engine's rpm over its own, so the pitch follows the engine, within a range that keeps a layer from
//! being stretched into mush.

use std::time::Duration;

use blackbox_carsound::synth;
use ea_audio::Pcm;
use kira::sound::static_sound::StaticSoundHandle;
use kira::{PlaybackRate, Tween};
use nfsmw_data::car::mods::{EngineLayer, EngineSound};

use crate::audio::volume::decibels;
use crate::audio::{Audio, Group, pcm};

/// A layer plays no faster or slower than this, so one recording is not asked to cover the whole range.
const RATE_RANGE: (f32, f32) = (0.55, 1.8);
/// The volume and pitch change over this long, so a frame's step is not heard as a click.
const SMOOTH: Duration = Duration::from_millis(50);

/// The built-in engine, for a car that gives no recordings of its own: a six-cylinder four-stroke that idles at 900 rpm
/// and is limited at 7000, with loops at these engine speeds. Nothing about it is any car's; it is a plain engine.
const BUILT_IN_CYLINDERS: u32 = 6;
const BUILT_IN_IDLE_RPM: f32 = 900.0;
const BUILT_IN_REDLINE_RPM: f32 = 7000.0;
const BUILT_IN_LAYER_RPMS: [f32; 5] = [900.0, 1700.0, 2900.0, 4400.0, 6500.0];
/// The sample rate the built-in loops are made at.
const BUILT_IN_RATE: u32 = 32_000;
/// The name the built-in layers carry where a file name would be.
const BUILT_IN_SOURCE: &str = "(built-in engine)";

/// What one layer should sound like right now.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LayerMix {
    pub gain: f32,
    pub rate: f32,
}

/// The mix of every layer of `sound` at engine speed `rpm`.
pub fn mix(sound: &EngineSound, rpm: f32) -> Vec<LayerMix> {
    let layers = &sound.layers;
    let rate = |layer: usize| (rpm / layers[layer].rpm).clamp(RATE_RANGE.0, RATE_RANGE.1);
    let mut out = vec![LayerMix { gain: 0.0, rate: 1.0 }; layers.len()];
    for (at, slot) in out.iter_mut().enumerate() {
        slot.rate = rate(at);
    }
    let Some(above) = layers.iter().position(|layer| layer.rpm >= rpm) else {
        let last = layers.len() - 1;
        out[last].gain = 1.0;
        return out;
    };
    if above == 0 {
        out[0].gain = 1.0;
        return out;
    }
    let below = above - 1;
    let span = (layers[above].rpm / layers[below].rpm).ln();
    let t = ((rpm / layers[below].rpm).ln() / span).clamp(0.0, 1.0);
    out[below].gain = (t * std::f32::consts::FRAC_PI_2).cos();
    out[above].gain = (t * std::f32::consts::FRAC_PI_2).sin();
    out
}

/// The built-in engine's description: the layers carry no file.
pub fn built_in() -> EngineSound {
    EngineSound {
        idle_rpm: BUILT_IN_IDLE_RPM,
        redline_rpm: BUILT_IN_REDLINE_RPM,
        layers: BUILT_IN_LAYER_RPMS.iter().map(|&rpm| EngineLayer { file: BUILT_IN_SOURCE.into(), rpm }).collect(),
    }
}

/// The loops of a car that is playing.
pub struct OwnEngine {
    sound: EngineSound,
    layers: Vec<StaticSoundHandle>,
}

impl OwnEngine {
    /// Follow the engine: `fraction` is how far it is from idle (0) to redline (1), `volume` the linear level.
    pub fn update(&mut self, fraction: f32, volume: f32) {
        let tween = Tween { duration: SMOOTH, ..Tween::default() };
        let mix = mix(&self.sound, self.sound.rpm_at(fraction));
        for (handle, layer) in self.layers.iter_mut().zip(mix) {
            handle.set_volume(decibels(layer.gain * volume), tween);
            handle.set_playback_rate(PlaybackRate(f64::from(layer.rate)), tween);
        }
    }
}

impl Drop for OwnEngine {
    /// A kira handle that is dropped leaves its sound playing, so the loops are stopped here.
    fn drop(&mut self) {
        let fade = Tween { duration: Duration::from_millis(120), ..Tween::default() };
        for handle in &mut self.layers {
            handle.stop(fade);
        }
    }
}

impl Audio {
    /// The engine of the mod car `name`: its own recordings when it gives them, else (and when a recording cannot be
    /// read) the built-in engine.
    pub(super) fn start_mod_engine(&mut self, name: &str) -> Result<OwnEngine, String> {
        if let Some(sound) = nfsmw_data::car::mods::engine_sound(name) {
            match self.start_own_engine(sound) {
                Ok(engine) => return Ok(engine),
                Err(e) => log::warn!("{name}: its engine recordings are not used, the built-in engine plays: {e}"),
            }
        }
        self.start_built_in_engine()
    }

    /// Start every layer of `sound`, silent. A layer that cannot be read or played fails the whole engine, so the car
    /// plays the built-in engine rather than half an engine.
    pub(super) fn start_own_engine(&mut self, sound: &EngineSound) -> Result<OwnEngine, String> {
        let mut layers = Vec::with_capacity(sound.layers.len());
        for layer in &sound.layers {
            let bytes = self.dir.read(&layer.file).map_err(|e| format!("{}: {e:#}", layer.file))?;
            let pcm = parse_wav(&bytes).map_err(|e| format!("{}: {e}", layer.file))?;
            layers.push(pcm);
        }
        self.start_layers(sound.clone(), layers)
    }

    /// Start the built-in engine, silent.
    pub(super) fn start_built_in_engine(&mut self) -> Result<OwnEngine, String> {
        let sound = built_in();
        let layers = sound.layers.iter().map(|layer| built_in_loop(layer.rpm)).collect();
        self.start_layers(sound, layers)
    }

    fn start_layers(&mut self, sound: EngineSound, pcms: Vec<Pcm>) -> Result<OwnEngine, String> {
        let mut layers = Vec::with_capacity(pcms.len());
        for pcm in &pcms {
            layers.push(self.play(Group::Engine, pcm::sound(pcm).volume(decibels(0.0)))?);
        }
        Ok(OwnEngine { sound, layers })
    }
}

/// The built-in engine's loop at `rpm`.
fn built_in_loop(rpm: f32) -> Pcm {
    let samples = synth::to_pcm16(&synth::engine_loop(rpm, BUILT_IN_CYLINDERS, BUILT_IN_RATE));
    let frames = samples.len() as u32;
    Pcm { sample_rate: BUILT_IN_RATE, channels: 1, samples, loop_range: Some((0, frames)) }
}

/// A 16-bit PCM WAV file, mono or stereo, as a sound that loops over its whole length.
pub fn parse_wav(bytes: &[u8]) -> Result<Pcm, String> {
    if bytes.get(..4) != Some(b"RIFF") || bytes.get(8..12) != Some(b"WAVE") {
        return Err("not a WAV file".into());
    }
    let (mut channels, mut sample_rate, mut data) = (0u16, 0u32, None);
    let mut at = 12;
    while at + 8 <= bytes.len() {
        let size = u32::from_le_bytes([bytes[at + 4], bytes[at + 5], bytes[at + 6], bytes[at + 7]]) as usize;
        let body = bytes.get(at + 8..at + 8 + size).ok_or("a chunk runs past the end of the file")?;
        match &bytes[at..at + 4] {
            b"fmt " if body.len() >= 16 => {
                let format = u16::from_le_bytes([body[0], body[1]]);
                let bits = u16::from_le_bytes([body[14], body[15]]);
                if format != 1 || bits != 16 {
                    return Err("only 16-bit PCM is read".into());
                }
                channels = u16::from_le_bytes([body[2], body[3]]);
                sample_rate = u32::from_le_bytes([body[4], body[5], body[6], body[7]]);
            }
            b"data" => data = Some(body),
            _ => {}
        }
        at += 8 + size + (size & 1);
    }
    let data = data.ok_or("no data chunk")?;
    if !(1..=2).contains(&channels) || sample_rate == 0 {
        return Err("a WAV of one or two channels is needed".into());
    }
    let samples: Vec<i16> = data.as_chunks::<2>().0.iter().map(|pair| i16::from_le_bytes(*pair)).collect();
    let frames = (samples.len() / usize::from(channels)) as u32;
    if frames == 0 {
        return Err("the WAV has no samples".into());
    }
    Ok(Pcm { sample_rate, channels, samples, loop_range: Some((0, frames)) })
}

#[cfg(test)]
mod real_tests;
#[cfg(test)]
mod tests;
