//! From the two REV models of a bike (engine and exhaust) to the engine layers of its manifest.
//!
//! The host mixes up to twelve looping layers by engine speed (`docs/modding/car-sound.md`). A REV model holds twenty
//! loops recorded under load, each at a known engine speed (`rev.rs`), and an idle recording. The layers are:
//!
//! - **the idle**: the idle recording of the engine model plus that of the exhaust model, at the pitch measured on it;
//! - **loops under load**: [`LOOP_LAYERS`] of the twenty loops of the engine model, spread evenly over its list (first and
//!   last included), each summed with the exhaust model's loop of the same number of engine cycles (the nearest, when it
//!   has none), at the engine speed `120 * cycles per second` of the loop.
//!
//! The loops *off load* and the two sweeps are not used: the host's mixer has no crossfade by load for layers mixed by
//! speed, and the layer limit leaves no room for a second set (`docs/formats/ride6-audio.md`).

use anyhow::{Context, Result, ensure};

use super::pitch;
use super::rev::{RPM_PER_CYCLE_HZ, RevModel, RevStream};
use super::wav::{self, Pcm};

/// Loops under load taken from a model: with the idle they fill the host's twelve layers.
pub const LOOP_LAYERS: usize = 11;
/// The range the pitch of the idle recording is looked for in, engine cycles per second.
const IDLE_SEARCH_HZ: (f32, f32) = (6.0, 60.0);
/// The largest difference in whole cycles, as a fraction, between an engine loop and the exhaust loop summed into it: one
/// cycle of thirteen, about a semitone and a half. Beyond it the exhaust is left out of that layer (the two models do not
/// cover the same pitches there).
pub const MAX_DETUNE: f32 = 0.08;
/// Crossfade that closes the idle loop, milliseconds. The loops under load hold whole cycles and need none.
const IDLE_FADE_MS: f32 = 40.0;

/// Where the decoded recordings of one model come from.
pub trait Recordings {
    /// The `number`-th recording of the model (the same numbering as [`RevStream::number`]), mono.
    fn decode(&self, number: usize) -> Result<Pcm>;
}

/// One engine layer: the engine speed it plays at and its loop.
#[derive(Debug, Clone, PartialEq)]
pub struct EngineLayer {
    pub rpm: f32,
    pub pcm: Pcm,
}

/// The layers and what was checked while making them.
#[derive(Debug, Clone, PartialEq)]
pub struct EngineLayers {
    /// Slowest first, all at one sample rate, levels relative to each other as recorded.
    pub layers: Vec<EngineLayer>,
    /// The largest pitch difference between a summed engine and exhaust loop, as a fraction.
    pub worst_detune: f32,
    pub notes: Vec<String>,
}

/// `count` indices spread evenly over `0..len`, the first and the last included, without repeats.
pub fn spread(len: usize, count: usize) -> Vec<usize> {
    if len == 0 || count == 0 {
        return Vec::new();
    }
    if count == 1 || len == 1 {
        return vec![0];
    }
    let mut out: Vec<usize> =
        (0..count.min(len)).map(|n| (n * (len - 1) * 2 + (count.min(len) - 1)) / ((count.min(len) - 1) * 2)).collect();
    out.dedup();
    out
}

fn mono(pcm: Pcm) -> Result<Pcm> {
    ensure!(pcm.channels == 1, "a REV recording should be mono, this has {} channels", pcm.channels);
    Ok(pcm)
}

/// The exhaust loop that suits `target`: the same number of cycles, else the nearest number.
fn partner<'a>(target: &RevStream, loops: &'a [RevStream]) -> Option<&'a RevStream> {
    loops.iter().min_by_key(|stream| (i64::from(stream.cycles) - i64::from(target.cycles)).unsigned_abs())
}

/// Sum an exhaust loop into an engine loop: the exhaust is resampled (as a periodic loop) to the engine's length, so the
/// layer is one seamless loop; the pitch of the exhaust part becomes `its cycles / the layer's length`.
fn sum_into(engine: &Pcm, exhaust: &Pcm) -> Result<Pcm> {
    ensure!(engine.sample_rate == exhaust.sample_rate, "the engine and exhaust recordings differ in sample rate");
    let fitted = pitch::resample_periodic(&exhaust.samples, engine.samples.len());
    let samples = engine.samples.iter().zip(&fitted).map(|(a, b)| a + b).collect();
    Ok(Pcm { sample_rate: engine.sample_rate, channels: 1, samples, loop_range: None })
}

/// The idle recordings summed: the exhaust's is resampled to the engine's length, unless their lengths differ by more than
/// [`MAX_DETUNE`] (the idles are then not at one pitch, and the engine's plays alone).
fn idle_sum(engine: &Pcm, exhaust: &Pcm) -> Result<Pcm> {
    let (e, x) = (engine.frames() as f32, exhaust.frames() as f32);
    if (e - x).abs() / e.max(1.0) > MAX_DETUNE {
        return Ok(engine.clone());
    }
    sum_into(engine, exhaust)
}

