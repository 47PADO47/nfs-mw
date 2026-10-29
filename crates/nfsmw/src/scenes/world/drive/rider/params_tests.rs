use super::*;

fn inertia() -> Inertia {
    Inertia {
        bone: "pelvis".into(),
        min: [-0.06, -0.15, -0.04],
        max: [0.0, 0.15, 0.2],
        braking_m: 0.0,
        full_acceleration: 0.0,
        rate: 4.0,
    }
}

fn sample() -> RiderSample {
    RiderSample { max_lean: 1.0, front_down: true, rear_down: true, ..RiderSample::default() }
}

fn run(sample: &RiderSample, seconds: f32) -> Smoothed {
    let mut smoothed = Smoothed::default();
    for _ in 0..(seconds * 60.0) as usize {
        smoothed.step(sample, &inertia(), 1.0 / 60.0);
    }
    smoothed
}

#[test]
fn a_wheelie_needs_the_front_tyre_up_and_the_rear_one_down_and_is_the_height_of_the_nose() {
    let up = RiderSample { elevation: 0.3, ..sample() };
    assert_eq!(up.wheelie(), 0.0, "the front tyre is down");
    let lifted = RiderSample { front_down: false, ..up };
    assert_eq!(lifted.wheelie(), 0.3);
    assert_eq!(RiderSample { elevation: -0.2, ..lifted }.wheelie(), 0.0);
    assert_eq!(RiderSample { rear_down: false, ..lifted }.wheelie(), 0.0, "both off the ground is neither");
    assert_eq!(lifted.stoppie(), 0.0);
}

#[test]
fn a_stoppie_needs_the_rear_tyre_up_and_the_front_one_down_and_the_nose_down() {
    let nose_down = RiderSample { elevation: -0.25, rear_down: false, ..sample() };
    assert_eq!(nose_down.stoppie(), 0.25);
    assert_eq!(RiderSample { rear_down: true, ..nose_down }.stoppie(), 0.0);
    assert_eq!(nose_down.wheelie(), 0.0);
}

#[test]
fn the_speed_is_in_kilometres_an_hour_and_never_negative() {
    assert!((RiderSample { speed: 10.0, ..sample() }.speed_kmh() - 36.0).abs() < 1e-4);
    assert_eq!(RiderSample { speed: -3.0, ..sample() }.speed_kmh(), 0.0);
}

#[test]
fn the_lean_is_the_mix_of_the_riders_and_the_bikes_over_85_percent_faded_in_with_the_speed() {
    // At 50 km/h and more the fade is full: a mix of 0.85 or more is the whole lean (times the multiplier of 1).
    let lean = |rider_lean: f32, bike_lean: f32, speed: f32| {
        let s = RiderSample { rider_lean, bike_lean, speed, ..sample() };
        run(&s, 8.0).values().lean
    };
    assert!((lean(1.0, 1.0, 20.0) - 1.0).abs() < 0.01);
    assert!((lean(-1.0, -1.0, 20.0) + 1.0).abs() < 0.01);
    // Half the lean at 0.425 of the mix: 0.3 * 0.5 + 0.7 * 0.5 = 0.5 is 0.588 of the full.
    assert!((lean(0.5, 0.5, 20.0) - 0.5 / 0.85).abs() < 0.01);
    // The bike's lean weighs 0.7 and the rider's 0.3.
    assert!((lean(0.0, 0.85, 20.0) - 0.7).abs() < 0.01);
    assert!((lean(0.85, 0.0, 20.0) - 0.3).abs() < 0.01);
    // Slower, less: the fade is 1/6 at a standstill and full at 50 km/h.
    assert!((lean(1.0, 1.0, 0.0) - 1.0 / 6.0).abs() < 0.01);
    // Above 250 km/h the lean decays towards 0.3 of itself at 400.
    assert!((lean(1.0, 1.0, 400.0 / 3.6) - 0.3).abs() < 0.01);
}

#[test]
fn the_lean_is_smoothed_and_starts_from_nothing() {
    let s = RiderSample { rider_lean: 1.0, bike_lean: 1.0, speed: 20.0, ..sample() };
    let first = run(&s, 1.0 / 60.0).values().lean;
    assert!(first > 0.0 && first < 0.05, "{first}");
    let later = run(&s, 3.0).values().lean;
    assert!(later > 0.9, "{later}");
}

