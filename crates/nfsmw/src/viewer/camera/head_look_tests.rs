use super::*;

/// Runs `seconds` of `stick` at `fps` frames a second.
fn hold(look: &mut HeadLook, stick: [f32; 2], seconds: f32, fps: u32) {
    for _ in 0..(seconds * fps as f32) as u32 {
        look.update(stick, 1.0 / fps as f32);
    }
}

#[test]
fn a_look_starts_straight_ahead_and_a_stick_at_rest_keeps_it_there() {
    let mut look = HeadLook::default();
    hold(&mut look, [0.0, 0.0], 1.0, 60);
    assert_eq!((look.right_deg(), look.up_deg()), (0.0, 0.0));
}

#[test]
fn a_held_stick_reaches_the_limits_and_no_further() {
    let mut look = HeadLook::default();
    hold(&mut look, [1.0, 1.0], 3.0, 60);
    assert!((look.right_deg() - 11.0).abs() < 1e-2 && (look.up_deg() - 25.0).abs() < 1e-2, "{look:?}");
    // A number beyond the stick's travel is the stick's end, and the other way is the other limit.
    hold(&mut look, [50.0, 9.0], 3.0, 60);
    assert!(look.right_deg() <= 11.0 && look.up_deg() <= LOOK_UP_LIMIT_DEG, "{look:?}");
    hold(&mut look, [-1.0, -1.0], 3.0, 60);
    assert!((look.right_deg() + 11.0).abs() < 1e-2 && (look.up_deg() + 30.0).abs() < 1e-2, "{look:?}");
}

#[test]
fn the_look_is_a_pure_function_of_the_stick_with_more_room_down_than_up() {
    assert_eq!((LOOK_UP_LIMIT_DEG, LOOK_DOWN_LIMIT_DEG, LOOK_YAW_LIMIT_DEG), (25.0, 30.0, 11.0));
    assert_eq!(HeadLook::target([0.0, 0.0]), [0.0, 0.0]);
    assert_eq!(HeadLook::target([1.0, 1.0]), [11.0, 25.0]);
    assert_eq!(HeadLook::target([-1.0, -1.0]), [-11.0, -30.0]);
    assert_eq!(HeadLook::target([0.5, 0.5]), [5.5, 12.5]);
    assert_eq!(HeadLook::target([0.5, -0.5]), [5.5, -15.0]);
    assert_eq!(HeadLook::target([7.0, -7.0]), [11.0, -30.0], "beyond the travel is the end");
    assert_eq!(HeadLook::target([f32::NAN, f32::INFINITY * 0.0]), [0.0, 0.0]);
    assert_eq!(
        HeadLook::target([f32::NEG_INFINITY, f32::INFINITY]),
        [0.0, 0.0],
        "an infinite stick is not a number: at rest"
    );
    // Monotonic and continuous across the middle.
    let mut last = HeadLook::target([0.0, -1.0])[1];
    for step in 1..=200 {
        let pitch = HeadLook::target([0.0, -1.0 + step as f32 / 100.0])[1];
        assert!(pitch >= last, "{step}");
        assert!((pitch - last).abs() < 0.4, "no jump at {step}: {last} -> {pitch}");
        last = pitch;
    }
    // The old data limit is inside the new ones.
    assert!(LOOK_PITCH_DATA_LIMIT_DEG < LOOK_UP_LIMIT_DEG && LOOK_PITCH_DATA_LIMIT_DEG < LOOK_DOWN_LIMIT_DEG);
}

#[test]
fn half_a_stick_is_half_the_limit() {
    let mut look = HeadLook::default();
    hold(&mut look, [0.5, -0.5], 3.0, 60);
    assert!((look.right_deg() - 5.5).abs() < 1e-2 && (look.up_deg() + 15.0).abs() < 1e-2, "{look:?}");
}

#[test]
fn it_springs_back_to_straight_ahead_within_two_seconds_of_letting_go() {
    for fps in [20, 30, 60, 144, 240] {
        let mut look = HeadLook::default();
        hold(&mut look, [-1.0, 1.0], 3.0, fps);
        assert!(look.right_deg() < -10.9 && look.up_deg() > 24.9, "{fps} fps: {look:?}");
        hold(&mut look, [0.0, 0.0], 2.0, fps);
        assert!(look.right_deg().abs() < 0.01 && look.up_deg().abs() < 0.01, "{fps} fps: {look:?}");
    }
}

#[test]
fn the_return_is_smooth_and_never_overshoots() {
    let mut look = HeadLook::default();
    hold(&mut look, [1.0, 0.0], 3.0, 60);
    let mut last = look.right_deg();
    for _ in 0..180 {
        look.update([0.0, 0.0], 1.0 / 60.0);
        assert!(look.right_deg() <= last && look.right_deg() >= 0.0, "{last} -> {}", look.right_deg());
        last = look.right_deg();
    }
}

#[test]
fn the_time_constant_is_a_quarter_of_a_second_whatever_the_frame_rate() {
    let a_quarter = 1.0 - (-1.0f32).exp();
    for fps in [4, 20, 60, 240] {
        let mut look = HeadLook::default();
        hold(&mut look, [1.0, 0.0], 0.25, fps);
        assert!((look.right_deg() / 11.0 - a_quarter).abs() < 1e-3, "{fps} fps: {}", look.right_deg());
    }
}

#[test]
fn a_frame_of_no_time_or_a_bad_input_does_not_move_it() {
    let mut look = HeadLook::default();
    hold(&mut look, [1.0, 1.0], 3.0, 60);
    let held = look;
    for dt in [0.0, -1.0, f32::NAN, f32::INFINITY] {
        look.update([-1.0, -1.0], dt);
        assert_eq!(look, held, "dt {dt}");
    }
    look.update([f32::NAN, f32::INFINITY], 1.0 / 60.0);
    assert!(look.right_deg() < held.right_deg(), "a stick that is not a number is the stick at rest: it heads back");
    assert!(look.right_deg().is_finite() && look.up_deg().is_finite());
}
