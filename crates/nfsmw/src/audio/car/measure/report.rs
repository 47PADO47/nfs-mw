//! Runs every scenario for a car and reduces the renders to named numbers (`Metrics`), the form the regression tests and the
//! comparison with the stock car read.

use std::collections::BTreeMap;

use super::dsp;
use super::rig::{FPS, Heard, RATE, Rig};
use super::scenario;

/// Steady engine speeds measured, as fractions of the way from idle to redline.
pub const STEPS: [f32; 11] = [0.0, 0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 0.9, 1.0];
/// The throttles of the steady table.
pub const THROTTLES: [f32; 2] = [0.3, 1.0];
/// Seconds of each steady segment, and how many of its last seconds are measured.
const SEGMENT: f32 = 1.6;
const TAIL: f32 = 0.5;

/// Named numbers of one car, sorted by name.
#[derive(Debug, Default, Clone)]
pub struct Metrics(pub BTreeMap<String, f32>);

impl Metrics {
    pub fn get(&self, key: &str) -> f32 {
        self.0.get(key).copied().unwrap_or(f32::NAN)
    }

    fn set(&mut self, key: impl Into<String>, value: f32) {
        self.0.insert(key.into(), value);
    }

    /// One line per number, `car key value`, for the comparison scripts.
    pub fn lines(&self, car: &str) -> String {
        self.0.iter().map(|(key, value)| format!("M {car} {key} {value:.3}\n")).collect()
    }
}

/// The key of a steady-table entry.
pub fn steady_key(throttle: f32, step: f32, what: &str) -> String {
    format!("steady.t{:03}.r{:03}.{what}", (throttle * 100.0) as u32, (step * 100.0) as u32)
}

/// Measure `car` in every scenario.
pub fn measure(rig: &Rig, car: &str) -> Metrics {
    let mut metrics = Metrics::default();
    steady(rig, car, &mut metrics);
    continuous(rig, car, "sweep", &scenario::sweep(1.0, 9.0), &mut metrics);
    continuous(rig, car, "sweep_part", &scenario::sweep(0.3, 9.0), &mut metrics);
    continuous(rig, car, "coast", &scenario::coast(5.0), &mut metrics);
    drive(rig, car, &mut metrics);
    metrics_for(rig, car, "nitrous", &scenario::nitrous(), &mut metrics);
    impacts(rig, car, &mut metrics);
    metrics
}

fn steady(rig: &Rig, car: &str, metrics: &mut Metrics) {
    let mut frames = Vec::new();
    let mut order = Vec::new();
    for throttle in THROTTLES {
        for step in STEPS {
            frames.extend(scenario::steady(step, throttle, SEGMENT));
            order.push((throttle, step));
        }
    }
    let pcm = rig.render(car, &frames, Heard::Engine);
    let segment = (SEGMENT * RATE as f32) as usize;
    let tail = (TAIL * RATE as f32) as usize;
    for (at, (throttle, step)) in order.into_iter().enumerate() {
        let part = &pcm[(at + 1) * segment - tail..(at + 1) * segment];
        metrics.set(steady_key(throttle, step, "db"), dsp::rms_db(part));
        metrics.set(steady_key(throttle, step, "centroid"), dsp::centroid_hz(part, RATE));
        metrics.set(steady_key(throttle, step, "f0"), dsp::fundamental_hz(part, RATE, 25.0, 500.0));
        metrics.set(steady_key(throttle, step, "peak"), dsp::peak_hz(part, RATE, 40.0, 600.0));
    }
}

