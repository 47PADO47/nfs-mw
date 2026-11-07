//! From the two REV models of a bike (engine and exhaust) to the engine layers of its manifest.
//!
//! The host mixes looping layers by engine speed (`docs/modding/car-sound.md`). A REV model holds twenty loops recorded under
//! load, about as many recorded off load, each at a known engine speed (`rev.rs`), and an idle recording. The layers are:
//!
//! - **the idle**: the idle recording of the engine model plus that of the exhaust model, at the pitch measured on it (looked
//!   for around the bike's idle speed) or, when it has no clear period, at the bike's idle speed; it is a layer under load and
//!   an identical one off load;
//! - **loops under load** and **loops off load**: a choice of the twenty loops of each run of the engine model, **dense below
//!   40 percent of the redline** (every loop there, up to half of the layers) and spread evenly above, each summed with the
//!   exhaust model's loop of the same number of engine cycles (the nearest, when it has none), at the engine speed of the loop.
//!   The host crossfades the two sets by the throttle (`docs/specs/ride6-engine-sound.md`, section 9).
//!
//! Three things keep the sound of the mix right (the same section): every layer gets **the same loudness** (root mean
//! square; the recorded loops differ by 20 dB between the low and the high rpm); the exhaust loop is **retimed to the exact
//! pitch of the engine loop** it is summed with (a loop one cycle off is 2 to 8 percent off in pitch, and the sum then held two
//! notes); and a model whose cycle is one turn of the engine (loops that come out at almost twice the redline) is read at 60
//! rpm per cycle per second instead of 120.

use anyhow::{Context, Result, ensure};

use super::pitch;
use super::rev::{RPM_PER_CYCLE_HZ, RevModel, RevStream};
use super::wav::{self, Pcm};

/// Loops under load, and loops off load, taken from a model at most; with the two idles they make the host's layers.
pub const LOOP_LAYERS: usize = 20;
/// Below this fraction of the redline the loops are taken densely (up to half of the loop layers).
pub const DENSE_BELOW: f32 = 0.4;
/// The idle recording's pitch is looked for between these fractions of the bike's idle speed.
const IDLE_BAND: (f32, f32) = (0.6, 1.6);
/// The highest engine speed the host accepts for a layer (`libs/blackbox-modding/src/content/engine_sound.rs`).
pub const MAX_LAYER_RPM: f32 = 20_000.0;
/// The largest difference in whole cycles, as a fraction, between an engine loop and the exhaust loop summed into it: one
/// cycle of thirteen, about a semitone and a half. The exhaust loop is retimed by it, so beyond it (a timbre moved by more
/// than that) the exhaust is left out of that layer (the two models do not cover the same pitches there).
pub const MAX_DETUNE: f32 = 0.08;
/// A model whose top loop, read at 120 rpm per cycle per second, is above this many times the bike's redline has a cycle of
/// one turn (the models of the Honda NS 400 R and the Yamaha RD 500 LC come out at 1.75 to 1.82).
const ONE_TURN_ABOVE: f32 = 1.5;
/// Crossfade that closes the idle loop, milliseconds. The loops under load hold whole cycles and need none.
const IDLE_FADE_MS: f32 = 40.0;

/// Where the decoded recordings of one model come from.
pub trait Recordings {
    /// The `number`-th recording of the model (the same numbering as [`RevStream::number`]), mono.
    fn decode(&self, number: usize) -> Result<Pcm>;
}

/// One engine layer: the engine speed it plays at, its loop, and whether it is played off load.
#[derive(Debug, Clone, PartialEq)]
pub struct EngineLayer {
    pub rpm: f32,
    pub pcm: Pcm,
    pub off_load: bool,
}

