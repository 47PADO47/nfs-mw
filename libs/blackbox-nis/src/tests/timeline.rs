use crate::{Playback, PlaybackState, SceneClock};

fn clock(start: i32, vanish: i32) -> SceneClock {
    SceneClock::new(start, vanish, 30.0).expect("30 fps is a valid rate")
}

#[test]
fn the_clock_converts_frames_to_seconds() {
    let whole = clock(0, 180);
    assert_eq!(whole.start_secs(), 0.0);
    assert_eq!(whole.end_secs(), 6.0);
    assert_eq!(whole.duration(), 6.0);

    let offset = clock(30, 60);
    assert_eq!(offset.start_secs(), 1.0);
    assert_eq!(offset.end_secs(), 2.0);
    assert_eq!(offset.duration(), 1.0);
}

#[test]
fn the_rate_is_the_callers() {
    let at_25 = SceneClock::new(0, 50, 25.0).expect("25 fps is valid");
    assert_eq!(at_25.end_secs(), 2.0);
    assert_eq!(at_25.fps(), 25.0);
}

#[test]
fn a_rate_that_is_not_positive_and_finite_is_refused() {
    assert!(SceneClock::new(0, 30, 0.0).is_none());
    assert!(SceneClock::new(0, 30, -30.0).is_none());
    assert!(SceneClock::new(0, 30, f32::NAN).is_none());
    assert!(SceneClock::new(0, 30, f32::INFINITY).is_none());
}

#[test]
fn a_reversed_range_has_no_duration() {
    assert_eq!(clock(60, 30).duration(), 0.0);
}

#[test]
fn playback_advances_and_finishes_at_the_end() {
    let mut run = Playback::new(clock(0, 180), false);
    assert_eq!(run.state(), PlaybackState::Playing { scene_time: 0.0 });
    assert_eq!(run.advance(2.0), PlaybackState::Playing { scene_time: 2.0 });
    assert_eq!(run.advance(-1.0), PlaybackState::Playing { scene_time: 2.0 }, "a negative step is ignored");
    assert_eq!(run.advance(f32::NAN), PlaybackState::Playing { scene_time: 2.0 }, "a NaN step is ignored");
    assert_eq!(run.advance(3.5), PlaybackState::Playing { scene_time: 5.5 });
    assert_eq!(run.advance(1.0), PlaybackState::Finished);
    assert_eq!(run.elapsed(), 6.0, "the elapsed time stops at the duration");
    assert_eq!(run.scene_time(), 6.0);
    assert_eq!(run.advance(1.0), PlaybackState::Finished, "a finished run stays finished");
}

#[test]
fn scene_time_starts_at_the_clock_start() {
    let mut run = Playback::new(clock(30, 60), false);
    assert_eq!(run.state(), PlaybackState::Playing { scene_time: 1.0 });
    assert_eq!(run.advance(0.5), PlaybackState::Playing { scene_time: 1.5 });
    assert_eq!(run.elapsed(), 0.5);
    assert_eq!(run.advance(0.5), PlaybackState::Finished);
}

#[test]
fn skip_jumps_to_the_last_frame_when_allowed() {
    let mut run = Playback::new(clock(0, 180), true);
    assert!(run.can_skip());
    run.advance(1.0);
    assert!(run.skip());
    assert!(run.is_finished());
    assert_eq!(run.state(), PlaybackState::Finished);
    assert_eq!(run.elapsed(), 6.0);
    assert!(!run.skip(), "a finished run has nothing to skip");
}

#[test]
fn skip_does_nothing_when_not_allowed() {
    let mut run = Playback::new(clock(0, 180), false);
    run.advance(1.0);
    assert!(!run.skip());
    assert_eq!(run.state(), PlaybackState::Playing { scene_time: 1.0 });
    assert_eq!(run.elapsed(), 1.0);
}
