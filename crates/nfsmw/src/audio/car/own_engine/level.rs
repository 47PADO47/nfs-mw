//! The loudness of a mod car's engine, made the same for every car.
//!
//! The recordings of two cars are mastered at different levels, and the graphs of the 2012 game give each voice a gain of
//! its own, so the engines of the converted cars differed by 40 dB at idle and 15 dB at the limiter (measured, see
//! `docs/modding/car-sound.md`). The cars of the 2005 game are all within a few dB of one another. This predicts the
//! level of the whole engine from the recordings and the layers' gains, and scales the engine to a fixed level at every
//! engine speed (or, for a car whose engine is only speed-driven loops, every car speed).
//!
//! The prediction is incoherent: the power of the layers adds, each layer at its gain times the root mean square of its
//! recording (a Ginsu recording at the stretch of it that its frequency plays).

use blackbox_ginsu::GinsuData;
use ea_audio::Pcm;
use nfsmw_data::car::mods::{CurveInput, EngineSound};

use super::mixing::{EngineInputs, mix_inputs};

/// The level the engine is brought to at the nominal engine volume, in dB re full scale (the stock cars measure -17 to
/// -22 dB over a sweep).
pub const TARGET_DB: f32 = -19.0;
/// The engine volume of the dynamic mixer at which the target holds (what the mixer gives a car on the road).
pub const NOMINAL_ENGINE_VOLUME: f32 = 0.415;
/// The measured loudness of the rendered engine over the predicted one, as a factor: the Ginsu synthesiser's grain
/// crossfades, the pan and the mono mix-down of the render are not in the prediction.
pub const RENDER_FACTOR: f32 = 1.0;
/// No correction is more than this many times up or down.
const LIMIT: f32 = 40.0;
/// Frequencies at which a Ginsu recording's level is read.
const FREQUENCIES: usize = 33;
/// Half of the stretch of a Ginsu recording, in seconds, that its level at a frequency is read over.
const STRETCH: f32 = 0.25;

/// The level of one layer's recording.
#[derive(Debug, Clone, PartialEq)]
pub enum Recording {
    /// A loop: its root mean square.
    Loop(f32),
    /// A Ginsu recording: its root mean square at evenly spaced frequencies from `low` to `high`.
    Ginsu { low: f32, high: f32, rms: Vec<f32> },
}

impl Recording {
    /// The level of a loop.
    pub fn of_loop(pcm: &Pcm) -> Self {
        let total: f64 = pcm.samples.iter().map(|s| (f64::from(*s) / 32768.0).powi(2)).sum();
        Self::Loop((total / pcm.samples.len().max(1) as f64).sqrt() as f32)
    }

    /// The level of a Ginsu recording at each frequency of its range.
    pub fn of_ginsu(data: &GinsuData) -> Self {
        let tables = data.tables();
        let (low, high) = (tables.min_frequency(), tables.max_frequency());
        let half = (STRETCH * tables.sample_rate() as f32) as i64;
        let rms = (0..FREQUENCIES)
            .map(|step| {
                let frequency = low + (high - low) * step as f32 / (FREQUENCIES - 1) as f32;
                let centre = i64::from(tables.frequency_to_sample(frequency));
                let pcm = data.pcm();
                let (from, to) = ((centre - half).max(0) as usize, ((centre + half) as usize).min(pcm.len()));
                let part = pcm.get(from..to).unwrap_or(&[]);
                (part.iter().map(|s| f64::from(*s).powi(2)).sum::<f64>() / part.len().max(1) as f64).sqrt() as f32
            })
            .collect();
        Self::Ginsu { low, high, rms }
    }

    /// The root mean square when the layer plays at `frequency` (ignored by a loop).
    fn rms_at(&self, frequency: f32) -> f32 {
        match self {
            Self::Loop(rms) => *rms,
            Self::Ginsu { low, high, rms } => {
                let t = ((frequency - low) / (high - low).max(1e-3)).clamp(0.0, 1.0) * (rms.len() - 1) as f32;
                let at = (t as usize).min(rms.len() - 2);
                rms[at] + (rms[at + 1] - rms[at]) * (t - at as f32)
            }
        }
    }
}

/// What the table is read by.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum By {
    Rpm,
    Speed,
}

