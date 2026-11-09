//! The arcade steering of this game against the reference's (`docs/specs/ride6-arcade-steering.md`): the closed loop of the example bike
//! (with a superbike's gearing, so it holds 60 m/s) at several speeds with `ArcadeSteering::default()` and `ArcadeSteering::REFERENCE`.

use glam::{Quat, Vec3};

use super::{ArcadeSteering, Bike, BikeInput, BikeSpec, InputRates};
use crate::FIXED_STEP;
use crate::ground::FlatGround;

/// The example bike with the data's input rates, a superbike's power and gearing, and the given steering.
fn bike_with(arcade: ArcadeSteering) -> BikeSpec {
    let mut spec = BikeSpec::example();
    spec.input = InputRates { throttle: [3.0, 5.0], brake: [3.0, 5.0], steer: [25.0, 6.0] };
    spec.engine.peak_torque *= 3.5;
    spec.transmission.final_ratio = 4.0;
    spec.steering.arcade = arcade;
    spec
}

/// This game's steering before the lean-in was slowed and the turn given its speed authority (`docs/specs/ride6-arcade-steering.md`, sections 10
/// and 11): the values of `ArcadeSteering::default()` of the previous round, the "now" the playtest report compared with.
fn previous() -> ArcadeSteering {
    ArcadeSteering {
        lean_rate: 3.0,
        roll_spring: 2.0,
        stick_response: 2.0,
        handlebar_response: 1.25,
        authority: [1.0, 1.0],
        wheelie_steer_drop: None,
        ..ArcadeSteering::default()
    }
}

/// The throttle that holds `speed`.
fn hold(speed: f32, bike: &Bike) -> f32 {
    (0.25 + 0.2 * (speed - bike.velocity().length())).clamp(0.0, 1.0)
}

/// Ride a held stick for `seconds` at `speed`: whether it fell, the lateral acceleration in g over the last 2 s, and the mean lean.
fn steady(spec: &BikeSpec, speed: f32, stick: f32, seconds: f32) -> (bool, f32, f32) {
    let ground = FlatGround::new(0.0);
    let mut bike = Bike::new(spec.clone());
    assert!(bike.place_moving(&ground, 0.0, 0.0, 2.0, 0.0, speed));
    let steps = (seconds / FIXED_STEP) as usize;
    let from = steps - (2.0 / FIXED_STEP) as usize;
    let (mut yaw_rate, mut lean, mut n, mut mean_speed) = (0.0_f32, 0.0_f32, 0.0_f32, 0.0_f32);
    for step in 0..steps {
        let throttle = hold(speed, &bike);
        bike.step(FIXED_STEP, &BikeInput { throttle, steer: stick, ..Default::default() }, &ground);
        if bike.fallen() {
            return (true, 0.0, 0.0);
        }
        if step >= from {
            yaw_rate += bike.body().angular_velocity.y.abs();
            mean_speed += bike.velocity().length();
            lean += bike.lean().abs();
            n += 1.0;
        }
    }
    (false, mean_speed / n * (yaw_rate / n) / 9.81, lean / n)
}

/// The seconds a flip of the stick from -1 to +1 takes to bring the lean to 80 percent of the steady lean on the other side; `None` for a fall.
fn flip_time(spec: &BikeSpec, speed: f32) -> Option<f32> {
    let ground = FlatGround::new(0.0);
    let mut bike = Bike::new(spec.clone());
    assert!(bike.place_moving(&ground, 0.0, 0.0, 2.0, 0.0, speed));
    let (hold_for, mut steady_lean) = (4.0_f32, 0.0_f32);
    for step in 0..(8.0 / FIXED_STEP) as usize {
        let t = step as f32 * FIXED_STEP;
        let stick = if t < hold_for { -1.0 } else { 1.0 };
        let throttle = hold(speed, &bike);
        bike.step(FIXED_STEP, &BikeInput { throttle, steer: stick, ..Default::default() }, &ground);
        if bike.fallen() {
            return None;
        }
        if t > hold_for - 0.1 && t < hold_for {
            steady_lean = -bike.lean();
        }
        if t >= hold_for && bike.lean() >= 0.8 * steady_lean {
            return Some(t - hold_for);
        }
    }
    None
}

/// The seconds a step of the stick from 0 to 1 takes to bring the lean from straight running to 90 percent of its steady value (the lean after
/// 10 s of the held stick), at `speed`; `None` for a fall.
fn lean_step_time(spec: &BikeSpec, speed: f32) -> Option<f32> {
    let ground = FlatGround::new(0.0);
    let mut bike = Bike::new(spec.clone());
    assert!(bike.place_moving(&ground, 0.0, 0.0, 2.0, 0.0, speed));
    for _ in 0..(1.0 / FIXED_STEP) as usize {
        let throttle = hold(speed, &bike);
        bike.step(FIXED_STEP, &BikeInput { throttle, ..Default::default() }, &ground);
    }
    let mut lean = Vec::new();
    for _ in 0..(10.0 / FIXED_STEP) as usize {
        let throttle = hold(speed, &bike);
        bike.step(FIXED_STEP, &BikeInput { throttle, steer: 1.0, ..Default::default() }, &ground);
        if bike.fallen() {
            return None;
        }
        lean.push(bike.lean());
    }
    let last = (2.0 / FIXED_STEP) as usize;
    let steady = lean.iter().rev().take(last).sum::<f32>() / last as f32;
    let index = lean.iter().position(|&l| l >= 0.9 * steady)?;
    Some(index as f32 * FIXED_STEP)
}

