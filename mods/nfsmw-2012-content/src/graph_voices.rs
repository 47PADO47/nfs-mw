//! The real engine sound of a 2012 car, from the sound graphs of its bundle (docs/formats/chameleon-sound-graph.md).
//!
//! The graphs hold one sequence per live value. This reads the three that make the engine:
//!
//! - the **audible engine speed** sequence of the local player: three Ginsu voices (granular engine recordings) and the
//!   idle loop, each active in a window of the audible engine speed with fades at its ends;
//! - the **engine load** sequence: mixer items that set the gain of each Ginsu voice's channel by a curve of the load
//!   (the voices fade into one another as the throttle opens);
//! - the **transmission speed** sequence: five whine loops, each active in a window of the car's speed and played at the
//!   speed over the speed it was recorded at.
//!
//! Each becomes one layer of the car's `[engine_sound]` with curves over the engine's own inputs: the audible engine
//! speed is taken to span idle to redline, so its windows are written as a fraction of the way from idle (the two ends of
//! the sequence are the ends of the range), the load as the throttle and the transmission speed as the car's speed. A car
//! whose bundle has no such sequence gets no layers here and keeps the pitch heuristic of `engine`.

use chameleon_bundle::Bundle;
use chameleon_sound_graph::{GinsuItem, Item, ItemKind, Modulator, Sequence, SoundGraphs, WaveItem, ginsu_resource};

use crate::engine::{Curved, Layer};
use crate::sample::{Sample, wav_bytes};

const ENGINE_RPM: &str = "Players.LocalPlayer.GetPlayerAudibleRPM";
const ENGINE_LOAD: &str = "Players.LocalPlayer.GetPlayerEngineLoad";
const WHINE_SPEED: &str = "GetTransmissionSpeedMph";
/// Samples taken along a load curve.
const LOAD_SAMPLES: usize = 17;
/// Points on each fade of a window.
const FADE_STEPS: usize = 6;
/// Most layers a car gets (what the car manifest allows).
const MAX_LAYERS: usize = 12;
/// The level of the transmission whine loops next to the rest of the engine, a factor on the gain the graph gives them. By the
/// graph alone (a gain of 0.6 on a channel at -16 dB) they sound 6 dB under the whole engine at a spectral centroid of 1 to
/// 3 kHz, which made the converted cars much brighter than the cars of the game (centroid at the limiter 745 Hz against 470
/// to 580 Hz for the stock cars, measured); at this factor the whine is a detail 15 dB under the engine.
const WHINE_BALANCE: f32 = 0.35;
/// The largest level of a curve point (what the car manifest allows).
const MAX_LEVEL: f32 = 8.0;

/// What a window is a function of.
#[derive(Clone, Copy)]
enum Domain {
    /// The audible engine speed, written as a fraction of the sequence's range.
    Rpm { lo: f32, hi: f32 },
    /// The transmission speed in miles per hour.
    Speed,
}

impl Domain {
    fn input(self) -> &'static str {
        match self {
            Self::Rpm { .. } => "rpm",
            Self::Speed => "speed_mph",
        }
    }

    /// The manifest's x of the live value `value`.
    fn x(self, value: f32) -> f32 {
        match self {
            Self::Rpm { lo, hi } => (value - lo) / (hi - lo),
            Self::Speed => value,
        }
    }
}

/// The layers of the engine the sound graphs describe, or none.
pub fn build(bundle: &Bundle, graphs: &SoundGraphs, samples: &[Sample]) -> Vec<Layer> {
    let mut layers = Vec::new();
    // A car with no Ginsu voice (the electric one) has the whine loops of its transmission alone: they are its motor.
    if let Some(engine) = engine_sequence(graphs) {
        let domain = Domain::Rpm { lo: engine.lo, hi: engine.hi };
        for item in &engine.items {
            let layer = match &item.kind {
                ItemKind::Ginsu(ginsu) => ginsu_layer(bundle, graphs, engine, item, ginsu),
                ItemKind::Wave(wave) => loop_layer(graphs, samples, item, wave, domain, engine),
                _ => None,
            };
            layers.extend(layer);
        }
    }
    for whine in graphs.driven_by(WHINE_SPEED).filter(|sequence| sequence.items.len() > 1) {
        for item in &whine.items {
            if let ItemKind::Wave(wave) = &item.kind {
                layers.extend(loop_layer(graphs, samples, item, wave, Domain::Speed, whine));
            }
        }
    }
    layers.truncate(MAX_LAYERS);
    layers
}

/// The sequence of the engine: of the local player's sequences driven by the audible engine speed that hold Ginsu voices (a
/// car has two sets), the first whose items sound at the bottom of the range. Some cars' first set starts its items above
/// the bottom of its range and is silent at idle; the other set is not.
fn engine_sequence(graphs: &SoundGraphs) -> Option<&Sequence> {
    let mut sets = graphs
        .driven_by(ENGINE_RPM)
        .filter(|sequence| sequence.items.iter().any(|item| matches!(item.kind, ItemKind::Ginsu(_))))
        .peekable();
    let first = sets.peek().copied()?;
    let sounds_at_idle = |sequence: &Sequence| {
        sequence.items.iter().any(|item| matches!(item.kind, ItemKind::Wave(_)) && item.window(sequence.lo) > 0.0)
    };
    sets.find(|sequence| sounds_at_idle(sequence)).or(Some(first))
}