/// Make the layers of one bike. `loops` is how many loops under load to take (at most [`LOOP_LAYERS`]).
pub fn build(
    engine: &RevModel,
    engine_audio: &dyn Recordings,
    exhaust: &RevModel,
    exhaust_audio: &dyn Recordings,
    loops: usize,
    idle_fallback_rpm: Option<f32>,
) -> Result<EngineLayers> {
    ensure!((1..=LOOP_LAYERS).contains(&loops), "1 to {LOOP_LAYERS} loops, not {loops}");
    let mut layers = Vec::new();
    let mut worst = 0.0_f32;
    let mut notes = Vec::new();
    let mut engine_only = 0;
    for position in spread(engine.on_load.len(), loops) {
        let stream = &engine.on_load[position];
        let partner = partner(stream, &exhaust.on_load).context("the exhaust model has no loops under load")?;
        let (a, b) = (mono(engine_audio.decode(stream.number)?)?, mono(exhaust_audio.decode(partner.number)?)?);
        let detune = (partner.cycles as f32 - stream.cycles as f32).abs() / stream.cycles as f32;
        if detune > MAX_DETUNE {
            engine_only += 1;
            layers.push(EngineLayer { rpm: stream.rpm(), pcm: a });
            continue;
        }
        worst = worst.max(detune);
        layers.push(EngineLayer { rpm: stream.rpm(), pcm: sum_into(&a, &b)? });
    }
    if engine_only > 0 {
        notes.push(format!(
            "{engine_only} loops without the exhaust: its pitch is more than {:.0}% away",
            MAX_DETUNE * 100.0
        ));
    }
    let (a, b) = (mono(engine_audio.decode(engine.idle.number)?)?, mono(exhaust_audio.decode(exhaust.idle.number)?)?);
    match pitch::cycle_frequency(&a.samples, a.sample_rate, IDLE_SEARCH_HZ.0, IDLE_SEARCH_HZ.1) {
        Some(hz) => {
            let fade = (a.sample_rate as f32 * IDLE_FADE_MS / 1000.0) as usize;
            layers.push(EngineLayer { rpm: hz * RPM_PER_CYCLE_HZ, pcm: idle_sum(&a, &b)?.seamless_loop(fade) });
        }
        None => {
            // An uneven idle (a V engine, a two-stroke) has no clear period. The recording is then played as the idle
            // sound of the engine at the idle speed of its physics data, when that is below the lowest loop.
            let lowest = layers.iter().map(|layer| layer.rpm).fold(f32::MAX, f32::min);
            match idle_fallback_rpm.filter(|rpm| *rpm < lowest && *rpm >= 100.0) {
                Some(rpm) => {
                    let fade = (a.sample_rate as f32 * IDLE_FADE_MS / 1000.0) as usize;
                    layers.push(EngineLayer { rpm, pcm: idle_sum(&a, &b)?.seamless_loop(fade) });
                    notes.push("the idle recording has no clear period: it plays at the physics idle speed".to_owned());
                }
                None => notes.push("the idle recording has no measurable pitch: no idle layer".to_owned()),
            }
        }
    }
    layers.sort_by(|x, y| x.rpm.total_cmp(&y.rpm));
    ensure!(layers.windows(2).all(|pair| pair[0].rpm < pair[1].rpm), "two layers have the same engine speed");
    Ok(EngineLayers { layers, worst_detune: worst, notes })
}

/// Scale every layer by one gain so the loudest peaks at `peak` (0 to 1); the layers keep their levels relative to each other.
pub fn normalise(layers: &mut EngineLayers, peak: f32) -> Result<()> {
    let loudest = layers.layers.iter().map(|layer| layer.pcm.peak()).fold(0.0_f32, f32::max);
    ensure!(loudest > 1e-6, "every engine layer is silent");
    let gain = peak / loudest;
    for layer in &mut layers.layers {
        layer.pcm.samples.iter_mut().for_each(|sample| *sample *= gain);
    }
    Ok(())
}

