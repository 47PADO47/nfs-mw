//! A 2012 car's other sounds, from the sample resources of its bundle: the gear change, the turbo's whistle and
//! blow-off, the backfire and the pops (docs/formats/chameleon-sound.md, "Which samples play when").
//!
//! The 2012 game plays a sample from a sound graph, and a graph is named by the live values its expressions read. A
//! sample is taken for an effect only when the graph that reads the effect's value plays it:
//!
//! | Effect | The graph reads | Samples |
//! |---|---|---|
//! | gear change | `GetPlayerNormalisedGearChangeTimeElapsed` | the one-shots it plays |
//! | backfire | `BackfireGain` | that one-shot and the variants listed beside it |
//! | pops | `GetPopProbability`, `PopGain` | the short one-shots it plays |
//! | turbo whistle | `NormalisedTurboRPM`, `TurboToneGain` | the loops it plays, and the loop of the shared `WAVES.BNDL` |
//! | turbo blow-off | `TurboBlowoffGain` | the one-shot of the shared `WAVES.BNDL` |
//!
//! What a graph does with the samples (which of them plays when, how loud) is not decoded, so the game plays them by
//! its own simple rules. A car with no graph for an effect gets none: it is silent for that effect.

use std::collections::BTreeSet;

use chameleon_bundle::Bundle;

use crate::engine::pitch_of;
use crate::graphs::Graphs;
use crate::sample::{self, Sample, gain_to_rms, wav_bytes};

const SHIFT: &str = "GetPlayerNormalisedGearChangeTimeElapsed";
const BACKFIRE: &str = "BackfireGain";
const POPS: [&str; 2] = ["GetPopProbability", "PopGain"];
const TURBO_WHINE: &str = "NormalisedTurboRPM";
const TURBO_BLOWOFF: &str = "TurboBlowoffGain";

/// The peak the loudest one-shot of a list is brought to; the others keep their level relative to it.
const SHOT_PEAK: f32 = 0.9;
/// The level (root mean square) a whistle loop is brought to.
const WHINE_RMS: f32 = 0.25;
/// A one-shot is not boosted more than this.
const MAX_BOOST: f32 = 8.0;
/// Most files in a list (what the car manifest allows).
const MAX_FILES: usize = 16;
/// A one-shot longer than this is not a bang or a click, and is left out of the pops.
const MAX_POP_SECONDS: f32 = 0.5;

/// A car's other sounds as WAV files, ready to write. A list that is empty is an effect the car does not have.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct EffectFiles {
    pub shift: Vec<Vec<u8>>,
    pub turbo_whine: Vec<Vec<u8>>,
    pub turbo_blowoff: Vec<Vec<u8>>,
    pub backfire: Vec<Vec<u8>>,
    pub pops: Vec<Vec<u8>>,
}

impl EffectFiles {
    pub fn is_empty(&self) -> bool {
        *self == Self::default()
    }
}

/// What a car's bundle gives: its effects, and the samples they use that are not engine recordings.
#[derive(Debug, Default)]
pub struct Found {
    pub files: EffectFiles,
    /// The ids of the loops that are the turbo's whistle: not part of the engine.
    pub not_engine: BTreeSet<u64>,
}

