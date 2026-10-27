//! The effects of real 2012 car bundles. Ignored without `MW2012_GAME_DIR` (the 2012 install).

use std::path::{Path, PathBuf};

use super::*;

fn install() -> Option<PathBuf> {
    std::env::var_os("MW2012_GAME_DIR").map(PathBuf::from)
}

fn effects_of(root: &Path, car: &str) -> Found {
    let bytes = std::fs::read(root.join("VEHICLES").join(format!("VEH_{car}_HI.BNDL"))).unwrap();
    let bundle = Bundle::parse(&bytes).unwrap();
    let samples = sample::read_all(&bundle);
    let shared = std::fs::read(root.join("VEHICLES/WAVES.BNDL")).ok();
    of(&bundle, &samples, shared.as_deref())
}

fn seconds(wav: &[u8]) -> f32 {
    (wav.len() - 44) as f32 / 2.0 / 48_000.0
}

fn describe(name: &str, files: &[Vec<u8>]) {
    let lengths: Vec<String> = files.iter().map(|wav| format!("{:.2}s", seconds(wav))).collect();
    println!("  {name}: {} files {lengths:?}", files.len());
}

#[test]
#[ignore = "reads the 2012 install (MW2012_GAME_DIR)"]
fn an_engine_without_a_turbo_has_shifts_backfires_and_pops_and_no_turbo() {
    let Some(root) = install() else { return };
    // The Porsche 911 Carrera S: a flat six without a turbo.
    let found = effects_of(&root, "1085007");
    let files = &found.files;
    describe("shift", &files.shift);
    describe("backfire", &files.backfire);
    describe("pops", &files.pops);
    assert_eq!(files.shift.len(), 6, "the six samples of the gear-change graphs");
    assert!(files.shift.iter().all(|wav| (0.1..0.4).contains(&seconds(wav))));
    assert_eq!(files.backfire.len(), 3, "the one the BackfireGain graph plays and the two variants listed beside it");
    assert!(files.backfire.iter().all(|wav| (0.6..0.9).contains(&seconds(wav))));
    assert_eq!(files.pops.len(), 9, "the samples of the graph that reads GetPopProbability");
    assert!(files.pops.iter().all(|wav| seconds(wav) <= MAX_POP_SECONDS));
    assert!(files.turbo_whine.is_empty() && files.turbo_blowoff.is_empty(), "no graph names a turbo");
    assert!(found.not_engine.is_empty());
}

#[test]
#[ignore = "reads the 2012 install (MW2012_GAME_DIR)"]
fn a_turbo_car_has_a_whistle_and_a_blow_off_and_the_whistle_is_not_part_of_its_engine() {
    let Some(root) = install() else { return };
    let found = effects_of(&root, "1085698");
    let files = &found.files;
    describe("turbo_whine", &files.turbo_whine);
    describe("turbo_blowoff", &files.turbo_blowoff);
    assert_eq!(files.turbo_whine.len(), 3, "the two loops of the car's NormalisedTurboRPM graph and the shared tone");
    assert_eq!(files.turbo_blowoff.len(), 1, "the shared one-shot that dies away over a couple of seconds");
    assert!((2.0..3.0).contains(&seconds(&files.turbo_blowoff[0])));
    assert_eq!(found.not_engine.len(), 2, "the shared tone is not in the car's bundle, so it was never an engine loop");

    // A turbo car whose graphs have no TurboBlowoffGain has a whistle and no blow-off.
    let found = effects_of(&root, "2076399");
    assert!(!found.files.turbo_whine.is_empty() && found.files.turbo_blowoff.is_empty());
}

#[test]
#[ignore = "reads the 2012 install (MW2012_GAME_DIR)"]
fn the_shared_turbo_samples_are_a_looping_tone_and_a_one_shot_that_dies_away() {
    let Some(root) = install() else { return };
    let bytes = std::fs::read(root.join("VEHICLES/VEH_1085698_HI.BNDL")).unwrap();
    let bundle = Bundle::parse(&bytes).unwrap();
    let graphs = Graphs::new(&bundle);
    let shared_bytes = std::fs::read(root.join("VEHICLES/WAVES.BNDL")).unwrap();
    let samples = shared_turbo(&graphs, Some(&shared_bytes));
    assert_eq!(samples.len(), 2);
    let tone = samples.iter().find(|s| s.looped).expect("a loop");
    let blowoff = samples.iter().find(|s| !s.looped).expect("a one-shot");
    // The one-shot dies away: the end of it is far quieter than the start.
    let third = blowoff.samples.len() / 3;
    let level = |part: &[f32]| (part.iter().map(|s| s * s).sum::<f32>() / part.len() as f32).sqrt();
    assert!(level(&blowoff.samples[2 * third..]) < 0.25 * level(&blowoff.samples[..third]));
    assert!(tone.seconds() < 1.0 && tone.rms() > blowoff.rms());
}
