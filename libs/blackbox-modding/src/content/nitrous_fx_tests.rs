use super::*;

const FULL: &str = r#"
format = 1
[audio]
start = "NITROUS/MW2012/START.WAV"
loop = "NITROUS/MW2012/LOOP.WAV"
end = "NITROUS/MW2012/END.WAV"
loop_gain = 0.8
loop_fade_out = 0.5
[flame]
tint = [0.4, 0.6, 1.0]
[screen]
vignette = 0.2
radial_blur = 0.3
fade_out = 1.0
"#;

#[test]
fn a_full_profile_parses_with_defaults_for_what_it_leaves_out() {
    let profile = parse(FULL).unwrap();
    let audio = profile.audio.unwrap();
    assert_eq!(audio.start.as_deref(), Some("NITROUS/MW2012/START.WAV"));
    assert_eq!(audio.looped.as_deref(), Some("NITROUS/MW2012/LOOP.WAV"));
    assert_eq!((audio.start_gain, audio.loop_gain, audio.loop_fade_in, audio.loop_fade_out), (1.0, 0.8, 0.1, 0.5));
    assert!(audio.replace_original, "the profile's sounds replace the game's by default");
    assert_eq!(
        profile.flame,
        Some(FlameProfile { tint: [0.4, 0.6, 1.0], gain: 1.0, replace_burn: false, replace_pop: false })
    );
    let screen = profile.screen.unwrap();
    assert_eq!((screen.vignette, screen.radial_blur, screen.fade_in, screen.fade_out), (0.2, 0.3, 0.25, 1.0));
}

#[test]
fn an_empty_profile_changes_nothing() {
    assert_eq!(parse("format = 1").unwrap(), NitrousProfile::default());
}

#[test]
fn a_bad_profile_is_refused_with_its_reason() {
    assert!(matches!(parse("format = 2"), Err(ContentError::Field { field: "format", .. })));
    assert!(matches!(parse("[audio]\nstart = \"A.WAV\""), Err(ContentError::Parse(_))), "format is required");
    assert!(matches!(parse("format = 1\nsurprise = 1"), Err(ContentError::Parse(_))), "unknown keys are refused");
    assert!(matches!(parse("format = 1\n[audio]\nstart = \"../x.wav\""), Err(ContentError::Field { .. })));
    assert!(matches!(parse("format = 1\n[audio]\nstart = \"X.MP3\""), Err(ContentError::Field { .. })));
    assert!(matches!(parse("format = 1\n[audio]\nloop_gain = 1.0"), Err(ContentError::Field { field: "audio", .. })));
    assert!(matches!(parse("format = 1\n[flame]\ntint = [1.0, 2.0, 9.0]"), Err(ContentError::Field { .. })));
    assert!(matches!(parse("format = 1\n[screen]\nvignette = 1.5"), Err(ContentError::Field { .. })));
    assert!(matches!(parse("format = 1\n[screen]\nfade_in = -1.0"), Err(ContentError::Field { .. })));
}

#[test]
fn the_profile_path_follows_the_variant_id() {
    assert_eq!(profile_path("mw2012"), "NITROUS/MW2012/profile.toml");
}

#[test]
fn a_burn_follows_the_fades_and_reports_both_edges_once() {
    let mut burn = BurnFollower::default();
    let idle = burn.step(false, 0.5, 1.0, 0.1);
    assert_eq!((idle.level, idle.started, idle.ended), (0.0, false, false));
    let lit = burn.step(true, 0.5, 1.0, 0.25);
    assert!(lit.started && !lit.ended);
    assert!((lit.level - 0.5).abs() < 1e-6);
    let still = burn.step(true, 0.5, 1.0, 0.25);
    assert!(!still.started);
    assert!((still.level - 1.0).abs() < 1e-6);
    let held = burn.step(true, 0.5, 1.0, 10.0);
    assert_eq!(held.level, 1.0, "the level never passes 1");
    let out = burn.step(false, 0.5, 1.0, 0.5);
    assert!(out.ended && !out.started);
    assert!((out.level - 0.5).abs() < 1e-6, "one sweep takes fade_out seconds");
    let gone = burn.step(false, 0.5, 1.0, 5.0);
    assert!(!gone.ended);
    assert_eq!(gone.level, 0.0);
}

