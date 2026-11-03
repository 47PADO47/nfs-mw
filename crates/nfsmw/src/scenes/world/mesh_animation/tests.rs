use super::*;

fn placement(flags: u32) -> Instance {
    Instance {
        tree: 0,
        section: 0,
        flags,
        speed: 1.0,
        master_delay: 0.0,
        loop_delay: 0.0,
        begin_range: 0,
        end_range: 0,
        start_trigger: 0,
        stop_trigger: 0,
        track: 0,
        direction: 0,
        matrix: [0.0; 16],
    }
}

#[test]
fn default_loop_keeps_end_frame_until_crossing() {
    let mut c = Playback::new(&placement(0), 30.0).unwrap();
    c.advance(1.0);
    assert_eq!(c.frame, 30.0);
    c.advance(0.25);
    assert_eq!(c.frame, 7.5);
}

#[test]
fn linear_nonloop_clamps_and_reverse_starts_at_zero() {
    let mut c = Playback::new(&placement(8), 30.0).unwrap();
    c.advance(2.0);
    assert_eq!(c.frame, 30.0);
    assert_eq!(c.state, State::Playing);
    let mut c = Playback::new(&placement(0x1028), 30.0).unwrap();
    c.advance(0.25);
    assert_eq!(c.frame, 22.5);
}

#[test]
fn pingpong_reflects_forward_but_clamps_reverse_overshoot() {
    let mut c = Playback::new(&placement(0x10), 30.0).unwrap();
    c.advance(1.25);
    assert_eq!(c.frame, 22.5);
    assert!(c.reverse);
    c.advance(1.0);
    assert_eq!(c.frame, 0.0);
    assert!(!c.reverse);
}

#[test]
fn triggers_override_linear_and_pause_at_endpoint() {
    let mut p = placement(0x28);
    p.stop_trigger = 1;
    let mut c = Playback::new(&p, 30.0).unwrap();
    c.advance(1.25);
    assert_eq!(c.frame, 22.5);
    assert_eq!(c.state, State::Paused);
    c.advance(1.0);
    assert_eq!(c.frame, 22.5);
    p.start_trigger = 2;
    let c = Playback::new(&p, 30.0).unwrap();
    assert!(!c.visible());
}

#[test]
fn master_and_loop_delays_are_separate_ticks() {
    let mut p = placement(0x28);
    p.master_delay = 1.0;
    p.loop_delay = 0.5;
    let mut c = Playback::new(&p, 30.0).unwrap();
    c.advance(1.25);
    assert_eq!(c.frame, 7.5);
    c.advance(0.75);
    assert_eq!(c.frame, 15.0);
    c.advance(0.75);
    assert_eq!(c.frame, 7.5);
    c.advance(0.25);
    assert_eq!(c.frame, 7.5);
}

#[test]
fn ranges_are_scaled_and_hitches_wrap_once_without_looping_work() {
    let mut p = placement(0x68);
    p.speed = 2.0;
    p.begin_range = 15;
    p.end_range = 30;
    let mut c = Playback::new(&p, 90.0).unwrap();
    c.advance(1.25);
    assert_eq!(c.frame, 45.0);
    c.advance(100.0);
    assert_eq!(c.frame, 6015.0);
    c.advance(f32::NAN);
    assert_eq!(c.frame, 6015.0);
}

#[test]
fn invalid_playback_metadata_does_not_poison_the_world() {
    let mut p = placement(0x28);
    p.speed = f32::NAN;
    assert!(Playback::new(&p, 30.0).is_none());
    p.speed = 0.0;
    assert!(Playback::new(&p, 30.0).is_some());
    p.master_delay = f32::INFINITY;
    assert!(Playback::new(&p, 30.0).is_none());
    assert!(Playback::new(&placement(0x68), 30.0).is_none());
}

#[test]
fn track_direction_is_not_a_reverse_playback_flag() {
    let mut p = placement(0x28);
    p.direction = -1;
    let mut c = Playback::new(&p, 30.0).unwrap();
    c.advance(1.0);
    assert_eq!(c.frame, 0.0);
    assert!(!c.visible());
}
