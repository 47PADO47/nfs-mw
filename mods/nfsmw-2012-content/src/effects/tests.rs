use super::*;

fn tone(id: u64, hz: f32, looped: bool, seconds: f32, level: f32) -> Sample {
    let rate = 8000;
    let samples = (0..(rate as f32 * seconds) as usize)
        .map(|i| level * (2.0 * std::f32::consts::PI * hz * i as f32 / rate as f32).sin())
        .collect();
    Sample { id, samples, rate, looped }
}

#[test]
fn picking_keeps_the_order_of_the_bundle_and_one_shots_leave_out_loops_and_long_samples() {
    let samples = [tone(1, 100.0, false, 0.2, 0.5), tone(2, 100.0, true, 0.2, 0.5), tone(3, 100.0, false, 1.0, 0.5)];
    let ids = BTreeSet::from([3, 1, 2, 9]);
    assert_eq!(pick(&samples, &ids).iter().map(|s| s.id).collect::<Vec<_>>(), [1, 2, 3]);
    assert_eq!(one_shots(&samples, &ids, f32::MAX).iter().map(|s| s.id).collect::<Vec<_>>(), [1, 3]);
    assert_eq!(one_shots(&samples, &ids, 0.5).iter().map(|s| s.id).collect::<Vec<_>>(), [1]);
}

#[test]
fn a_list_of_one_shots_keeps_its_levels_in_proportion_and_the_loudest_gets_the_peak() {
    let (quiet, loud) = (tone(1, 200.0, false, 0.1, 0.1), tone(2, 200.0, false, 0.1, 0.4));
    let files = shot_files(&[&quiet, &loud]);
    let peak = |wav: &Vec<u8>| wav[44..].as_chunks::<2>().0.iter().map(|b| i16::from_le_bytes(*b).abs()).max().unwrap();
    let (a, b) = (f32::from(peak(&files[0])), f32::from(peak(&files[1])));
    assert!((b / 32767.0 - SHOT_PEAK).abs() < 0.01, "{b}");
    assert!((a / b - 0.25).abs() < 0.01, "{a} against {b}");
    assert!(shot_files(&[]).is_empty());
    let silent = Sample { id: 3, samples: vec![0.0; 100], rate: 8000, looped: false };
    assert_eq!(shot_files(&[&silent]).len(), 1, "silence is written as it is");
}

#[test]
fn no_list_has_more_files_than_a_manifest_allows() {
    let many: Vec<Sample> = (0..20).map(|id| tone(id, 100.0, false, 0.05, 0.5)).collect();
    let refs: Vec<&Sample> = many.iter().collect();
    assert_eq!(shot_files(&refs).len(), MAX_FILES);
}

#[test]
fn the_whistle_is_the_loops_lowest_pitch_first() {
    let samples = [
        tone(1, 600.0, true, 1.0, 0.5),
        tone(2, 200.0, true, 1.0, 0.5),
        tone(3, 400.0, true, 1.0, 0.5),
        tone(4, 100.0, false, 1.0, 0.5),
    ];
    let refs: Vec<&Sample> = samples.iter().collect();
    let ordered: Vec<u64> = whistles(refs).iter().map(|s| s.id).collect();
    assert_eq!(ordered, [2, 3, 1], "a one-shot is not a whistle loop");
}

#[test]
fn a_car_with_nothing_has_no_effects() {
    assert!(EffectFiles::default().is_empty());
    assert!(!EffectFiles { pops: vec![vec![1]], ..Default::default() }.is_empty());
}

fn graphs_of_a_car(gear_change: bool) -> SoundGraphs {
    use chameleon_sound_graph::{Group, Sequence};

    let sequence = |id: u64, driver: &str| Sequence { id, driver: driver.into(), lo: 0.0, hi: 1.0, items: Vec::new() };
    let mut sequences = vec![sequence(1, "Players.LocalPlayer.GetPlayerEngineLoad")];
    let mut handles = vec![1];
    if gear_change {
        sequences.push(sequence(2, "LocalPlayer.GetPlayerNormalisedGearChangeTimeElapsed"));
        handles.push(2);
    }
    // The opponents' engine sound has a gear change of its own, which is not the player's.
    sequences.push(sequence(3, "LocalPlayer.GetPlayerNormalisedGearChangeTimeElapsed"));
    let groups = vec![
        Group { class: "EngineSound2".into(), id: 10, handles },
        Group { class: "EngineSound2".into(), id: 11, handles: vec![3] },
    ];
    SoundGraphs { sequences, groups, ..SoundGraphs::default() }
}

#[test]
fn a_car_whose_player_engine_has_no_gear_change_does_not_clunk() {
    assert!(has_no_gear_change(&graphs_of_a_car(false)));
    assert!(!has_no_gear_change(&graphs_of_a_car(true)));
    assert!(!has_no_gear_change(&SoundGraphs::default()), "a bundle with no engine sound object keeps the clunk");
}

#[test]
fn no_clunk_alone_is_an_effect_the_car_has() {
    assert!(!EffectFiles { no_clunk: true, ..Default::default() }.is_empty());
}