/// Encode a layer for the manifest: 16-bit PCM WAV.
pub fn encode(layer: &EngineLayer) -> Vec<u8> {
    wav::encode16(&layer.pcm)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sound::rev;
    use crate::sound::rev::tests::{model, usual};

    /// Recordings that are a sine of a pitch chosen by the stream number, at 48 kHz.
    struct Tones;

    impl Recordings for Tones {
        fn decode(&self, number: usize) -> Result<Pcm> {
            // Streams 1 to 3 and 5 to 7 are loops (30, 60, 90 Hz and 35, 55, 80 Hz), 9 is the idle (20 Hz, 1.5 s).
            let (hz, frames) = match number {
                1 => (30.0, 28_800),
                2 => (60.0, 28_800),
                3 => (90.0, 28_800),
                5 => (35.0, 28_800),
                6 => (55.0, 28_800),
                7 => (80.0, 28_800),
                9 => (20.0, 72_000),
                _ => (10.0, 1_000),
            };
            let samples = (0..frames)
                .map(|i| (std::f32::consts::TAU * hz * i as f32 / 48_000.0).sin() * 0.25 * (1.0 + number as f32 / 20.0))
                .collect();
            Ok(Pcm { sample_rate: 48_000, channels: 1, samples, loop_range: None })
        }
    }

    #[test]
    fn indices_spread_over_a_list_with_both_ends() {
        assert_eq!(spread(20, 11), [0, 2, 4, 6, 8, 10, 11, 13, 15, 17, 19]);
        assert_eq!(spread(3, 3), [0, 1, 2]);
        assert_eq!(spread(3, 8), [0, 1, 2]);
        assert_eq!(spread(5, 1), [0]);
        assert!(spread(0, 4).is_empty());
        assert_eq!(spread(19, 11).first(), Some(&0));
        assert_eq!(spread(19, 11).last(), Some(&18));
    }

    #[test]
    fn layers_are_the_loops_under_load_and_the_idle_at_their_engine_speeds() {
        let parsed = rev::parse(&model(&usual())).unwrap();
        let mut layers = build(&parsed, &Tones, &parsed, &Tones, 3, None).unwrap();
        // The idle (20 Hz, 2400 rpm) comes first, then 30, 60 and 90 Hz of the loops: 3600, 7200 and 10800 rpm.
        let rpms: Vec<f32> = layers.layers.iter().map(|layer| layer.rpm).collect();
        assert_eq!(rpms.len(), 4);
        assert!((rpms[0] - 2400.0).abs() < 60.0, "{rpms:?}");
        assert!((rpms[1] - 3600.0).abs() < 0.1 && (rpms[2] - 7200.0).abs() < 0.1 && (rpms[3] - 10800.0).abs() < 0.1);
        assert_eq!(layers.worst_detune, 0.0);
        // Engine and exhaust of one number of cycles are summed: twice one tone.
        let loop_frames = layers.layers[1].pcm.frames();
        assert_eq!(loop_frames, 28_800);
        normalise(&mut layers, 0.9).unwrap();
        let peak = layers.layers.iter().map(|layer| layer.pcm.peak()).fold(0.0_f32, f32::max);
        assert!((peak - 0.9).abs() < 1e-3);
        assert_eq!(encode(&layers.layers[1])[..4], *b"RIFF");
    }

    #[test]
    fn an_exhaust_with_other_cycle_counts_is_matched_to_the_nearest_and_the_detune_is_reported() {
        let engine = rev::parse(&model(&usual())).unwrap();
        let mut other = usual();
        other[0].1 = 17;
        other[1].1 = 40;
        let exhaust = rev::parse(&model(&other)).unwrap();
        let layers = build(&engine, &Tones, &exhaust, &Tones, 3, None).unwrap();
        // 18 against 17 cycles (5.6%) is summed; 36 against 40 (11%) is left out, and so is 54 against 54's neighbour none.
        assert!(layers.worst_detune > 0.05 && layers.worst_detune <= MAX_DETUNE, "{}", layers.worst_detune);
        assert!(layers.notes.iter().any(|note| note.contains("without the exhaust")));
    }

    /// Like [`Tones`], but the idle (stream 9) is noise without a period.
    struct NoisyIdle;

    impl Recordings for NoisyIdle {
        fn decode(&self, number: usize) -> Result<Pcm> {
            if number != 9 {
                return Tones.decode(number);
            }
            let mut seed = 7_u32;
            let samples = (0..72_000)
                .map(|_| {
                    seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                    ((seed >> 8) as f32 / 8_388_608.0 - 1.0) * 0.2
                })
                .collect();
            Ok(Pcm { sample_rate: 48_000, channels: 1, samples, loop_range: None })
        }
    }

    #[test]
    fn an_idle_without_a_period_plays_at_the_physics_idle_or_is_left_out() {
        let parsed = rev::parse(&model(&usual())).unwrap();
        let with = build(&parsed, &NoisyIdle, &parsed, &NoisyIdle, 3, Some(1500.0)).unwrap();
        assert_eq!(with.layers.len(), 4);
        assert_eq!(with.layers[0].rpm, 1500.0);
        assert!(with.notes.iter().any(|note| note.contains("physics idle")));
        let too_high = build(&parsed, &NoisyIdle, &parsed, &NoisyIdle, 3, Some(5000.0)).unwrap();
        assert_eq!(too_high.layers.len(), 3);
        let without = build(&parsed, &NoisyIdle, &parsed, &NoisyIdle, 3, None).unwrap();
        assert_eq!(without.layers.len(), 3);
        assert!(without.notes[0].contains("no idle layer"));
    }

    #[test]
    fn silent_input_and_bad_counts_are_refused() {
        let parsed = rev::parse(&model(&usual())).unwrap();
        assert!(build(&parsed, &Tones, &parsed, &Tones, 0, None).is_err());
        assert!(build(&parsed, &Tones, &parsed, &Tones, LOOP_LAYERS + 1, None).is_err());
        let mut silent = EngineLayers {
            layers: vec![EngineLayer {
                rpm: 3000.0,
                pcm: Pcm { sample_rate: 48_000, channels: 1, samples: vec![0.0; 100], loop_range: None },
            }],
            worst_detune: 0.0,
            notes: Vec::new(),
        };
        assert!(normalise(&mut silent, 0.9).is_err());
    }
}