/// The factor the engine is scaled by so that it is as loud as the cars of the game, from the recordings and the gains
/// the layers have at this engine speed, load and car speed.
#[derive(Debug, Clone, PartialEq)]
pub struct Normaliser {
    by: By,
    sound: EngineSound,
    recordings: Vec<Recording>,
    /// The typical factor at each load (the median over the engine speeds, or the one factor of a speed-driven engine).
    typical: [f32; LOADS],
}

/// The speed the car is taken to be at when only the engine speed is known, in mph: a plain curve from a crawl to 110.
fn nominal_mph(rpm: f32) -> f32 {
    25.0 + 85.0 * rpm
}

/// The highest speed of the table of a car whose engine is only speed-driven, in mph.
const SPEED_RANGE_MPH: f32 = 130.0;
/// Rows of the table: loads 0, 0.25, 0.5, 0.75 and 1.
const LOADS: usize = 5;
/// The level at no load, in dB re full scale: the cars of the game are a few dB quieter off the throttle.
const TARGET_DB_NO_LOAD: f32 = -22.0;
/// The factor at an engine speed is within this many times of the typical one: a car that idles much quieter than it runs
/// (the stock Cobalt does, by 15 dB) keeps its quiet idle instead of having it pulled up to the level of a pull.
const SPREAD: f32 = 4.0;
/// Engine speeds the typical factor is taken over.
const GRID: usize = 41;

fn load_of(row: usize) -> f32 {
    row as f32 / (LOADS - 1) as f32
}

/// The factor that brings `power` to the target of `load`.
fn factor(power: f32, load: f32) -> f32 {
    let target_db = TARGET_DB_NO_LOAD + (TARGET_DB - TARGET_DB_NO_LOAD) * load.clamp(0.0, 1.0);
    10f32.powf(target_db / 20.0) / (NOMINAL_ENGINE_VOLUME * RENDER_FACTOR * power.sqrt())
}

impl Normaliser {
    /// The normaliser of `sound` whose layers' recordings are `recordings` (one each, in order).
    pub fn new(sound: &EngineSound, recordings: &[Recording]) -> Self {
        let uses_rpm = sound.layers.iter().any(|layer| layer.gain.is_empty() || layer.span.is_some())
            || sound.layers.iter().flat_map(|layer| &layer.gain).any(|curve| curve.input == CurveInput::Rpm);
        let by = if uses_rpm { By::Rpm } else { By::Speed };
        let mut typical = [1.0f32; LOADS];
        for (row, typical) in typical.iter_mut().enumerate() {
            let load = load_of(row);
            let powers: Vec<f32> = (0..GRID)
                .map(|at| {
                    let x = at as f32 / (GRID - 1) as f32;
                    let inputs = match by {
                        By::Rpm => EngineInputs { rpm: x, load, speed_mph: nominal_mph(x) },
                        By::Speed => EngineInputs { rpm: 0.5, load, speed_mph: x * SPEED_RANGE_MPH },
                    };
                    predicted_power(sound, recordings, &inputs)
                })
                .collect();
            *typical = match by {
                // A speed-driven engine fades its loops in and out by speed: one factor, from its loudest point, so that
                // the fade-in of a loop is not undone by a correction that follows it.
                By::Speed => factor(powers.iter().copied().fold(1e-12, f32::max), load).clamp(1.0 / LIMIT, LIMIT),
                By::Rpm => {
                    let mut factors: Vec<f32> =
                        powers.iter().filter(|power| **power > 1e-12).map(|power| factor(*power, load)).collect();
                    factors.sort_by(f32::total_cmp);
                    factors.get(factors.len() / 2).copied().unwrap_or(1.0).clamp(1.0 / LIMIT, LIMIT)
                }
            };
        }
        Self { by, sound: sound.clone(), recordings: recordings.to_vec(), typical }
    }

    /// The factor for `inputs`.
    pub fn scale(&self, inputs: &EngineInputs) -> f32 {
        let l = inputs.load.clamp(0.0, 1.0) * (LOADS - 1) as f32;
        let below = (l as usize).min(LOADS - 2);
        let typical = self.typical[below] + (self.typical[below + 1] - self.typical[below]) * (l - below as f32);
        if self.by == By::Speed {
            return typical;
        }
        let power = predicted_power(&self.sound, &self.recordings, inputs);
        if power <= 1e-12 {
            return typical;
        }
        factor(power, inputs.load).clamp(typical / SPREAD, typical * SPREAD)
    }
}