#[test]
fn the_tuck_follows_the_riders_tuck_level_unless_he_brakes_or_lifts_the_front() {
    let tucked = RiderSample { tuck_level: 1.0, speed: 50.0, ..sample() };
    assert!(run(&tucked, 3.0).values().tuck > 0.95);
    assert!(run(&RiderSample { tuck_level: 0.0, ..tucked }, 3.0).values().tuck < 0.01, "no input, no throttle");
    assert!(run(&RiderSample { front_brake: 1.0, ..tucked }, 3.0).values().tuck < 0.01);
    // The throttle held for a moment tucks him slowly (the sum of the input and the held throttle, smoothed at 1).
    let throttle = RiderSample { throttle: 1.0, speed: 50.0, ..sample() };
    let quick = run(&throttle, 0.5).values().tuck;
    let slow = run(&throttle, 4.0).values().tuck;
    assert!(quick > 0.0 && quick < 0.5 && slow > 0.95, "{quick} {slow}");
    // A wheelie in progress lifts him out of the tuck.
    let wheelie = RiderSample { elevation: 0.6, front_down: false, ..tucked };
    assert!(run(&wheelie, 5.0).values().tuck < 0.05);
}

#[test]
fn the_wheelie_pose_follows_the_height_of_the_nose_with_a_delay() {
    let wheelie = RiderSample { elevation: 0.5, front_down: false, ..sample() };
    let early = run(&wheelie, 0.15).values();
    assert_eq!(early.wheelie_alpha, 0.0, "the delay of 0.2 s has not passed");
    assert!(early.wheelie > 0.45, "the entry follows at 8 per second: {}", early.wheelie);
    let late = run(&wheelie, 4.0).values();
    assert!(late.wheelie_alpha > 0.99 && (late.wheelie - 0.5).abs() < 1e-4, "{late:?}");
    // Under 0.15 there is no pose.
    let low = RiderSample { elevation: 0.1, front_down: false, ..sample() };
    assert_eq!(run(&low, 4.0).values().wheelie_alpha, 0.0);
}

#[test]
fn the_wheelie_leans_to_the_side_of_the_bikes_lean() {
    let side = |bike_lean: f32| wheelie_direction(bike_lean);
    assert_eq!(side(0.03), 0.0);
    assert_eq!(side(0.5), 1.0);
    assert_eq!(side(-0.5), -1.0);
    // Between the dead band and the full range the weight is under 1: the value is squared away from the default.
    assert!((side(0.125) - 0.25).abs() < 1e-6, "{}", side(0.125));
    assert!((side(-0.125) + 0.25).abs() < 1e-6);
}

#[test]
fn the_stoppie_latches_above_a_fifth_of_the_nose_and_lets_go_when_it_is_level() {
    let mut smoothed = Smoothed::default();
    let step = |smoothed: &mut Smoothed, dive: f32| {
        let s = RiderSample { elevation: -dive, rear_down: false, ..sample() };
        for _ in 0..180 {
            smoothed.step(&s, &inertia(), 1.0 / 60.0);
        }
        smoothed.values()
    };
    assert_eq!(step(&mut smoothed, 0.1).stoppie, 0.0, "under 0.18 and never above: still 0");
    assert!(step(&mut smoothed, 0.25).stoppie > 0.99);
    assert!(step(&mut smoothed, 0.1).stoppie > 0.99, "between 0 and 0.18 it holds what it had");
    let level = RiderSample { rear_down: true, ..sample() };
    for _ in 0..180 {
        smoothed.step(&level, &inertia(), 1.0 / 60.0);
    }
    assert!(smoothed.values().stoppie < 0.01 && smoothed.values().stoppie_time < 0.01);
}

#[test]
fn the_displacement_is_taken_into_the_cars_axes_and_held_to_the_ranges_of_the_data() {
    let inertia = inertia();
    assert_eq!(shift_target(Vec3::ZERO, &inertia), Vec3::ZERO);
    // The physics axes are x right, y up, z forward: back 3 cm, 10 cm to the right, 5 cm up.
    let shift = shift_target(Vec3::new(0.10, 0.05, -0.03), &inertia);
    assert!(shift.distance(Vec3::new(-0.03, -0.10, 0.05)) < 1e-6, "{shift:?}");
    // The ranges: forward from -6 cm to 0, sideways 15 cm either way, up from -4 cm to 20 cm.
    assert_eq!(shift_target(Vec3::new(0.0, 0.0, 0.2), &inertia).x, 0.0, "forward is not asked for");
    assert_eq!(shift_target(Vec3::new(0.0, 0.0, -0.2), &inertia).x, -0.06);
    assert_eq!(shift_target(Vec3::new(-0.5, 0.0, 0.0), &inertia).y, 0.15);
    assert_eq!(shift_target(Vec3::new(0.5, 0.0, 0.0), &inertia).y, -0.15);
    assert_eq!(shift_target(Vec3::new(0.0, -0.3, 0.0), &inertia).z, -0.04);
    assert_eq!(shift_target(Vec3::new(0.0, 0.9, 0.0), &inertia).z, 0.2);
    assert_eq!(shift_target(Vec3::splat(f32::NAN), &inertia), Vec3::ZERO);
}