/// A continuous pull or coast: smoothness, gaps, clicks and how the pitch follows the engine speed.
fn continuous(rig: &Rig, car: &str, name: &str, frames: &[super::rig::Frame], metrics: &mut Metrics) {
    let pcm = rig.render(car, frames, Heard::Engine);
    dump(car, name, &pcm);
    levels(name, &pcm, metrics);
    // The first half second is the engine starting from silence; the rest of the pull is what is judged.
    let body = &pcm[RATE as usize / 2..];
    let loudest = dsp::window_db(body, RATE, 0.1).into_iter().fold(dsp::FLOOR_DB, f32::max);
    metrics.set(format!("{name}.step100ms"), dsp::max_level_step_db(body, RATE, 0.1, loudest - 40.0));
    metrics.set(format!("{name}.click"), dsp::click_ratio(body, RATE));
    metrics.set(format!("{name}.gaps"), dsp::gaps(body, RATE, 0.05, loudest - 30.0) as f32);
    let hop = (0.5 * RATE as f32) as usize;
    let size = (0.4 * RATE as f32) as usize;
    let tracks: Vec<f32> = (0..pcm.len().saturating_sub(size) / hop + 1)
        .filter(|at| at * hop + size <= pcm.len())
        .map(|at| dsp::fundamental_hz(&pcm[at * hop..at * hop + size], RATE, 25.0, 500.0))
        .collect();
    let found: Vec<f32> = tracks.iter().copied().filter(|f| *f > 0.0).collect();
    let drops = found.windows(2).filter(|pair| pair[1] < pair[0] * 0.93).count();
    let jumps = found.windows(2).filter(|pair| pair[1] > pair[0] * 1.4 || pair[1] < pair[0] / 1.4).count();
    metrics.set(format!("{name}.f0_unpitched"), (tracks.len() - found.len()) as f32 / tracks.len().max(1) as f32);
    metrics.set(format!("{name}.f0_drops"), drops as f32);
    metrics.set(format!("{name}.f0_jumps"), jumps as f32);
    metrics.set(
        format!("{name}.f0_ratio"),
        match (found.first(), found.last()) {
            (Some(first), Some(last)) => last / first,
            _ => 0.0,
        },
    );
    let third = pcm.len() / 3;
    metrics.set(format!("{name}.centroid_first"), dsp::centroid_hz(&pcm[..third], RATE));
    metrics.set(format!("{name}.centroid_last"), dsp::centroid_hz(&pcm[2 * third..], RATE));
}

fn levels(name: &str, pcm: &[f32], metrics: &mut Metrics) {
    metrics.set(format!("{name}.loud"), dsp::loudness_db(pcm, RATE));
    metrics.set(format!("{name}.rms"), dsp::rms_db(pcm));
    metrics.set(format!("{name}.peak"), dsp::peak_db(pcm));
    metrics.set(format!("{name}.clip"), dsp::clipped(pcm) as f32);
}

/// The standing start with shifts, heard as engine, as the other sounds, and all together.
fn drive(rig: &Rig, car: &str, metrics: &mut Metrics) {
    let frames = scenario::start_with_shifts();
    let engine = rig.render(car, &frames, Heard::Engine);
    levels("start.engine", &engine, metrics);
    let sfx = rig.render(car, &frames, Heard::Sfx);
    levels("start.sfx", &sfx, metrics);
    metrics.set("start.sfx.bursts", dsp::bursts(&sfx, RATE, 0.02, 9.0) as f32);
    metrics.set("start.sfx.bursts_per_s", dsp::bursts(&sfx, RATE, 0.02, 9.0) as f32 / (sfx.len() as f32 / RATE as f32));
    let both = rig.render(car, &frames, Heard::Both);
    levels("start.all", &both, metrics);
    metrics.set("start.sfx_minus_engine_db", dsp::loudness_db(&sfx, RATE) - dsp::loudness_db(&engine, RATE));
}

fn metrics_for(rig: &Rig, car: &str, name: &str, frames: &[super::rig::Frame], metrics: &mut Metrics) {
    for (heard, label) in [(Heard::Engine, "engine"), (Heard::Sfx, "sfx")] {
        let pcm = rig.render(car, frames, heard);
        levels(&format!("{name}.{label}"), &pcm, metrics);
    }
}

fn impacts(rig: &Rig, car: &str, metrics: &mut Metrics) {
    let crash = rig.render(car, &scenario::crash(), Heard::Sfx);
    let from = FPS as usize * RATE as usize / FPS as usize;
    let after = &crash[from..from + RATE as usize / 2];
    let before = &crash[..from];
    metrics.set("hit.peak", dsp::peak_db(after));
    metrics.set("hit.rms_500ms", dsp::rms_db(after));
    metrics.set("hit.before_rms", dsp::rms_db(before));
    let scrape = rig.render(car, &scenario::scrape(), Heard::Sfx);
    let inside = &scrape[RATE as usize + RATE as usize / 4..3 * RATE as usize];
    metrics.set("scrape.rms", dsp::rms_db(inside));
    metrics.set("scrape.peak", dsp::peak_db(inside));
    metrics.set("scrape.before_rms", dsp::rms_db(&scrape[..RATE as usize]));
}

/// With `NFSMW_MEASURE_DUMP` naming a folder, write the level of each 100 ms window of a render to `<car>-<name>.txt`, for
/// looking at where a step or a hole is.
fn dump(car: &str, name: &str, pcm: &[f32]) {
    let Some(folder) = std::env::var_os("NFSMW_MEASURE_DUMP") else { return };
    let text: String = dsp::window_db(pcm, RATE, 0.1)
        .iter()
        .enumerate()
        .map(|(at, db)| {
            format!(
                "{:.1} {db:.1}
",
                at as f32 * 0.1
            )
        })
        .collect();
    let _ = std::fs::create_dir_all(&folder);
    let _ = std::fs::write(std::path::Path::new(&folder).join(format!("{car}-{name}.txt")), text);
}