/// The layers and what was checked while making them.
#[derive(Debug, Clone, PartialEq)]
pub struct EngineLayers {
    /// Slowest first, all at one sample rate, levels relative to each other as recorded (until [`equalise`]).
    pub layers: Vec<EngineLayer>,
    /// The largest retiming of a summed exhaust loop, as a fraction of its cycles.
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

/// rpm per cycle per second of an engine model: 120 (a cycle of two turns), or 60 when its loops come out above
/// [`ONE_TURN_ABOVE`] times the redline at 120, which no engine does (the cycle of those models is one turn).
pub fn rpm_per_cycle_hz(engine: &RevModel, redline_rpm: f32) -> f32 {
    let top = engine.on_load.iter().map(RevStream::cycle_hz).fold(0.0_f32, f32::max);
    if top * RPM_PER_CYCLE_HZ > ONE_TURN_ABOVE * redline_rpm {
        return RPM_PER_CYCLE_HZ / 2.0;
    }
    RPM_PER_CYCLE_HZ
}

/// Sum an exhaust loop into an engine loop **at the engine loop's pitch**: the exhaust of `exhaust_cycles` cycles is read for
/// the engine's `engine_cycles` over the engine's length (`pitch::retime_periodic`), so every partial of the exhaust is on
/// a partial of the engine, whatever the cycle counts. (Resampling the exhaust to the engine's length instead leaves it at
/// `exhaust_cycles` over that length: a note of its own, up to a semitone and a half from the engine's.)
fn sum_matched(engine: &Pcm, engine_cycles: u32, exhaust: &Pcm, exhaust_cycles: u32) -> Result<Pcm> {
    ensure!(engine.sample_rate == exhaust.sample_rate, "the engine and exhaust recordings differ in sample rate");
    let fitted = pitch::retime_periodic(&exhaust.samples, exhaust_cycles, engine_cycles, engine.samples.len());
    let samples = engine.samples.iter().zip(&fitted).map(|(a, b)| a + b).collect();
    Ok(Pcm { sample_rate: engine.sample_rate, channels: 1, samples, loop_range: None })
}

/// The idle recordings summed: the exhaust's is resampled to the engine's length, unless their lengths differ by more than
/// [`MAX_DETUNE`] (the idles are then not at one pitch, and the engine's plays alone). An idle recording is not a whole number
/// of cycles, so it is stretched, not retimed.
fn idle_sum(engine: &Pcm, exhaust: &Pcm) -> Result<Pcm> {
    let (e, x) = (engine.frames() as f32, exhaust.frames() as f32);
    if (e - x).abs() / e.max(1.0) > MAX_DETUNE {
        return Ok(engine.clone());
    }
    ensure!(engine.sample_rate == exhaust.sample_rate, "the engine and exhaust recordings differ in sample rate");
    let fitted = pitch::resample_periodic(&exhaust.samples, engine.samples.len());
    let samples = engine.samples.iter().zip(&fitted).map(|(a, b)| a + b).collect();
    Ok(Pcm { sample_rate: engine.sample_rate, channels: 1, samples, loop_range: None })
}

/// What the layers are made for: how many loops, and the engine's own idle and redline speeds.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Plan {
    /// Loops of each set (under load, off load) to take, at most [`LOOP_LAYERS`].
    pub loops: usize,
    pub idle_rpm: f32,
    pub redline_rpm: f32,
}

/// Which of `speeds` (rising) to take, `count` of them: all those under `dense_below` when they are at most half of `count`,
/// otherwise an even spread of them for half of `count`; the rest spread evenly over the speeds above. Indices, rising.
pub fn choose(speeds: &[f32], dense_below: f32, count: usize) -> Vec<usize> {
    let low = speeds.iter().take_while(|&&rpm| rpm <= dense_below).count();
    if low == 0 || low == speeds.len() {
        return spread(speeds.len(), count);
    }
    let low_count = low.min(count.div_ceil(2));
    let mut out = spread(low, low_count);
    out.extend(spread(speeds.len() - low, count - low_count).into_iter().map(|index| index + low));
    out
}

/// What was met while making the sets of loops.
#[derive(Debug, Default)]
struct Tally {
    worst: f32,
    engine_only: usize,
}

/// One engine's recordings: its model and where its audio is decoded from.
struct Source<'a> {
    model: &'a RevModel,
    audio: &'a dyn Recordings,
}

/// The layers of one set of loops: `engine_loops` of the engine, each summed with the nearest of `exhaust_loops`, at `unit`
/// rpm per cycle per second.
fn loop_layers(
    (engine_loops, engine_audio): (&[RevStream], &dyn Recordings),
    (exhaust_loops, exhaust_audio): (&[RevStream], &dyn Recordings),
    plan: &Plan,
    unit: f32,
    off_load: bool,
    tally: &mut Tally,
) -> Result<Vec<EngineLayer>> {
    // A loop above the host's highest layer speed (a model recorded for a far higher redline than the bike's) is never reached.
    let usable: Vec<&RevStream> = engine_loops.iter().filter(|stream| stream.rpm_with(unit) <= MAX_LAYER_RPM).collect();
    let speeds: Vec<f32> = usable.iter().map(|stream| stream.rpm_with(unit)).collect();
    let mut layers = Vec::new();
    for position in choose(&speeds, DENSE_BELOW * plan.redline_rpm, plan.loops) {
        let stream = usable[position];
        let rpm = stream.rpm_with(unit);
        let partner = partner(stream, exhaust_loops).context("the exhaust model has no loops")?;
        let (a, b) = (mono(engine_audio.decode(stream.number)?)?, mono(exhaust_audio.decode(partner.number)?)?);
        let detune = (partner.cycles as f32 - stream.cycles as f32).abs() / stream.cycles as f32;
        if detune > MAX_DETUNE {
            tally.engine_only += 1;
            layers.push(EngineLayer { rpm, pcm: a, off_load });
            continue;
        }
        tally.worst = tally.worst.max(detune);
        layers.push(EngineLayer { rpm, pcm: sum_matched(&a, stream.cycles, &b, partner.cycles)?, off_load });
    }
    Ok(layers)
}

