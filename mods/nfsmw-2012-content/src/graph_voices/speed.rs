//! The layers a 2012 car's speed drives (docs/formats/chameleon-sound-graph.md, "The transmission whine" and "The
//! electric motor"): the loops of the sequences driven by `GetTransmissionSpeedMph`.
//!
//! Two kinds of sequence read that value:
//!
//! - the **gearbox whine** of the car's transmission-whine object: five loops in windows of the speed, each played at the
//!   speed over the speed it was recorded at, and two sets of them that the gas pedal crossfades (the one set on the
//!   throttle, the other off it). It is a detail under the engine, so it is scaled down by [`WHINE_BALANCE`];
//! - the **motor** of the player's engine sound (an electric or hybrid car; every other car has none): loops whose pitch is
//!   a curve of the speed, over the item's window, and that a mixer item of their own sequence may swell and fade by the
//!   speed. For the electric car it is the whole sound; it plays at the level the graph gives it.
//!
//! A bundle without the objects that gather its sequences keeps the older reading: every speed-driven sequence of more
//! than one item is a whine.

use chameleon_sound_graph::{Item, ItemKind, Sequence, SoundGraphs, WaveItem};

use super::{Domain, WHINE_SPEED, channel_gain, curve_toml, driver_curve, normalise, round, window_curve};
use crate::engine::{Curved, Layer};
use crate::sample::{Sample, wav_bytes};

/// The gas pedal's value in the graphs.
const GAS: &str = "GetPlayerGas";
/// The class of the object that holds the gearbox whine.
const WHINE_OBJECT: &str = "TransmissionWhine";
/// Samples taken along a pitch curve or a mixer curve over a window.
const SAMPLES: usize = 17;
/// The level of the gearbox whine loops next to the rest of the engine, a factor on the gain the graph gives them. By the
/// graph alone (a gain of 0.6 on a channel at -16 dB) they sound 6 dB under the whole engine at a spectral centroid of 1 to
/// 3 kHz, which made the converted cars much brighter than the cars of the game (centroid at the limiter 745 Hz against 470
/// to 580 Hz for the stock cars, measured); at this factor the whine is a detail 15 dB under the engine.
pub(super) const WHINE_BALANCE: f32 = 0.35;

/// What a speed-driven sequence is in the car.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Role {
    Whine,
    Motor,
}

impl Role {
    fn balance(self) -> f32 {
        match self {
            Self::Whine => WHINE_BALANCE,
            Self::Motor => 1.0,
        }
    }
}

/// The layers of every speed-driven sequence of the car, the motor first.
pub(super) fn layers(graphs: &SoundGraphs, samples: &[Sample]) -> Vec<Layer> {
    let mut out = Vec::new();
    for (role, sequence) in sequences(graphs) {
        for item in &sequence.items {
            let ItemKind::Wave(wave) = &item.kind else { continue };
            out.extend(layer(graphs, samples, sequence, item, wave, role));
        }
    }
    out
}

/// The speed-driven sequences of the car with what each is: the motor of the player's engine sound, then the whine.
fn sequences(graphs: &SoundGraphs) -> Vec<(Role, &Sequence)> {
    let (whine, player) = (graphs.group_of(WHINE_OBJECT), graphs.player_engine());
    if whine.is_none() && player.is_none() {
        return graphs
            .driven_by(WHINE_SPEED)
            .filter(|sequence| sequence.items.len() > 1)
            .map(|sequence| (Role::Whine, sequence))
            .collect();
    }
    let speed = |sequence: &&Sequence| sequence.driven_by(WHINE_SPEED);
    let motors = player.into_iter().flat_map(|group| graphs.sequences_in(group)).filter(speed);
    let whines = whine.into_iter().flat_map(|group| graphs.sequences_in(group)).filter(speed);
    motors.map(|sequence| (Role::Motor, sequence)).chain(whines.map(|sequence| (Role::Whine, sequence))).collect()
}

fn layer(
    graphs: &SoundGraphs,
    samples: &[Sample],
    sequence: &Sequence,
    item: &Item,
    wave: &WaveItem,
    role: Role,
) -> Option<Layer> {
    let sample = samples.iter().find(|sample| wave.waves.first() == Some(&sample.id) && sample.looped)?;
    let mut gain = vec![window_curve(item, sequence, Domain::Speed)?];
    gain.extend(channel_curve(sequence, wave.mixer));
    if role == Role::Whine {
        gain.extend(driver_curve(graphs, GAS, wave.mixer));
    }
    let level = if wave.gain.follows_input() { 1.0 } else { wave.gain.base };
    let volume = round(level * channel_gain(graphs, wave.mixer) * role.balance());
    let fields = format!("volume = {volume:?}, gain = [{}]{}", gain.join(", "), pitch_fields(item, wave));
    Some(Layer {
        rpm: 0.0,
        wav: wav_bytes(&sample.samples, sample.rate, 1.0),
        curves: Some(Curved { ginsu: false, fields }),
    })
}

/// How the loop's playback rate follows the speed, as manifest fields (empty when it plays at its own speed): the speed
/// over the speed it was recorded at, for a whine loop; for a motor loop the curve of its pitch modulator, which is read
/// over the item's window (0 at the window's low end, 1 at its high end).
fn pitch_fields(item: &Item, wave: &WaveItem) -> String {
    if wave.tracks_driver && wave.pitch.base > 0.0 {
        return format!(", pitch = {{ input = \"speed_mph\", reference = {:?} }}", wave.pitch.base);
    }
    let own_curve = wave.pitch.expression.is_empty() && !wave.pitch.curve.is_empty();
    if !own_curve {
        return String::new();
    }
    let Some(points) = across_window(item, |t| wave.pitch.eval(t)) else { return String::new() };
    format!(", rate = {}", curve_toml("speed_mph", &points))
}

/// The gain curve of the speed that the mixer item of `channel` in `sequence` gives, over that item's window, as TOML.
/// `None` when the sequence has no such item or its value is read from a named live value.
fn channel_curve(sequence: &Sequence, channel: u64) -> Option<String> {
    if channel == 0 {
        return None;
    }
    let (item, mixer) = sequence.items.iter().find_map(|item| match &item.kind {
        ItemKind::Mixer(mixer) if mixer.channel == channel => Some((item, mixer)),
        _ => None,
    })?;
    let gain = mixer.gains.first().filter(|gain| gain.expression.is_empty())?;
    let points = across_window(item, |t| gain.eval(t))?;
    Some(curve_toml("speed_mph", &points))
}

/// `value` sampled along the window of `item`, as points `(speed, value)`: `t` runs from 0 at the item's low end to 1 at its
/// high end, the way a curve with no expression of its own is read. `None` for an empty window or fewer than two points.
fn across_window(item: &Item, value: impl Fn(f32) -> f32) -> Option<Vec<(f32, f32)>> {
    if item.hi <= item.lo {
        return None;
    }
    let points = (0..SAMPLES)
        .map(|step| {
            let t = step as f32 / (SAMPLES - 1) as f32;
            (item.lo + (item.hi - item.lo) * t, value(t))
        })
        .collect();
    normalise(points)
}

#[cfg(test)]
mod tests;