#[test]
fn a_fade_of_zero_is_instant_and_a_negative_step_does_not_run_backwards() {
    let mut burn = BurnFollower::default();
    assert_eq!(burn.step(true, 0.0, 0.0, 0.0).level, 1.0);
    assert_eq!(burn.step(false, 0.0, 0.0, 0.016).level, 0.0);
    let mut slow = BurnFollower::default();
    slow.step(true, 1.0, 1.0, 0.5);
    let back = slow.step(true, 1.0, 1.0, -3.0);
    assert!((back.level - 0.5).abs() < 1e-6);
}

const CAMERA_AND_GRADE: &str = r#"
format = 1
[screen]
edge = [0.04, 0.11, 0.09]
edge_inner = 0.6
tint = [0.9, 1.0, 1.1]
contrast = 1.2
fov = 8.0
pull_back = 0.4
shake = 0.01
shake_rate = 20.0
"#;

#[test]
fn the_camera_and_grade_keys_parse_and_the_defaults_change_nothing() {
    let screen = parse(CAMERA_AND_GRADE).unwrap().screen.unwrap();
    assert_eq!((screen.edge, screen.edge_inner, screen.tint), ([0.04, 0.11, 0.09], 0.6, [0.9, 1.0, 1.1]));
    assert_eq!((screen.contrast, screen.fov, screen.pull_back), (1.2, 8.0, 0.4));
    assert_eq!((screen.shake, screen.shake_rate), (0.01, 20.0));
    assert!(screen.has_grade() && screen.has_camera());
    let plain = parse("format = 1\n[screen]\nvignette = 0.1").unwrap().screen.unwrap();
    assert!(!plain.has_grade() && !plain.has_camera(), "a profile with neither key changes neither");
    assert_eq!(plain, ScreenProfile { vignette: 0.1, ..Default::default() });
}

#[test]
fn bad_camera_and_grade_values_are_refused() {
    for text in [
        "edge = [2.0, 5.0, 1.0]",
        "tint = [1.0, 1.0]",
        "contrast = -0.5",
        "fov = 90.0",
        "pull_back = 20.0",
        "shake = 1.0",
        "shake_rate = 0.0",
        "edge_inner = 1.5",
    ] {
        let result = parse(&format!("format = 1\n[screen]\n{text}"));
        assert!(matches!(result, Err(ContentError::Field { .. }) | Err(ContentError::Parse(_))), "{text}: {result:?}");
    }
}

#[test]
fn the_values_follow_the_level_from_neutral_to_the_full_value() {
    let screen = parse(CAMERA_AND_GRADE).unwrap().screen.unwrap();
    assert_eq!(screen.edge_at(0.0), [1.0; 3]);
    assert!(screen.edge_at(1.0).iter().zip([0.04, 0.11, 0.09]).all(|(a, b)| (a - b).abs() < 1e-6));
    assert!((screen.tint_at(0.5)[0] - 0.95).abs() < 1e-6);
    assert!((screen.contrast_at(0.5) - 1.1).abs() < 1e-6);
    assert_eq!((screen.fov_at(0.0), screen.fov_at(1.0), screen.fov_at(0.25)), (0.0, 8.0, 2.0));
    assert!((screen.pull_back_at(0.5) - 0.2).abs() < 1e-6);
    assert_eq!(screen.fov_at(7.0), 8.0, "the level is clamped");
}

#[test]
fn the_shake_is_zero_at_rest_bounded_by_its_size_and_moves_with_time() {
    let screen = parse(CAMERA_AND_GRADE).unwrap().screen.unwrap();
    assert_eq!(screen.shake_at(1.0, 0.0), [0.0; 3], "no level, no shake");
    assert_eq!(ScreenProfile::default().shake_at(1.0, 1.0), [0.0; 3], "no size, no shake");
    let mut seen = std::collections::BTreeSet::new();
    for step in 0..200 {
        // Not a multiple of the shake rate, or the samples alias onto a few values.
        let offset = screen.shake_at(step as f32 / 47.0, 1.0);
        assert!(offset.iter().all(|c| c.abs() <= 0.01 + 1e-6), "{offset:?}");
        assert!(offset[2].abs() <= 0.003 + 1e-6, "the forward axis moves least");
        seen.insert((offset[0] * 1e6) as i32);
    }
    assert!(seen.len() > 50, "it keeps moving");
    let half = screen.shake_at(0.3, 0.5);
    let full = screen.shake_at(0.3, 1.0);
    assert!((half[0] * 2.0 - full[0]).abs() < 1e-7, "the size follows the level");
    assert_eq!(screen.shake_at(f32::NAN, 1.0), [0.0; 3]);
}
