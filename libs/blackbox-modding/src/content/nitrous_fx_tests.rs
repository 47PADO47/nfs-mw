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