/// The predicted power of the engine at `inputs`: the layers' gains times their recordings' levels, added as powers.
pub fn predicted_power(sound: &EngineSound, recordings: &[Recording], inputs: &EngineInputs) -> f32 {
    mix_inputs(sound, inputs)
        .iter()
        .zip(recordings)
        .map(|(layer, recording)| (layer.gain * recording.rms_at(layer.frequency)).powi(2))
        .sum()
}

#[cfg(test)]
mod tests {
    use nfsmw_data::car::mods::{EngineLayer, LayerCurve};

    use super::*;

    fn curved(file: &str, curve: LayerCurve) -> EngineLayer {
        EngineLayer { gain: vec![curve], ..EngineLayer::classic(file, 0.0) }
    }

    fn rpm_flat(level: f32) -> LayerCurve {
        LayerCurve { input: CurveInput::Rpm, points: vec![(0.0, level), (1.0, level)] }
    }

    #[test]
    fn a_louder_recording_gets_a_smaller_factor_and_the_result_is_one_level() {
        let sound = |volume: f32| EngineSound {
            idle_rpm: 800.0,
            redline_rpm: 7000.0,
            layers: vec![EngineLayer { volume, ..curved("a", rpm_flat(1.0)) }],
        };
        let at = EngineInputs { rpm: 0.5, load: 1.0, speed_mph: 60.0 };
        let (quiet, loud) = (Recording::Loop(0.05), Recording::Loop(0.5));
        let a = Normaliser::new(&sound(1.0), &[quiet.clone()]).scale(&at);
        let b = Normaliser::new(&sound(1.0), &[loud]).scale(&at);
        assert!((a / b - 10.0).abs() < 1e-3, "{a} {b}");
        let result = |volume: f32| {
            let engine = sound(volume);
            let table = Normaliser::new(&engine, &[quiet.clone()]);
            table.scale(&at) * predicted_power(&engine, &[quiet.clone()], &at).sqrt() * NOMINAL_ENGINE_VOLUME
        };
        assert!((result(1.0) - result(0.5)).abs() < 1e-4, "the volume of the graph does not change the level");
        assert!((20.0 * result(1.0).log10() - TARGET_DB).abs() < 0.1);
    }

    #[test]
    fn the_correction_follows_the_engine_speed_within_its_spread_and_stops_at_the_limit() {
        let ramp = LayerCurve { input: CurveInput::Rpm, points: vec![(0.0, 0.1), (1.0, 1.0)] };
        let engine = EngineSound { idle_rpm: 800.0, redline_rpm: 7000.0, layers: vec![curved("a", ramp)] };
        let table = Normaliser::new(&engine, &[Recording::Loop(0.5)]);
        let (idle, top) = (
            table.scale(&EngineInputs { rpm: 0.0, load: 1.0, speed_mph: 0.0 }),
            table.scale(&EngineInputs { rpm: 1.0, load: 1.0, speed_mph: 0.0 }),
        );
        assert!(
            idle > top * 5.0 && idle <= top * 10.0,
            "a layer ten times quieter at idle is raised, but only by the spread: {idle} {top}"
        );
        let silent = Normaliser::new(&engine, &[Recording::Loop(1e-6)]);
        assert!(silent.scale(&EngineInputs { rpm: 0.5, load: 1.0, speed_mph: 0.0 }) <= LIMIT);
    }

    #[test]
    fn an_engine_of_speed_driven_loops_has_one_factor_from_its_loudest_point() {
        let window = LayerCurve { input: CurveInput::SpeedMph, points: vec![(20.0, 0.0), (40.0, 1.0), (90.0, 1.0)] };
        let engine = EngineSound { idle_rpm: 800.0, redline_rpm: 7000.0, layers: vec![curved("w", window)] };
        let table = Normaliser::new(&engine, &[Recording::Loop(0.05)]);
        assert_eq!(table.by, By::Speed);
        let standing = table.scale(&EngineInputs { rpm: 0.0, load: 0.0, speed_mph: 0.0 });
        let cruising = table.scale(&EngineInputs { rpm: 0.0, load: 0.0, speed_mph: 60.0 });
        assert!(
            standing > 0.0 && (standing - cruising).abs() < 1e-6,
            "one factor over the speeds: {standing} {cruising}"
        );
    }
}