/// The radius (m) of the circle a held full stick rides at `speed` m/s: the speed over the yaw rate of the last 2 s; `None` for a fall.
fn radius(spec: &BikeSpec, speed: f32) -> Option<f32> {
    let (fell, g, _) = steady(spec, speed, 1.0, 8.0);
    (!fell).then(|| speed * speed / (g * 9.81))
}

/// Several bikes: the example, a heavy one and a light one.
fn bikes(arcade: ArcadeSteering) -> [BikeSpec; 3] {
    let mut heavy = bike_with(arcade);
    heavy.chassis.mass *= 1.35;
    heavy.engine.peak_torque *= 1.3;
    let mut light = bike_with(arcade);
    light.chassis.mass *= 0.85;
    [bike_with(arcade), heavy, light]
}

#[test]
fn the_lean_in_is_two_thirds_of_the_way_from_the_arcade_time_to_the_originals() {
    // 0 to 90 percent of the steady lean at 60 km/h, with the stick stepped to the right.
    let speed = 60.0 / 3.6;
    let now = lean_step_time(&bike_with(previous()), speed).expect("no fall");
    let original = lean_step_time(&bike_with(ArcadeSteering::REFERENCE), speed).expect("no fall");
    let new = lean_step_time(&bike_with(ArcadeSteering::default()), speed).expect("no fall");
    println!("lean 0 to 90 percent at 60 km/h: now {now:.3} s, original {original:.3} s, new {new:.3} s");
    assert!(now < new && new < original, "{now} < {new} < {original}");
    let share = (new - now) / (original - now);
    assert!((0.55..=0.80).contains(&share), "{:.0} percent of the way to the original", share * 100.0);
}

#[test]
fn the_slower_lean_in_does_not_slow_with_the_speed() {
    let spec = bike_with(ArcadeSteering::default());
    let at_20 = lean_step_time(&spec, 20.0).expect("no fall at 20 m/s");
    for speed in [10.0_f32, 40.0, 60.0] {
        let at = lean_step_time(&spec, speed).expect("no fall");
        assert!(at < at_20 * 1.25 + 0.05, "{speed} m/s: {at} s against {at_20} s at 20 m/s");
    }
}

/// The speeds of the turning circle table (km/h).
const KMH: [f32; 5] = [20.0, 40.0, 80.0, 120.0, 200.0];

#[test]
fn the_turning_circle_is_tighter_when_slow_and_wider_when_fast_progressively() {
    for (which, (now, new)) in bikes(previous()).into_iter().zip(bikes(ArcadeSteering::default())).enumerate() {
        let ratio: Vec<f32> = KMH
            .iter()
            .map(|kmh| {
                let speed = kmh / 3.6;
                let was = radius(&now, speed).expect("no fall (now)");
                let is = radius(&new, speed).expect("no fall (new)");
                println!("bike {which}: {kmh} km/h radius now {was:.1} m, new {is:.1} m, ratio {:.2}", is / was);
                is / was
            })
            .collect();
        assert!(ratio[0] < 0.97 && ratio[1] < 0.97, "bike {which}: tighter at 20 and 40 km/h: {ratio:?}");
        assert!(ratio[3] > 1.03 && ratio[4] > 1.03, "bike {which}: wider at 120 and 200 km/h: {ratio:?}");
        for pair in ratio.windows(2) {
            assert!(pair[1] >= pair[0] - 0.02, "bike {which}: the ratio rises with the speed: {ratio:?}");
        }
    }
}

#[test]
fn the_turning_radius_grows_with_the_speed_in_every_steering() {
    for arcade in [ArcadeSteering::default(), previous(), ArcadeSteering::REFERENCE] {
        let spec = bike_with(arcade);
        let radii: Vec<f32> = KMH.iter().map(|kmh| radius(&spec, kmh / 3.6).expect("no fall")).collect();
        assert!(radii.windows(2).all(|pair| pair[1] > pair[0]), "{radii:?}");
    }
}

#[test]
fn the_reference_steering_is_neutral_and_the_default_is_not() {
    assert!(ArcadeSteering::default().is_enabled() && !ArcadeSteering::REFERENCE.is_enabled());
    // A steering whose every field is set to its neutral value by hand rides exactly as the reference does.
    let by_hand = ArcadeSteering {
        lateral_grip: 1.0,
        lean_limit: 1.0,
        lean_rate: 1.0,
        time_scale_speed_floor: 0.0,
        mirror_stick_curve: false,
        roll_spring: 1.0,
        stick_response: 1.0,
        handlebar_response: 1.0,
        lean_help: 1.0,
        lean_help_limit: 1.0,
        fall_angular_speed: 1.0,
        cornering_assist: 0.0,
        authority: [1.0, 1.0],
        wheelie_steer_drop: None,
        ..ArcadeSteering::REFERENCE
    };
    assert_eq!(by_hand, ArcadeSteering::REFERENCE);
    let a = steady(&bike_with(ArcadeSteering::REFERENCE), 20.0, 1.0, 5.0);
    let b = steady(&bike_with(by_hand), 20.0, 1.0, 5.0);
    assert_eq!(a, b);
}