/// The effects of the car in `bundle`, whose samples are `samples`. `shared` is the shared sample bundle of the install
/// (`VEHICLES/WAVES.BNDL`), where the turbo's blow-off is kept.
pub fn of(bundle: &Bundle, samples: &[Sample], shared: Option<&[u8]>) -> Found {
    let graphs = Graphs::new(bundle);
    let shift = one_shots(samples, &graphs.samples_reading(SHIFT), f32::MAX);
    let backfire = one_shots(samples, &graphs.with_variants(&graphs.samples_reading(BACKFIRE)), f32::MAX);
    let pop_seeds: BTreeSet<u64> = POPS.iter().flat_map(|value| graphs.samples_reading(value)).collect();
    let pops = one_shots(samples, &graphs.with_variants(&pop_seeds), MAX_POP_SECONDS);
    let mut turbo = pick(samples, &graphs.samples_reading(TURBO_WHINE)).into_iter().cloned().collect::<Vec<_>>();
    let own_whistles: BTreeSet<u64> = turbo.iter().filter(|s| s.looped).map(|s| s.id).collect();
    let shared_turbo = match own_whistles.is_empty() {
        true => Vec::new(),
        false => shared_turbo(&graphs, shared),
    };
    turbo.extend(shared_turbo.iter().filter(|s| s.looped).cloned());
    let blowoff: Vec<&Sample> = shared_turbo.iter().filter(|s| !s.looped).collect();
    let whistles = whistles(turbo.iter().collect());

    let not_engine = own_whistles;
    let whine = whistles.iter().map(|s| wav_bytes(&s.samples, s.rate, gain_to_rms(s.rms(), WHINE_RMS, MAX_BOOST)));
    Found {
        files: EffectFiles {
            shift: shot_files(&shift),
            turbo_whine: whine.collect(),
            turbo_blowoff: shot_files(&blowoff),
            backfire: shot_files(&backfire),
            pops: shot_files(&pops),
        },
        not_engine,
    }
}

/// The samples of `samples` whose ids are in `ids`, in the order of `samples`.
fn pick<'a>(samples: &'a [Sample], ids: &BTreeSet<u64>) -> Vec<&'a Sample> {
    samples.iter().filter(|sample| ids.contains(&sample.id)).collect()
}

/// The samples in `ids` that do not loop and are no longer than `longest` seconds.
fn one_shots<'a>(samples: &'a [Sample], ids: &BTreeSet<u64>, longest: f32) -> Vec<&'a Sample> {
    let mut shots = pick(samples, ids);
    shots.retain(|sample| !sample.looped && sample.seconds() <= longest);
    shots
}

/// The looping samples among `found`, the lowest-pitched first (the whistle rises with the spool).
fn whistles(found: Vec<&Sample>) -> Vec<&Sample> {
    let mut loops: Vec<(f32, &Sample)> =
        found.into_iter().filter(|s| s.looped).map(|s| (pitch_of(&s.samples, s.rate), s)).collect();
    loops.sort_by(|a, b| {
        let key = |pitch: f32| if pitch > 0.0 { pitch } else { f32::MAX };
        key(a.0).total_cmp(&key(b.0)).then(a.1.id.cmp(&b.1.id))
    });
    loops.into_iter().map(|(_, s)| s).take(MAX_FILES).collect()
}

/// WAV files of one-shots, all in proportion: the loudest has the peak [`SHOT_PEAK`].
fn shot_files(shots: &[&Sample]) -> Vec<Vec<u8>> {
    let loudest = shots.iter().map(|s| s.peak()).fold(0.0f32, f32::max);
    let gain = if loudest > 1e-4 { (SHOT_PEAK / loudest).min(MAX_BOOST) } else { 1.0 };
    shots.iter().take(MAX_FILES).map(|s| wav_bytes(&s.samples, s.rate, gain)).collect()
}

/// The samples the turbo graph imports from the shared bundle: a high tone that loops (the whistle at full spool) and a
/// one-shot that dies away over a couple of seconds (the blow-off). Empty when the shared bundle is not at hand or the
/// car's graphs name none.
fn shared_turbo(graphs: &Graphs, shared: Option<&[u8]>) -> Vec<Sample> {
    let Some(bundle) = shared.and_then(|bytes| Bundle::parse(bytes).ok()) else { return Vec::new() };
    let wanted = graphs.shared_imports_reading(TURBO_BLOWOFF);
    wanted.iter().filter_map(|id| bundle.find(*id)).filter_map(|at| sample::read(&bundle, at)).collect()
}

#[cfg(test)]
mod real_tests;
#[cfg(test)]
mod tests;
