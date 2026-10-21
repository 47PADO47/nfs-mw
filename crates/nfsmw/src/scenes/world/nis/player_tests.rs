use super::*;
use blackbox_chunk::ice::{Key, Space};
use blackbox_nis::SceneType;
use glam::Vec3;

/// A three-second scene: frames 0 to 90 at 30 frames per second, with no camera (the orbit plays).
fn plan(name: &str, kind: NisKind) -> Plan {
    Plan {
        name: name.into(),
        kind,
        clock: SceneClock::new(0, 90, SCENE_FPS).expect("a clock"),
        anchor: Anchor::facing(Vec3::ZERO, 0),
        camera: None,
    }
}

/// A camera key whose eye runs from `eye[0]` to `eye[1]` in scene space, with the letterbox percent given.
fn key(eye: [[f32; 3]; 2], letterbox: u8) -> Key {
    Key {
        kind: 13,
        smooth: false,
        cubic_eye: false,
        cubic_look: false,
        space_eye: Space::Scene,
        space_look: Space::Scene,
        time: 0.0,
        tangent_length: [1.0, 1.0],
        eye,
        look: [[0.0, 3.0, 2.0]; 2],
        dutch: [0.0; 2],
        lens: [24.0; 2],
        near_clip: [0.5; 2],
        aperture: [37; 2],
        letterbox: [letterbox; 2],
        sim_speed: [100; 2],
    }
}

/// A track of `length` seconds with one key.
fn track(length: f32, letterbox: u8) -> Track {
    Track {
        name: "Track 0".into(),
        start: 0.0,
        length,
        keys: vec![key([[0.0, -8.0, 5.0], [10.0, -8.0, 2.0]], letterbox)],
    }
}

/// A header with the given frame window; the rest is what the player does not read.
fn scene(start_frame: i32, vanish_frame: i32) -> Scene {
    Scene {
        name_hash: 0,
        name: "IntroNis05".into(),
        description_pointer: 0,
        scene_type: SceneType::Type4,
        have_layout: false,
        have_car_animation: true,
        number_of_cars: 0,
        start_frame,
        vanish_frame,
        overlay_name: String::new(),
        description: String::new(),
        characters: Vec::new(),
    }
}

#[test]
fn a_scene_plays_for_its_clock_then_completes_and_hands_back_the_camera() {
    let mut nis = NisPlayer::default();
    nis.start(plan("IntroNis05", NisKind::Intro)).unwrap();
    assert!(nis.is_playing());
    assert!(nis.take_cut(), "the camera cuts to the scene");
    assert!(!nis.take_cut(), "a cut is taken once");
    nis.advance(2.9);
    assert!(nis.is_playing());
    assert_eq!(nis.take_outcome(), None, "no outcome while the scene plays");
    nis.advance(0.2);
    assert!(!nis.is_playing());
    assert!(nis.take_cut(), "the camera cuts back");
    assert_eq!(nis.take_outcome(), Some(NisOutcome::Completed));
    assert_eq!(nis.take_outcome(), None, "the outcome is taken once");
    assert!(nis.status().contains("IntroNis05: Completed"), "{}", nis.status());
}

#[test]
fn only_real_positive_time_moves_the_scene() {
    let mut nis = NisPlayer::default();
    nis.start(plan("IntroNis05", NisKind::Intro)).unwrap();
    for dt in [0.0, -5.0, f32::NAN, f32::INFINITY] {
        nis.advance(dt);
    }
    nis.advance(2.9);
    assert!(nis.is_playing(), "no step of zero, negative or non-finite time counted");
    nis.advance(0.2);
    assert!(!nis.is_playing());
}

#[test]
fn a_skippable_scene_ends_as_skipped_when_the_player_skips_it() {
    for kind in [NisKind::Intro, NisKind::Outro, NisKind::Moment] {
        let mut nis = NisPlayer::default();
        nis.start(plan("Scene", kind)).unwrap();
        assert!(nis.skip(), "{kind:?} can be skipped");
        assert!(!nis.is_playing());
        assert_eq!(nis.take_outcome(), Some(NisOutcome::Skipped), "{kind:?}");
    }
}

#[test]
fn an_arrest_or_an_ending_plays_on_when_the_player_skips_it() {
    for kind in [NisKind::Arrest, NisKind::Ending] {
        let mut nis = NisPlayer::default();
        nis.start(plan("Scene", kind)).unwrap();
        assert!(!nis.skip(), "{kind:?} cannot be skipped");
        assert!(nis.is_playing());
        assert_eq!(nis.take_outcome(), None);
        nis.advance(3.0);
        assert_eq!(nis.take_outcome(), Some(NisOutcome::Completed), "{kind:?} runs to its end");
    }
}