#[test]
fn the_reference_turns_at_about_one_g_and_the_arcade_at_about_one_and_a_half() {
    // The arcade bike turns harder than 1.3 g at low and medium speeds and still more than the lean alone (0.75 g) at 144 km/h, where the
    // speed authority takes part of the assist away.
    for (speed, least, most) in [(20.0_f32, 1.4, 1.95), (40.0, 1.15, 1.85)] {
        let (fell, reference, _) = steady(&bike_with(ArcadeSteering::REFERENCE), speed, 1.0, 8.0);
        assert!(!fell && reference < 1.3, "{speed} m/s: the reference turns at {reference} g");
        let (fell, arcade, lean) = steady(&bike_with(ArcadeSteering::default()), speed, 1.0, 8.0);
        assert!(!fell, "{speed} m/s: fell");
        assert!((least..=most).contains(&arcade), "{speed} m/s: {arcade} g");
        assert!(lean < 0.85, "{speed} m/s: the arcade bike leans less, {lean} rad");
    }
}

#[test]
fn the_arcade_stick_turns_in_proportion_and_both_ways_alike() {
    let spec = bike_with(ArcadeSteering::default());
    let (_, full, _) = steady(&spec, 30.0, 1.0, 8.0);
    let (_, half, _) = steady(&spec, 30.0, 0.5, 8.0);
    let (_, left, _) = steady(&spec, 30.0, -1.0, 8.0);
    assert!(half > 0.35 * full && half < 0.75 * full, "half stick {half} g, full {full} g");
    assert!((left - full).abs() < 0.1 * full, "left {left} g, right {full} g");
}

#[test]
fn a_flip_of_the_stick_is_quicker_than_the_original_slower_than_before_and_does_not_slow_with_the_speed() {
    let arcade = bike_with(ArcadeSteering::default());
    let at_20 = flip_time(&arcade, 20.0).expect("no fall at 20 m/s");
    let before_20 = flip_time(&bike_with(previous()), 20.0).expect("no fall at 20 m/s");
    let reference_20 = flip_time(&bike_with(ArcadeSteering::REFERENCE), 20.0).expect("no fall at 20 m/s");
    println!("flip to 80 percent at 20 m/s: before {before_20:.2} s, new {at_20:.2} s, original {reference_20:.2} s");
    assert!(before_20 < at_20 && at_20 < reference_20, "{before_20} < {at_20} < {reference_20}");
    for speed in [40.0_f32, 60.0] {
        let at = flip_time(&arcade, speed).expect("no fall");
        assert!(at < at_20 * 1.2 + 0.05, "{speed} m/s: {at} s against {at_20} s at 20 m/s");
    }
}

#[test]
fn an_arcade_bike_still_falls_when_it_is_thrown_over_or_tumbles() {
    let ground = FlatGround::new(0.0);
    let mut over = Bike::new(bike_with(ArcadeSteering::default()));
    assert!(over.place_moving(&ground, 0.0, 0.0, 2.0, 0.0, 20.0));
    over.body_mut().set_orientation(Quat::from_rotation_z(1.4));
    for _ in 0..30 {
        over.step(FIXED_STEP, &BikeInput::default(), &ground);
    }
    assert!(over.fallen(), "a bike on its side has fallen");
    let mut tumbling = Bike::new(bike_with(ArcadeSteering::default()));
    assert!(tumbling.place_moving(&ground, 0.0, 0.0, 2.0, 0.0, 20.0));
    tumbling.body_mut().angular_velocity = Vec3::new(0.0, 0.0, 60.0);
    tumbling.step(FIXED_STEP, &BikeInput::default(), &ground);
    assert!(tumbling.fallen(), "a roll of 60 rad/s is a tumble, not the flip of a rider");
}

#[test]
fn the_arcade_grip_scales_the_tyres_and_so_the_lean_limit_the_fall_limit_and_the_help_cap_together() {
    let reference = Bike::new(bike_with(ArcadeSteering::REFERENCE));
    let grippy = Bike::new(bike_with(ArcadeSteering { lateral_grip: 1.3, ..ArcadeSteering::REFERENCE }));
    let (a, b) = (&reference.spec().front.tyre, &grippy.spec().front.tyre);
    for load in [0.8_f32, 1.5, 3.0] {
        assert!((b.lateral_peak(load) / a.lateral_peak(load) - 1.3).abs() < 1e-4);
        assert!((b.lateral(load, 2.0, 10.0) / a.lateral(load, 2.0, 10.0) - 1.3).abs() < 1e-3);
    }
}