fn ginsu_layer(
    bundle: &Bundle,
    graphs: &SoundGraphs,
    sequence: &Sequence,
    item: &Item,
    ginsu: &GinsuItem,
) -> Option<Layer> {
    let resource = ginsu_resource(bundle, ginsu.resource)?;
    let domain = Domain::Rpm { lo: sequence.lo, hi: sequence.hi };
    let mut gain = vec![window_curve(item, sequence, domain)?];
    gain.extend(load_curve(graphs, ginsu.mixer));
    let volume = channel_gain(graphs, ginsu.mixer) * resource.volume;
    let fields = format!(
        "kind = \"ginsu\", span = [{:?}, {:?}], volume = {:?}, gain = [{}]",
        sequence.lo,
        sequence.hi,
        round(volume),
        gain.join(", ")
    );
    Some(Layer { rpm: 0.0, wav: resource.to_gin(), curves: Some(Curved { ginsu: true, fields }) })
}

fn loop_layer(
    graphs: &SoundGraphs,
    samples: &[Sample],
    item: &Item,
    wave: &WaveItem,
    domain: Domain,
    sequence: &Sequence,
) -> Option<Layer> {
    let sample = samples.iter().find(|sample| wave.waves.first() == Some(&sample.id) && sample.looped)?;
    let gain = [window_curve(item, sequence, domain)?];
    let level = if wave.gain.follows_input() { 1.0 } else { wave.gain.base };
    let balance = if matches!(domain, Domain::Speed) { WHINE_BALANCE } else { 1.0 };
    let volume = round(level * channel_gain(graphs, wave.mixer) * balance);
    let pitch = match (wave.tracks_driver, domain) {
        (true, Domain::Speed) if wave.pitch.base > 0.0 => {
            format!(", pitch = {{ input = \"speed_mph\", reference = {:?} }}", wave.pitch.base)
        }
        _ => String::new(),
    };
    let fields = format!("volume = {volume:?}, gain = [{}]{pitch}", gain.join(", "));
    Some(Layer {
        rpm: 0.0,
        wav: wav_bytes(&sample.samples, sample.rate, 1.0),
        curves: Some(Curved { ginsu: false, fields }),
    })
}

/// The base gain of the mixer channel `channel` (1 when the bundle does not list it).
fn channel_gain(graphs: &SoundGraphs, channel: u64) -> f32 {
    graphs.channel_gains.get(&channel).copied().unwrap_or(1.0)
}

/// The window of `item` as a curve of the domain's input, as TOML.
fn window_curve(item: &Item, sequence: &Sequence, domain: Domain) -> Option<String> {
    let (floor, ceiling) = match domain {
        Domain::Rpm { lo, hi } => (lo, hi),
        Domain::Speed => (f32::MIN, f32::MAX),
    };
    let edge = match domain {
        Domain::Rpm { lo, hi } => (hi - lo) * 1e-4,
        Domain::Speed => 0.01,
    };
    let mut values = vec![item.lo, item.hi, item.lo - edge, item.hi + edge];
    for step in 1..=FADE_STEPS {
        let t = step as f32 / FADE_STEPS as f32;
        values.push(item.lo + item.fade_in.span * t);
        values.push(item.hi - item.fade_out.span * t);
    }
    let points = values
        .into_iter()
        .filter(|value| (floor..=ceiling).contains(value) || matches!(domain, Domain::Speed))
        .map(|value| (domain.x(sequence_value(sequence, domain, value)), item.window(value)))
        .collect::<Vec<_>>();
    Some(curve_toml(domain.input(), &normalise(points)?))
}

/// The live value a window is read at: the audible engine speed is clamped into its sequence's range.
fn sequence_value(sequence: &Sequence, domain: Domain, value: f32) -> f32 {
    match domain {
        Domain::Rpm { .. } => sequence.clamp(value),
        Domain::Speed => value,
    }
}

/// The curve of the engine load that the mixer item of `channel` gives, as TOML.
fn load_curve(graphs: &SoundGraphs, channel: u64) -> Option<String> {
    let (_, item) = graphs.mixer_items(ENGINE_LOAD, channel).next()?;
    let modulator: &Modulator = item.gains.first()?;
    let points = (0..LOAD_SAMPLES)
        .map(|step| {
            let x = step as f32 / (LOAD_SAMPLES - 1) as f32;
            (x, modulator.eval(x))
        })
        .collect::<Vec<_>>();
    Some(curve_toml("load", &normalise(points)?))
}

/// Sorted points with rising x, y within the manifest's range and rounded; `None` when fewer than two remain.
fn normalise(mut points: Vec<(f32, f32)>) -> Option<Vec<(f32, f32)>> {
    points.retain(|point| point.0.is_finite() && point.1.is_finite());
    points.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mut kept: Vec<(f32, f32)> = Vec::new();
    for (x, y) in points {
        let point = (round(x), round(y.clamp(0.0, MAX_LEVEL)));
        if kept.last().is_some_and(|last| last.0 >= point.0) {
            continue;
        }
        kept.push(point);
    }
    (kept.len() >= 2).then_some(kept)
}

fn round(value: f32) -> f32 {
    (value * 1e4).round() / 1e4
}

fn curve_toml(input: &str, points: &[(f32, f32)]) -> String {
    let list: Vec<String> = points.iter().map(|(x, y)| format!("[{x:?}, {y:?}]")).collect();
    format!("{{ input = \"{input}\", points = [{}] }}", list.join(", "))
}

#[cfg(test)]
mod real_tests;
#[cfg(test)]
mod tests;
