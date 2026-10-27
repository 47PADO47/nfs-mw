//! The wind of the 2012 game as the files of the `wind_variant` effect variant (`profile.toml` and WAV loops under
//! `files/WIND/MW2012/`). Spec of the profile: `docs/modding/effect-variants.md`, "Profile format of the wind"; what the
//! 2012 data holds: `docs/formats/chameleon-sound-graph.md`, "The sound of speed".
//!
//! The 2012 game's `SpeedSoundSet.WindLayer` names a 100 ms stretch of silence in every slot, so its wind is not there. The wind
//! that is heard is a sequence of `SOUND/GLOBAL.BNDL` driven by the player's speed: loops in windows of the speed, each on a
//! mixer channel with a base gain, some with a curve of the speed of their own. This reads that sequence and writes, for each
//! loop, the level it has at every speed (the window with its fades, the item's curve and the channel's curve multiplied) as a
//! `[[layer]]` of the profile. Two loops swell while the car is in the air (their gain reads `TimeInAir`) and two sit on
//! channels of gain 0.0001; neither is written.

use std::path::Path;

use anyhow::{Context, Result, bail};
use chameleon_bundle::Bundle;
use chameleon_sound_graph::{Item, ItemKind, MixerItem, Modulator, Sequence, SoundGraphs};

use crate::sample::{Sample, wav_bytes};
use crate::source::Source;

/// The id of the variant in the `wind_variant` option, and what the menu shows for it.
pub const VARIANT: &str = "mw2012";
pub const LABEL: &str = "NFS Most Wanted 2012";
/// The folder of the variant under the mod's `files/`, and the bundle with the wind.
const FOLDER: &str = "WIND/MW2012";
const GLOBAL_SOUNDS: &str = "SOUND/GLOBAL.BNDL";
/// Speeds between the points of a layer's level curve, mph. A profile curve holds up to 64 points.
const STEP_MPH: f32 = 5.0;
/// A channel under this gain is muted in the data (0.0001) and its loops are not written.
const MUTED: f32 = 0.001;
/// The range of a playback rate and of a level the profile takes.
const RATES: std::ops::RangeInclusive<f32> = 0.25..=4.0;
const MAX_LEVEL: f32 = 8.0;

/// One loop of the wind: the sample, a constant level, the playback rate, and its level at the car's speed, as points
/// `(mph, level)`.
#[derive(Debug, Clone, PartialEq)]
pub struct WindLayer {
    pub sample: u64,
    pub volume: f32,
    pub rate: f32,
    pub gain: Vec<(f32, f32)>,
}

/// The loops of the wind, and how many the data has that are not written.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct WindSet {
    pub layers: Vec<WindLayer>,
    pub in_air: usize,
    pub muted: usize,
}

/// A value read at the speed `speed` of an item whose window is `item`: a modulator whose input range is the default 0 to 1
/// is read along the item's window (0 at its low end, 1 at its high end, the way the motor of an electric car is), any other
/// range is a range of the speed itself.
fn modulated(item: &Item, modulator: &Modulator, speed: f32) -> f32 {
    if modulator.input != (0.0, 1.0) {
        return modulator.eval(speed);
    }
    let span = item.hi - item.lo;
    let t = if span > f32::EPSILON { ((speed - item.lo) / span).clamp(0.0, 1.0) } else { 0.0 };
    modulator.eval(t)
}

/// The level the mixer item of `channel` gives at `speed` (1 when the sequence has no item for the channel).
fn channel_level(sequence: &Sequence, channel: u64, speed: f32) -> f32 {
    let found = sequence.items.iter().find_map(|item| match &item.kind {
        ItemKind::Mixer(MixerItem { channel: found, gains }) if *found == channel => Some((item, gains)),
        _ => None,
    });
    let Some((item, gains)) = found else { return 1.0 };
    let Some(gain) = gains.first().filter(|gain| gain.expression.is_empty()) else { return 1.0 };
    item.window(speed) * modulated(item, gain, speed)
}

/// The speeds a layer's curve is written at: a grid over the sequence's range and the corners of the item's window.
fn speeds(sequence: &Sequence, item: &Item) -> Vec<f32> {
    let (lo, hi) = (sequence.lo.min(sequence.hi), sequence.lo.max(sequence.hi));
    let mut found: Vec<f32> = Vec::new();
    let mut at = lo;
    while at < hi {
        found.push(at);
        at += STEP_MPH;
    }
    found.push(hi);
    for corner in
        [item.lo, item.hi, item.lo + item.fade_in.span, item.hi - item.fade_out.span, item.lo - 0.01, item.hi + 0.01]
    {
        found.push(corner);
    }
    found.retain(|speed| speed.is_finite() && (lo..=hi).contains(speed));
    found.sort_by(f32::total_cmp);
    found.dedup_by(|a, b| (*a - *b).abs() < 0.005);
    found
}