/// Make the layers of one bike: the loops under load, the loops off load, and the idle once for each set.
pub fn build(
    engine: &RevModel,
    engine_audio: &dyn Recordings,
    exhaust: &RevModel,
    exhaust_audio: &dyn Recordings,
    plan: &Plan,
) -> Result<EngineLayers> {
    ensure!((1..=LOOP_LAYERS).contains(&plan.loops), "1 to {LOOP_LAYERS} loops, not {}", plan.loops);
    let unit = rpm_per_cycle_hz(engine, plan.redline_rpm);
    let mut notes = Vec::new();
    if unit != RPM_PER_CYCLE_HZ {
        notes.push(format!("the loops are read at {unit} rpm per cycle per second: their cycle is one turn"));
    }
    let mut tally = Tally::default();
    let (engine_source, exhaust_source) =
        (Source { model: engine, audio: engine_audio }, Source { model: exhaust, audio: exhaust_audio });
    let mut layers = loop_layers(
        (engine_source.model.on_load.as_slice(), engine_source.audio),
        (exhaust_source.model.on_load.as_slice(), exhaust_source.audio),
        plan,
        unit,
        false,
        &mut tally,
    )?;
    let off = loop_layers(
        (engine_source.model.off_load.as_slice(), engine_source.audio),
        (exhaust_source.model.off_load.as_slice(), exhaust_source.audio),
        plan,
        unit,
        true,
        &mut tally,
    )?;
    if tally.engine_only > 0 {
        notes.push(format!(
            "{} loops without the exhaust: its pitch is more than {:.0}% away",
            tally.engine_only,
            MAX_DETUNE * 100.0
        ));
    }
    let idle = idle_layer(&engine_source, &exhaust_source, plan, unit, &layers, &mut notes)?;
    layers.extend(off);
    if let Some(idle) = idle {
        // The same recording under load and off load: at idle the throttle decides which one the engine plays.
        let off_idle = EngineLayer { off_load: true, ..idle.clone() };
        let free = layers.iter().all(|layer| !layer.off_load || (layer.rpm - off_idle.rpm).abs() >= 1.0);
        layers.push(idle);
        if free {
            layers.push(off_idle);
        }
    }
    layers.sort_by(|x, y| x.rpm.total_cmp(&y.rpm));
    for off_load in [false, true] {
        let set: Vec<f32> = layers.iter().filter(|layer| layer.off_load == off_load).map(|layer| layer.rpm).collect();
        ensure!(set.windows(2).all(|pair| pair[0] < pair[1]), "two layers have the same engine speed");
    }
    Ok(EngineLayers { layers, worst_detune: tally.worst, notes })
}

/// The idle layer, or `None` (with a note) when the idle recording cannot be placed.
fn idle_layer(
    engine: &Source,
    exhaust: &Source,
    plan: &Plan,
    unit: f32,
    loops: &[EngineLayer],
    notes: &mut Vec<String>,
) -> Result<Option<EngineLayer>> {
    let a = mono(engine.audio.decode(engine.model.idle.number)?)?;
    let b = mono(exhaust.audio.decode(exhaust.model.idle.number)?)?;
    let fade = (a.sample_rate as f32 * IDLE_FADE_MS / 1000.0) as usize;
    let idle_hz = plan.idle_rpm / unit;
    let measured = pitch::cycle_frequency(&a.samples, a.sample_rate, IDLE_BAND.0 * idle_hz, IDLE_BAND.1 * idle_hz);
    let (rpm, note) = match measured {
        Some(hz) => (hz * unit, None),
        None => (
            plan.idle_rpm,
            Some("the idle recording has no clear period near the idle speed: it plays at the physics idle speed"),
        ),
    };
    if rpm < 100.0 || loops.iter().any(|layer| (layer.rpm - rpm).abs() < 1.0) {
        notes.push("no idle layer: its speed is out of range or the same as a loop's".to_owned());
        return Ok(None);
    }
    notes.extend(note.map(str::to_owned));
    Ok(Some(EngineLayer { rpm, pcm: idle_sum(&a, &b)?.seamless_loop(fade), off_load: false }))
}

/// Give every layer the same loudness (root mean square), the largest that keeps the loudest peak at `peak` (0 to 1).
///
/// The recorded loops differ by 20 dB between the low and the high speeds (the synthesiser's own gain follows its throttle and
/// load inputs); the host brings the whole engine to a fixed level from the layers' loudness, and with its correction limited
/// the quiet low layers made holes in the level at low engine speeds.
pub fn equalise(layers: &mut EngineLayers, peak: f32) -> Result<()> {
    let rms = |layer: &EngineLayer| {
        (layer.pcm.samples.iter().map(|s| s * s).sum::<f32>() / layer.pcm.samples.len().max(1) as f32).sqrt()
    };
    ensure!(layers.layers.iter().all(|layer| rms(layer) > 1e-6), "an engine layer is silent");
    let target = layers.layers.iter().map(|layer| rms(layer) * peak / layer.pcm.peak()).fold(f32::MAX, f32::min);
    for layer in &mut layers.layers {
        let gain = target / rms(layer);
        layer.pcm.samples.iter_mut().for_each(|sample| *sample *= gain);
    }
    Ok(())
}

/// Encode a layer for the manifest: 16-bit PCM WAV.
pub fn encode(layer: &EngineLayer) -> Vec<u8> {
    wav::encode16(&layer.pcm)
}

#[cfg(test)]
#[path = "rev_layers_tests.rs"]
mod tests;