#[test]
fn skip_and_stop_do_nothing_when_no_scene_plays() {
    let mut nis = NisPlayer::default();
    assert!(!nis.skip());
    assert!(!nis.stop());
    assert_eq!(nis.take_outcome(), None);
    assert!(!nis.take_cut());
}

#[test]
fn stop_ends_any_scene_as_skipped_even_an_arrest() {
    let mut nis = NisPlayer::default();
    nis.start(plan("ArrestF02", NisKind::Arrest)).unwrap();
    assert!(nis.stop());
    assert!(!nis.is_playing());
    assert_eq!(nis.take_outcome(), Some(NisOutcome::Skipped));
}

#[test]
fn a_second_scene_waits_for_the_first_to_end() {
    let mut nis = NisPlayer::default();
    nis.start(plan("IntroNis05", NisKind::Intro)).unwrap();
    let err = nis.start(plan("Other", NisKind::Outro)).expect_err("one scene at a time");
    assert!(err.contains("IntroNis05"), "{err}");
    nis.advance(3.0);
    nis.start(plan("Other", NisKind::Outro)).expect("the first has ended");
}

#[test]
fn a_scene_with_no_time_is_refused() {
    let mut nis = NisPlayer::default();
    let mut empty = plan("Empty", NisKind::Outro);
    empty.clock = SceneClock::new(90, 90, SCENE_FPS).expect("a clock");
    assert!(nis.start(empty).is_err());
    assert!(!nis.is_playing());
}

#[test]
fn a_new_scene_drops_an_outcome_nobody_took() {
    let mut nis = NisPlayer::default();
    nis.start(plan("A", NisKind::Intro)).unwrap();
    assert!(nis.skip());
    nis.start(plan("B", NisKind::Intro)).unwrap();
    assert_eq!(nis.take_outcome(), None);
}

#[test]
fn the_plan_takes_its_length_from_the_header_window_or_else_the_track() {
    let anchor = Anchor::facing(Vec3::ZERO, 0);
    let windowed = Plan::new("A", &scene(30, 90), None, anchor, NisKind::Outro).expect("a window");
    assert_eq!((windowed.clock.start_secs(), windowed.clock.end_secs()), (1.0, 3.0));
    let timed = Plan::new("B", &scene(0, 0), Some(track(2.5, 0)), anchor, NisKind::Outro).expect("a track");
    assert_eq!(timed.clock.duration(), 2.5, "an empty window takes the track's length from frame zero");
    assert!(Plan::new("C", &scene(0, 0), None, anchor, NisKind::Outro).is_none(), "no window and no track");
}

#[test]
fn the_camera_is_the_track_placed_at_the_anchor_when_there_is_one() {
    let mut nis = NisPlayer::default();
    let mut with_track = plan("IntroNis05", NisKind::Intro);
    with_track.anchor = Anchor { origin: Vec3::ZERO, turn: 0 };
    with_track.camera = Some(track(3.0, 0));
    nis.start(with_track).unwrap();
    let start = nis.lens(16.0 / 9.0).expect("a camera while the scene plays");
    // The anchor faces +x (turn 0), so the scene's own axes are the render axes.
    assert!((start.position - Vec3::new(0.0, -8.0, 5.0)).length() < 1e-3, "{:?}", start.position);
    assert_eq!(nis.letterbox(), 0.0, "no bars on this track");
}

#[test]
fn the_letterbox_bars_come_from_the_track_sample() {
    let mut nis = NisPlayer::default();
    let mut letterboxed = plan("IntroNis05", NisKind::Intro);
    letterboxed.camera = Some(Track { keys: vec![key([[0.0, -8.0, 5.0]; 2], 40)], ..track(3.0, 0) });
    nis.start(letterboxed).unwrap();
    assert!((nis.letterbox() - 0.05).abs() < 1e-6, "40 percent of the screen height is 0.05");
}

#[test]
fn a_scene_with_no_camera_plays_the_orbit_and_idle_state_has_no_camera() {
    let mut nis = NisPlayer::default();
    assert!(nis.lens(1.0).is_none(), "nothing plays, no camera");
    nis.start(plan("IntroNis05", NisKind::Intro)).unwrap();
    assert!(nis.lens(1.0).is_some(), "the orbit");
    assert!(nis.status().contains("orbit"), "{}", nis.status());
}
