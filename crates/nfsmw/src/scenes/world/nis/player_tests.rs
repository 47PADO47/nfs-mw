use super::*;
use std::collections::BTreeMap;

use blackbox_chunk::ice::{Key, Space};
use blackbox_nis::SceneType;
use blackbox_sequence::{Action, Engine, Event, Response, State, System};
use glam::Vec3;

use crate::scenes::world::nis::events::START;

/// A three-second scene: frames 0 to 90 at 30 frames per second, with no camera (the orbit plays).
fn plan(name: &str, kind: NisKind) -> Plan {
    Plan {
        name: name.into(),
        kind,
        clock: SceneClock::new(0, 90, SCENE_FPS).expect("a clock"),
        anchor: Anchor::facing(Vec3::ZERO, 0),
        focus: Anchor::facing(Vec3::ZERO, 0),
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

#[test]
fn a_type_4_scene_is_not_anchored_at_the_marker_but_its_orbit_still_circles_it() {
    let marker = Anchor::facing(Vec3::new(10.0, 20.0, 3.0), 4000);
    let render_space = Scene { scene_type: SceneType::Type4, ..scene(0, 90) };
    let plan = Plan::new("IntroNisDD", &render_space, None, marker, NisKind::Intro).expect("a plan");
    assert_eq!(plan.anchor, Anchor::IDENTITY, "type 4 is authored in render space");
    assert_eq!(plan.focus, marker, "the orbit circles the marker the player stands at");
    let at_marker = Scene { scene_type: SceneType::Type0, ..scene(0, 90) };
    let plan = Plan::new("IntroNis05", &at_marker, None, marker, NisKind::Intro).expect("a plan");
    assert_eq!(plan.anchor, marker, "type 0 stands at the marker");
}

#[test]
fn the_car_links_are_made_once_and_the_final_cars_are_handed_back_once() {
    let mut nis = NisPlayer::default();
    assert!(!nis.needs_links(), "no scene, nothing to link");
    nis.start(plan("IntroNis05", NisKind::Intro)).unwrap();
    assert!(nis.needs_links(), "the scene's cars wait for their real cars");
    nis.set_links(vec![(0, Target::Player)]);
    assert!(!nis.needs_links(), "linked once");
    assert!(nis.car_spots().is_empty(), "no car channel was read, so no car stands anywhere");
    nis.advance(3.0);
    assert_eq!(nis.take_final_cars(), Some(Vec::new()), "the scene hands back its linked cars at its end");
    assert_eq!(nis.take_final_cars(), None, "the cars are handed back once");
    nis.set_links(vec![(0, Target::Player)]);
    assert!(!nis.needs_links(), "no scene plays, so nothing is linked");
}

#[test]
fn a_cop_channel_waits_for_its_car_and_gives_up_once_its_time_is_up() {
    let mut nis = NisPlayer::default();
    nis.start(plan("IntroNis05", NisKind::Intro)).unwrap();
    nis.set_links(vec![(0, Target::Player)]);
    nis.wait_for_cops(vec![8, 9]);
    assert_eq!(nis.cop_waiting(), &[8, 9]);
    assert!(nis.linked_cars().is_empty(), "the player's car is no police car");
    nis.link_waiting_cop(8, Target::Spawned(4));
    assert_eq!(nis.cop_waiting(), &[9], "the channel that got its car no longer waits");
    assert_eq!(nis.linked_cars(), vec![4], "the spawned car is taken by its channel");
    assert!(nis.tick_cop_wait(1.0).is_empty(), "one second is under the limit");
    assert_eq!(nis.tick_cop_wait(1.0), vec![9], "two seconds in all and the last channel gives up");
    assert!(nis.cop_waiting().is_empty());
}

#[test]
fn a_skipped_scene_hands_back_its_cars_too() {
    let mut nis = NisPlayer::default();
    nis.start(plan("IntroNis05", NisKind::Intro)).unwrap();
    nis.set_links(vec![(0, Target::Traffic(3))]);
    assert!(nis.skip());
    assert_eq!(nis.take_final_cars(), Some(Vec::new()));
    assert!(!nis.needs_links());
}

/// One engine, named after the scene: a call (id 13) at time zero and a call (id 11) at 1 s, over a 2 s action.
fn sequence() -> Package {
    let mut states = BTreeMap::new();
    let responses = BTreeMap::from([(START, Response { end_state: 20, action: 40 })]);
    states.insert(20, State { filter: None, responses });
    let system = System { initial: 20, default: 20, states };
    let engine = Engine { name: "IntroNis05".into(), name_hash: 77, systems: BTreeMap::from([(10, system)]) };
    let action = Action { name_hash: 40, duration: 2.0, tags: BTreeMap::new(), times: vec![(0.0, 101), (1.0, 102)] };
    let call = |id: u32| Event {
        id: 0x885F_756E,
        data: [0x885F_756E, id].iter().flat_map(|w: &u32| w.to_le_bytes()).collect(),
    };
    let event_lists = BTreeMap::from([(101, vec![call(13)]), (102, vec![call(11)])]);
    Package { engines: vec![engine], actions: BTreeMap::from([(40, action)]), event_lists, strings: BTreeMap::new() }
}

#[test]
fn a_scene_hands_out_its_events_as_its_clock_passes_them() {
    let mut nis = NisPlayer::default();
    nis.start_with_cars(
        plan("IntroNis05", NisKind::Intro),
        Vec::new(),
        Some(Arc::new(sequence())),
        Characters::default(),
    )
    .unwrap();
    assert_eq!(nis.advance(0.0), [NisEvent::CellCall(13)], "the time-zero event fires at the start");
    assert!(nis.advance(0.5).is_empty());
    assert_eq!(nis.advance(1.5), [NisEvent::CellCall(11)]);
    assert!(nis.advance(5.0).is_empty(), "the clock runs out at 3 s and no event waits past it");
    assert_eq!(nis.take_outcome(), Some(NisOutcome::Completed));
}

#[test]
fn skipping_a_scene_stops_its_events() {
    let mut nis = NisPlayer::default();
    nis.start_with_cars(
        plan("IntroNis05", NisKind::Intro),
        Vec::new(),
        Some(Arc::new(sequence())),
        Characters::default(),
    )
    .unwrap();
    assert_eq!(nis.advance(0.0), [NisEvent::CellCall(13)]);
    assert!(nis.skip());
    assert!(nis.advance(2.0).is_empty(), "the skipped scene fires nothing more");
    assert_eq!(nis.take_outcome(), Some(NisOutcome::Skipped));
}

#[test]
fn a_scene_with_no_sequence_fires_nothing() {
    let mut nis = NisPlayer::default();
    nis.start(plan("IntroNis05", NisKind::Intro)).unwrap();
    assert!(nis.advance(0.0).is_empty());
}

#[test]
fn the_audio_request_is_the_playing_scene_until_it_ends() {
    let mut nis = NisPlayer::default();
    assert_eq!(nis.audio_request(), None);
    nis.start(plan("IntroNis05", NisKind::Intro)).unwrap();
    assert_eq!(nis.audio_request(), Some(&NisAudioRequest { scene: Some("IntroNis05".into()) }));
    nis.stop();
    assert_eq!(nis.audio_request(), None);
}

#[test]
fn the_screen_is_cleared_when_the_scene_ends() {
    let mut nis = NisPlayer::default();
    nis.start(plan("IntroNis05", NisKind::Intro)).unwrap();
    nis.screen_mut().start_flash(2.0);
    assert!(nis.screen().flash_alpha().is_some());
    nis.stop();
    assert_eq!(nis.screen().flash_alpha(), None);
}

#[test]
fn a_scene_that_loads_counts_as_playing_and_refuses_a_second_scene() {
    let mut nis = NisPlayer::default();
    let (loading, _answer) = Loading::for_test("IntroNis05");
    nis.loading = Some(loading);
    assert!(nis.is_playing());
    assert_eq!(nis.playing_name(), None, "nothing plays yet");
    assert_eq!(nis.audio_request(), None, "the voice follows the scene, not its load");
    assert!(nis.refuse_while_playing().is_err());
    assert!(nis.advance(1.0).is_empty());
    assert!(nis.is_playing(), "an unfinished load goes on");
    assert!(!nis.skip(), "a scene that has not begun cannot be skipped");
    assert!(nis.status().contains("loading"));
}

#[test]
fn a_scene_that_fails_to_load_ends_as_skipped() {
    let mut nis = NisPlayer::default();
    let (loading, answer) = Loading::for_test("IntroNis05");
    nis.loading = Some(loading);
    answer.send(Err("no bundle".into())).unwrap();
    assert!(nis.advance(1.0).is_empty());
    assert!(!nis.is_playing());
    assert_eq!(nis.take_outcome(), Some(NisOutcome::Skipped));
    assert!(nis.take_cut());
    assert!(nis.status().contains("failed (no bundle)"));
}

#[test]
fn a_load_whose_thread_vanished_ends_as_skipped() {
    let mut nis = NisPlayer::default();
    let (loading, answer) = Loading::for_test("IntroNis05");
    nis.loading = Some(loading);
    drop(answer);
    nis.advance(0.0);
    assert!(!nis.is_playing());
    assert_eq!(nis.take_outcome(), Some(NisOutcome::Skipped));
}

#[test]
fn stopping_a_scene_that_loads_ends_it_as_skipped() {
    let mut nis = NisPlayer::default();
    let (loading, _answer) = Loading::for_test("IntroNis05");
    nis.loading = Some(loading);
    assert!(nis.stop());
    assert!(!nis.is_playing());
    assert_eq!(nis.take_outcome(), Some(NisOutcome::Skipped));
    assert!(!nis.stop(), "nothing is left to stop");
}