fn round(value: f32) -> f32 {
    (value * 1e4).round() / 1e4
}

/// The loops of the wind in `graphs`, `None` when the bundle has no wind sequence.
pub fn build(graphs: &SoundGraphs) -> Option<WindSet> {
    let sequence = graphs.wind_sequence()?;
    let mut set = WindSet::default();
    for item in &sequence.items {
        let ItemKind::Wave(wave) = &item.kind else { continue };
        let Some(&sample) = wave.waves.first() else { continue };
        let channel = graphs.channel_gains.get(&wave.mixer).copied().unwrap_or(1.0);
        if channel < MUTED {
            set.muted += 1;
            continue;
        }
        if !wave.gain.expression.is_empty() {
            set.in_air += 1;
            continue;
        }
        let own_curve = wave.gain.follows_input();
        let level = if own_curve { 1.0 } else { wave.gain.base };
        let gain: Vec<(f32, f32)> = speeds(sequence, item)
            .into_iter()
            .map(|speed| {
                let at = sequence.clamp(speed);
                let own = if own_curve { modulated(item, &wave.gain, at) } else { 1.0 };
                (
                    round(speed),
                    round((item.window(at) * own * channel_level(sequence, wave.mixer, at)).clamp(0.0, MAX_LEVEL)),
                )
            })
            .collect();
        let volume = round((level * channel).clamp(0.0, MAX_LEVEL));
        if gain.len() < 2 || gain.iter().all(|point| point.1 == 0.0) || volume == 0.0 {
            continue;
        }
        let rate = if wave.pitch.follows_input() { 1.0 } else { wave.pitch.base.clamp(*RATES.start(), *RATES.end()) };
        set.layers.push(WindLayer { sample, volume, rate: round(rate), gain });
    }
    Some(set)
}

/// The file name of the loop of `sample` under the variant's folder: `W` and the low 24 bits of the id in hex.
pub fn file_name(sample: u64) -> String {
    format!("W{:06X}.WAV", sample & 0xff_ffff)
}

/// The text of the profile of `set`.
pub fn profile_text(set: &WindSet) -> String {
    let mut text = String::from("format = 1\n");
    for layer in &set.layers {
        let points: Vec<String> = layer.gain.iter().map(|(speed, level)| format!("[{speed:?}, {level:?}]")).collect();
        text.push_str(&format!(
            "\n[[layer]]\nfile = \"{FOLDER}/{}\"\nvolume = {:?}\nrate = {:?}\ngain = [{}]\n",
            file_name(layer.sample),
            layer.volume,
            layer.rate,
            points.join(", ")
        ));
    }
    text
}

/// Read the wind of the install and write the variant under `out/files/WIND/MW2012/`. A failure only costs the variant: the
/// game's own wind stays the only choice.
pub fn write(source: &Source, out: &Path) {
    if let Err(error) = try_write(source, out) {
        log::warn!("the wind variant is not written: {error:#}");
    }
}

fn try_write(source: &Source, out: &Path) -> Result<()> {
    let bytes = source.install_file(GLOBAL_SOUNDS).with_context(|| format!("the install has no {GLOBAL_SOUNDS}"))?;
    let bundle = Bundle::parse(&bytes).with_context(|| format!("reading {GLOBAL_SOUNDS}"))?;
    let samples = crate::sample::read_all(&bundle);
    let set = build(&SoundGraphs::read(&bundle)).context("the global sounds have no wind sequence")?;
    let set = WindSet { layers: playable(set.layers, &samples), in_air: set.in_air, muted: set.muted };
    if set.layers.is_empty() {
        bail!("none of the wind loops decodes");
    }
    write_set(out, &set, &samples)
}

/// The layers whose loop is among the decoded `samples`.
fn playable(layers: Vec<WindLayer>, samples: &[Sample]) -> Vec<WindLayer> {
    layers.into_iter().filter(|layer| samples.iter().any(|sample| sample.id == layer.sample && sample.looped)).collect()
}

/// Write the loops and the profile of `set`.
fn write_set(out: &Path, set: &WindSet, samples: &[Sample]) -> Result<()> {
    let folder = out.join("files").join(FOLDER);
    std::fs::create_dir_all(&folder).with_context(|| format!("creating {}", folder.display()))?;
    for layer in &set.layers {
        let Some(sample) = samples.iter().find(|sample| sample.id == layer.sample) else { continue };
        let name = file_name(layer.sample);
        std::fs::write(folder.join(&name), wav_bytes(&sample.samples, sample.rate, 1.0))
            .with_context(|| format!("writing {name}"))?;
    }
    std::fs::write(folder.join("profile.toml"), profile_text(set)).context("writing the wind profile")?;
    log::info!("wind variant: {} loops ({} in the air, {} muted left out)", set.layers.len(), set.in_air, set.muted);
    Ok(())
}

#[cfg(test)]
mod real_tests;
#[cfg(test)]
mod tests;
