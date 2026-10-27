//! A mod car's engine: layers that play all the time (docs/modding/car-sound.md, "Engine: `[engine_sound]`"). They are
//! the car's own recordings (`[engine_sound]`), or, for a car that gives none, loops the game synthesises
//! (`blackbox_carsound::synth`). A mod car has no engine voice of the game's cars: this is its whole engine.
//!
//! A layer without curves is loudest when the engine turns at its own rpm and fades out, equal power, into its
//! neighbour's; each is played at the engine's rpm over its own, so the pitch follows the engine, within a range that
//! keeps a layer from being stretched into mush. A layer with curves (`mixing`) is as loud as its curves say for the
//! car's engine speed, load and speed; a Ginsu layer is a granular engine recording that plays at the frequency its
//! span gives the engine speed.

use std::sync::Arc;
use std::time::Duration;

use blackbox_carsound::synth;
use blackbox_ginsu::{GinsuData, GinsuTables};
use ea_audio::Pcm;
use kira::sound::static_sound::StaticSoundHandle;
use kira::{PlaybackRate, Tween};
use nfsmw_data::car::mods::{EngineLayer, EngineSound, LayerKind};

use crate::audio::volume::decibels;
use crate::audio::{Audio, EngineHandle, EngineMix, EngineVoice, Group, pcm};

mod level;
mod mixing;
use level::{Normaliser, Recording};
pub use mixing::{EngineInputs, mix_inputs};
#[cfg(test)]
use mixing::{RATE_RANGE, mix};

/// The one layer the offline measurement lets sound (all of them when it is `usize::MAX`).
#[cfg(test)]
pub(super) static SOLO_LAYER: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(usize::MAX);

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

/// The built-in engine's description: the layers carry no file.
pub fn built_in() -> EngineSound {
    EngineSound {
        idle_rpm: BUILT_IN_IDLE_RPM,
        redline_rpm: BUILT_IN_REDLINE_RPM,
        layers: BUILT_IN_LAYER_RPMS.iter().map(|&rpm| EngineLayer::classic(BUILT_IN_SOURCE, rpm)).collect(),
    }
}

/// One playing layer.
enum Voice {
    Loop(StaticSoundHandle),
    Ginsu(EngineHandle),
}

/// What a layer is made of, before it plays.
enum Source {
    Loop(Pcm),
    Ginsu(Arc<GinsuData>),
}

impl Source {
    /// How loud the recording is, for the level of the engine (`level`).
    fn recording(&self) -> Recording {
        match self {
            Self::Loop(pcm) => Recording::of_loop(pcm),
            Self::Ginsu(data) => Recording::of_ginsu(data),
        }
    }
}

/// The layers of a car that is playing.
pub struct OwnEngine {
    sound: EngineSound,
    layers: Vec<Voice>,
    /// What the engine is scaled by so that every car is as loud as the cars of the game.
    level: Normaliser,
}

/// `mixed` with every layer but the solo one silenced, when the measurement asks for one.
#[cfg(test)]
fn solo(mut mixed: Vec<mixing::LayerMix>) -> Vec<mixing::LayerMix> {
    let only = SOLO_LAYER.load(std::sync::atomic::Ordering::Relaxed);
    for (at, layer) in mixed.iter_mut().enumerate() {
        if only != usize::MAX && at != only {
            layer.gain = 0.0;
        }
    }
    mixed
}

impl OwnEngine {
    /// Follow the engine when only its speed is known: `fraction` is how far it is from idle (0) to redline (1),
    /// `volume` the linear level. The load is taken as full and the car as standing.
    #[cfg(test)]
    pub fn update(&mut self, fraction: f32, volume: f32) {
        self.update_inputs(&EngineInputs { rpm: fraction, load: 1.0, speed_mph: 0.0 }, volume);
    }

    /// Follow the engine speed, the throttle and the car's speed; `volume` is the linear level.
    pub fn update_inputs(&mut self, inputs: &EngineInputs, volume: f32) {
        let tween = Tween { duration: SMOOTH, ..Tween::default() };
        let mixed = mix_inputs(&self.sound, inputs);
        #[cfg(test)]
        let mixed = solo(mixed);
        let volume = volume * self.level.scale(inputs);
        for (voice, layer) in self.layers.iter_mut().zip(mixed) {
            match voice {
                Voice::Loop(handle) => {
                    handle.set_volume(decibels(layer.gain * volume), tween);
                    handle.set_playback_rate(PlaybackRate(f64::from(layer.rate)), tween);
                }
                Voice::Ginsu(handle) => {
                    handle.set(EngineMix::shared(layer.frequency, layer.rate, layer.gain * volume, 0.0));
                }
            }
        }
    }
}

impl Drop for OwnEngine {
    /// A kira handle that is dropped leaves its sound playing, so the loops are stopped here. A Ginsu voice ends with
    /// its handle.
    fn drop(&mut self) {
        let fade = Tween { duration: Duration::from_millis(120), ..Tween::default() };
        for voice in &mut self.layers {
            if let Voice::Loop(handle) = voice {
                handle.stop(fade);
            }
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
        let mut sources = Vec::with_capacity(sound.layers.len());
        for layer in &sound.layers {
            let bytes = self.dir.read(&layer.file).map_err(|e| format!("{}: {e:#}", layer.file))?;
            let source = match layer.kind {
                LayerKind::Loop => Source::Loop(parse_wav(&bytes).map_err(|e| format!("{}: {e}", layer.file))?),
                LayerKind::Ginsu => {
                    Source::Ginsu(Arc::new(load_ginsu(&bytes).map_err(|e| format!("{}: {e}", layer.file))?))
                }
            };
            sources.push(source);
        }
        self.start_layers(sound.clone(), sources)
    }

    /// Start the built-in engine, silent.
    pub(super) fn start_built_in_engine(&mut self) -> Result<OwnEngine, String> {
        let sound = built_in();
        let sources = sound.layers.iter().map(|layer| Source::Loop(built_in_loop(layer.rpm))).collect();
        self.start_layers(sound, sources)
    }

    fn start_layers(&mut self, sound: EngineSound, sources: Vec<Source>) -> Result<OwnEngine, String> {
        let mut layers = Vec::with_capacity(sources.len());
        for source in &sources {
            let voice = match source {
                Source::Loop(pcm) => Voice::Loop(self.play(Group::Engine, pcm::sound(pcm).volume(decibels(0.0)))?),
                Source::Ginsu(data) => {
                    let silent = EngineMix::shared(data.tables().min_frequency(), 1.0, 0.0, 0.0);
                    let voice = EngineVoice { accel: data.clone(), decel: None, start: silent };
                    Voice::Ginsu(self.start_engine(voice)?)
                }
            };
            layers.push(voice);
        }
        let recordings: Vec<Recording> = sources.iter().map(Source::recording).collect();
        let level = Normaliser::new(&sound, &recordings);
        Ok(OwnEngine { sound, layers, level })
    }
}

/// A `.gin` file: the tables of `blackbox-ginsu` and the audio of `ea-audio`.
fn load_ginsu(bytes: &[u8]) -> Result<GinsuData, String> {
    let (tables, _) = GinsuTables::parse(bytes).map_err(|e| format!("its tables: {e}"))?;
    let pcm = ea_audio::gin::decode(bytes).map_err(|e| format!("its audio: {e}"))?;
    if pcm.sample_rate != tables.sample_rate() {
        return Err(format!("the audio is {} Hz and the tables {} Hz", pcm.sample_rate, tables.sample_rate()));
    }
    GinsuData::from_i16(tables, &pcm.samples).map_err(|e| e.to_string())
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
