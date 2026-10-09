//! The director's rules: which set, which control value, when the songs may play.

use super::super::director::{Director, FALL, Plan, RESUME_DELAY, RISE, SET_HOLD};
use super::super::state::MusicState;

const DT: f32 = 0.1;

/// Run `director` for `secs` of game time and return the last plan.
fn run(director: &mut Director, state: &MusicState, driving: bool, secs: f32) -> Plan {
    let mut plan = director.update(state, driving, 0.0);
    for _ in 0..(secs / DT).round() as usize {
        plan = director.update(state, driving, DT);
    }
    plan
}

#[test]
fn without_a_pursuit_the_songs_play() {
    let mut director = Director::default();
    let plan = director.update(&MusicState::default(), true, DT);
    assert_eq!(plan, Plan { pursuit: None, control: 0, radio: true });
}

#[test]
fn a_pursuit_takes_the_music_at_the_intensity_it_starts_with() {
    let mut director = Director::default();
    let plan = director.update(&MusicState::chased(3, 1.0), true, DT);
    assert_eq!(plan, Plan { pursuit: Some(3), control: 127, radio: false });
}

#[test]
fn the_songs_return_forty_seconds_after_the_pursuit_ends() {
    let mut director = Director::default();
    run(&mut director, &MusicState::chased(1, 0.5), true, 5.0);
    let plan = director.update(&MusicState::default(), true, DT);
    assert_eq!((plan.pursuit, plan.radio), (None, false));
    assert_eq!(director.resume_in(), Some(RESUME_DELAY));
    let plan = run(&mut director, &MusicState::default(), true, RESUME_DELAY - 1.0);
    assert!(!plan.radio, "still waiting after {} s", RESUME_DELAY - 1.0);
    let plan = run(&mut director, &MusicState::default(), true, 1.5);
    assert!(plan.radio);
    assert_eq!(director.resume_in(), None);
}

#[test]
fn a_new_pursuit_cancels_the_wait() {
    let mut director = Director::default();
    run(&mut director, &MusicState::chased(1, 0.5), true, 1.0);
    run(&mut director, &MusicState::default(), true, 10.0);
    assert!(director.resume_in().is_some());
    let plan = director.update(&MusicState::chased(2, 0.5), true, DT);
    assert_eq!(plan.pursuit, Some(2));
    assert_eq!(director.resume_in(), None);
}

#[test]
fn a_new_set_needs_to_be_asked_for_a_while() {
    let mut director = Director::default();
    run(&mut director, &MusicState::chased(1, 0.5), true, 1.0);
    // Heat flickers between 2 and 1: the music stays on set 1.
    for _ in 0..5 {
        let plan = run(&mut director, &MusicState::chased(2, 0.5), true, SET_HOLD - 0.5);
        assert_eq!(plan.pursuit, Some(1));
        let plan = run(&mut director, &MusicState::chased(1, 0.5), true, 0.2);
        assert_eq!(plan.pursuit, Some(1));
    }
    // Asked for long enough, it switches.
    let plan = run(&mut director, &MusicState::chased(2, 0.5), true, SET_HOLD + 0.5);
    assert_eq!(plan.pursuit, Some(2));
    assert!(!plan.radio);
}

#[test]
fn the_control_value_rises_fast_and_falls_slowly() {
    let mut director = Director::default();
    run(&mut director, &MusicState::chased(1, 0.0), true, 1.0);
    assert_eq!(director.control(), 0);

    let plan = run(&mut director, &MusicState::chased(1, 1.0), true, 1.0);
    assert_eq!(plan.control, RISE.round() as u8, "one second of rising");
    let plan = run(&mut director, &MusicState::chased(1, 1.0), true, 5.0);
    assert_eq!(plan.control, 127, "it stops at the target");

    let plan = run(&mut director, &MusicState::chased(1, 0.0), true, 1.0);
    assert_eq!(plan.control, 127 - FALL.round() as u8, "one second of falling");
    let plan = run(&mut director, &MusicState::chased(1, 0.0), true, 30.0);
    assert_eq!(plan.control, 0);
}

#[test]
fn leaving_the_game_drops_the_pursuit_without_a_wait() {
    let mut director = Director::default();
    run(&mut director, &MusicState::chased(4, 1.0), true, 1.0);
    let plan = director.update(&MusicState::chased(4, 1.0), false, DT);
    assert_eq!((plan.pursuit, plan.radio), (None, true));
    assert_eq!(director.resume_in(), None);
}

#[test]
fn a_pursuit_outside_a_game_is_not_played() {
    let mut director = Director::default();
    let plan = run(&mut director, &MusicState::chased(2, 1.0), false, 2.0);
    assert_eq!(plan.pursuit, None);
}

#[test]
fn a_race_leaves_the_songs_alone() {
    let mut director = Director::default();
    let state = MusicState { pursuit: None, racing: true };
    assert_eq!(director.update(&state, true, DT), Plan { pursuit: None, control: 0, radio: true });
}
