//! The car's own engine sound, from the sample resources of its 2012 bundle (docs/formats/chameleon-sound.md).
//!
//! A car bundle holds about forty samples: mono 48 kHz `EL31` streams behind a 128-byte header, seven or so of them
//! loops. The 2012 game mixes them in a sound graph with synthesised harmonics, which this mod does not rebuild. What
//! it does instead is read each loop's pitch, treat the loops as recordings of the engine at different speeds (a loop's
//! rpm is the idle rpm times its pitch over the lowest pitch), and let the game's mixer play them by engine speed. That
//! is an approximation. The loops another effect claims (the turbo's whistle, see `effects`) are left out; the rest are
//! taken as the engine's, which includes the five loops of the graph that reads `GetTransmissionSpeedMph` (by its
//! name, the transmission's whine, not an engine) because the engine alone has too few pitches to cover the range.

use std::collections::BTreeSet;

use crate::sample::{Sample, gain_to_rms, wav_bytes};

/// Shortest loop kept: shorter ones are one-shot-like and repeat too fast.
const MIN_SECONDS: f32 = 0.8;
/// The pitch range searched, in Hz: idle of a big engine to the whine of a high-revving one.
const PITCH_RANGE: (f32, f32) = (20.0, 700.0);
/// Layers closer than this ratio in pitch are one layer.
const SAME_PITCH: f32 = 1.06;
/// A layer above this many times the redline would be played at a pitch the engine never reaches.
const BEYOND_REDLINE: f32 = 1.15;
/// The level the layers are brought to (root mean square).
const TARGET_RMS: f32 = 0.2;
/// Most layers a car gets (what the car manifest allows).
const MAX_LAYERS: usize = 8;

/// One looping sample of a car, decoded.
pub struct Loop {
    pub samples: Vec<f32>,
    pub rate: u32,
    /// The pitch of the loop in Hz, 0 when it has none.
    pub pitch: f32,
}

/// A layer of the car's engine sound: a loop and the engine speed it stands for, or a layer that plays by the curves
/// of the car's sound graphs (`graph_voices`).
pub struct Layer {
    pub rpm: f32,
    /// The bytes of the file: a WAV, or a `.gin` file for a Ginsu layer.
    pub wav: Vec<u8>,
    pub curves: Option<Curved>,
}

/// The manifest fields of a layer that plays by curves.
pub struct Curved {
    /// The file is a Ginsu recording (`.GIN`), not a WAV.
    pub ginsu: bool,
    /// The fields of the layer's inline table after `file` (`volume = ..., gain = [...]`).
    pub fields: String,
}

/// The looping samples among `samples` that are long enough to be an engine's, except the ones in `not_engine` (the
/// loops other effects use, such as the turbo's whistle).
pub fn loops(samples: &[Sample], not_engine: &BTreeSet<u64>) -> Vec<Loop> {
    samples
        .iter()
        .filter(|sample| sample.looped && !not_engine.contains(&sample.id) && sample.seconds() >= MIN_SECONDS)
        .map(|sample| Loop {
            pitch: pitch_of(&sample.samples, sample.rate),
            samples: sample.samples.clone(),
            rate: sample.rate,
        })
        .collect()
}

/// The pitch of `samples` in Hz: the first strong peak of the normalised autocorrelation of a stretch from the middle of
/// the loop, or 0 when nothing in [`PITCH_RANGE`] repeats.
pub fn pitch_of(samples: &[f32], rate: u32) -> f32 {
    let window = 8192.min(samples.len() / 2);
    let from = (samples.len() - window) / 2;
    let stretch = &samples[from..from + window];
    let (shortest, longest) = ((rate as f32 / PITCH_RANGE.1) as usize, (rate as f32 / PITCH_RANGE.0) as usize);
    let longest = longest.min(window / 2);
    if shortest >= longest {
        return 0.0;
    }
    let energy: f32 = stretch.iter().map(|s| s * s).sum();
    if energy <= 1e-6 {
        return 0.0;
    }
    let correlation: Vec<f32> = (shortest..longest)
        .map(|lag| stretch[..window - lag].iter().zip(&stretch[lag..]).map(|(a, b)| a * b).sum::<f32>() / energy)
        .collect();
    let best = correlation.iter().copied().fold(f32::MIN, f32::max);
    if best < 0.2 {
        return 0.0;
    }
    // The first local peak that is nearly as strong as the best: a multiple of the period is as strong or stronger.
    let peak = (1..correlation.len().saturating_sub(1)).find(|&at| {
        correlation[at] >= 0.85 * best
            && correlation[at] >= correlation[at - 1]
            && correlation[at] > correlation[at + 1]
    });
    let Some(at) = peak else { return 0.0 };
    rate as f32 / (shortest + at) as f32
}

/// The layers of an engine that idles at `idle_rpm` and is limited at `redline_rpm`, from the loops of its car: the
/// pitched loops, slowest first, each given the rpm its pitch stands for, at one level. Empty when there is no pitched
/// loop.
pub fn layers(loops: Vec<Loop>, idle_rpm: f32, redline_rpm: f32) -> Vec<Layer> {
    let mut pitched: Vec<Loop> = loops.into_iter().filter(|l| l.pitch > 0.0).collect();
    pitched.sort_by(|a, b| a.pitch.total_cmp(&b.pitch));
    let Some(lowest) = pitched.first().map(|l| l.pitch) else { return Vec::new() };
    let mut layers: Vec<(f32, Loop)> = Vec::new();
    for loop_ in pitched {
        let rpm = idle_rpm * loop_.pitch / lowest;
        if rpm > redline_rpm * BEYOND_REDLINE {
            continue;
        }
        let same = layers.last().is_some_and(|(known, _)| rpm / known < SAME_PITCH);
        if same {
            continue;
        }
        layers.push((rpm, loop_));
    }
    layers.into_iter().take(MAX_LAYERS).map(|(rpm, l)| Layer { rpm, wav: loop_wav(&l), curves: None }).collect()
}

/// A 16-bit mono WAV of the loop at the level every layer is brought to.
fn loop_wav(loop_: &Loop) -> Vec<u8> {
    let rms = (loop_.samples.iter().map(|s| s * s).sum::<f32>() / loop_.samples.len().max(1) as f32).sqrt();
    wav_bytes(&loop_.samples, loop_.rate, gain_to_rms(rms, TARGET_RMS, 8.0))
}

#[cfg(test)]
mod tests;