#[test]
fn the_turn_of_the_body_is_passed_on_as_the_physics_has_it() {
    let turned = RiderSample { rotation: Vec3::new(0.1, -0.05, 0.02), ..sample() };
    assert_eq!(run(&turned, 0.1).rotation(), Vec3::new(0.1, -0.05, 0.02));
    let bad = RiderSample { rotation: Vec3::splat(f32::NAN), ..sample() };
    assert_eq!(run(&bad, 0.1).rotation(), Vec3::ZERO);
}

#[test]
fn the_shift_is_the_mirror_image_for_the_other_side() {
    let inertia = inertia();
    let right = shift_target(Vec3::new(0.08, -0.02, -0.01), &inertia);
    let left = shift_target(Vec3::new(-0.08, -0.02, -0.01), &inertia);
    assert!((right.y + left.y).abs() < 1e-7);
    assert_eq!((right.x, right.z), (left.x, left.z));
}

#[test]
fn the_shift_follows_the_riders_place_at_the_rate_of_the_data() {
    let hanging = RiderSample { offset: Vec3::new(0.1, -0.02, -0.04), ..sample() };
    let after_a_tick = run(&hanging, 1.0 / 60.0).shift();
    let settled = run(&hanging, 3.0).shift();
    assert!(settled.distance(Vec3::new(-0.04, -0.1, -0.02)) < 1e-4, "{settled:?}");
    assert!(after_a_tick.length() > 0.0 && after_a_tick.length() < settled.length() * 0.2, "{after_a_tick:?}");
    // Back in its place he comes home.
    let mut smoothed = run(&hanging, 3.0);
    for _ in 0..180 {
        smoothed.step(&sample(), &inertia(), 1.0 / 60.0);
    }
    assert!(smoothed.shift().length() < 1e-3);
}

#[test]
fn a_step_of_no_time_or_a_bad_time_changes_nothing() {
    let mut smoothed = run(&RiderSample { rider_lean: 0.5, bike_lean: 0.5, speed: 10.0, ..sample() }, 1.0);
    let before = smoothed.clone();
    for dt in [0.0, -1.0, f32::NAN, f32::INFINITY] {
        smoothed.step(&RiderSample { bike_lean: -1.0, ..sample() }, &inertia(), dt);
        assert_eq!(smoothed, before);
    }
}

#[test]
fn the_rider_stands_on_the_pegs_when_his_weight_is_forward_through_a_wheelie() {
    let wheelie = RiderSample { elevation: 0.5, front_down: false, ..sample() };
    assert!(run(&RiderSample { weight: 0.0, ..wheelie }, 4.0).values().standing < 0.01, "weight not forward: he sits");
    assert!((run(&RiderSample { weight: 0.8, ..wheelie }, 4.0).values().standing - 1.0).abs() < 0.01);
    // Halfway through the weight ramp (0.35 of 0.2 to 0.5) is 0.5.
    assert!((run(&RiderSample { weight: 0.35, ..wheelie }, 4.0).values().standing - 0.5).abs() < 0.01);
    assert!(
        run(&RiderSample { weight: 0.8, ..sample() }, 4.0).values().standing < 0.01,
        "both wheels down: no wheelie"
    );
}

#[test]
fn the_standing_wheelies_entry_value_is_half_the_entry_ramp_delayed_and_followed() {
    let wheelie = RiderSample { elevation: 0.5, front_down: false, ..sample() };
    assert_eq!(run(&sample(), 2.0).values().standing_entry, 0.0);
    // 0.5 elevation is past the end of the ramp (0.4): the value settles at 0.5.
    assert!((run(&wheelie, 4.0).values().standing_entry - 0.5).abs() < 0.01);
    // It has not moved 0.2 s after the wheelie began: the 0.4 s delay.
    assert!(run(&wheelie, 0.2).values().standing_entry < 0.01);
}

#[test]
fn a_drift_pose_takes_over_between_8_and_20_degrees_to_either_side() {
    let settled = |angle: f32| run(&RiderSample { drift_deg: angle, ..sample() }, 3.0).values().drift;
    assert_eq!(settled(0.0), 0.0);
    assert_eq!(settled(7.9), 0.0);
    assert!((settled(20.0) - 1.0).abs() < 0.01);
    assert!((settled(-20.0) + 1.0).abs() < 0.01);
    // 14 degrees is half the ramp, and the rule weighs the ramp by itself: 0.25.
    assert!((settled(14.0) - 0.25).abs() < 0.01);
    assert!((settled(-14.0) + 0.25).abs() < 0.01);
}

#[test]
fn the_steering_the_blend_space_reads_follows_the_input_at_6_per_second() {
    let steer = |seconds: f32| run(&RiderSample { steer: 1.0, ..sample() }, seconds).values().steer;
    assert!((steer(1.0 / 12.0) - 0.5).abs() < 0.02, "{}", steer(1.0 / 12.0));
    assert!((steer(1.0) - 1.0).abs() < 1e-4);
    assert!((run(&RiderSample { steer: 3.0, ..sample() }, 2.0).values().steer - 1.0).abs() < 1e-4, "clamped");
}
