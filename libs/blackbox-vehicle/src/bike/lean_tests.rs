use glam::Vec3;

use super::*;
use crate::bike::BikeSpec;

const DT: f32 = 1.0 / 60.0;

fn input(stick: f32, lean: f32, speed: f32) -> LeanInput {
    LeanInput {
        stick,
        speed,
        lean,
        lean_rate: 0.0,
        lateral_grip: 1.3,
        terrain_slope: 0.0,
        lateral_g: 0.0,
        heading_error: 0.0,
        pitch_cut: (0.0, 0.0),
        roll_inertia: 47.8,
        cg_height: 0.8,
        target_cap: f32::MAX,
        dt: DT,
    }
}

#[test]
fn the_lean_limit_follows_the_grip_and_is_capped_by_the_spec() {
    let steering = BikeSpec::example().steering;
    let weak = lean_limit(&steering, &LeanInput { lateral_grip: 0.3, ..input(1.0, 0.0, 30.0) });
    assert!((weak - 0.3_f32.atan()).abs() < 1e-5);
    let strong = lean_limit(&steering, &LeanInput { lateral_grip: 50.0, ..input(1.0, 0.0, 30.0) });
    assert!(strong <= steering.max_lean + 1e-6);
}

#[test]
fn the_lean_limit_falls_at_a_standstill_with_the_lateral_g_and_in_a_skid() {
    let steering = BikeSpec::example().steering;
    let base = lean_limit(&steering, &input(1.0, 0.0, 30.0));
    assert!((lean_limit(&steering, &input(1.0, 0.0, 0.0)) - steering.lean.stopped_limit).abs() < 1e-6);
    assert!(lean_limit(&steering, &input(1.0, 0.0, 4.0)) < base);
    assert!(lean_limit(&steering, &LeanInput { lateral_g: 1.1, ..input(1.0, 0.0, 30.0) }) < base * 0.95);
    assert!(lean_limit(&steering, &LeanInput { heading_error: 0.9, ..input(1.0, 0.0, 30.0) }) < base);
}

#[test]
fn a_wheelie_pulls_the_lean_limit_down() {
    let steering = BikeSpec::example().steering;
    let base = lean_limit(&steering, &input(1.0, 0.0, 30.0));
    let cut = lean_limit(&steering, &LeanInput { pitch_cut: (0.02, 1.0), ..input(1.0, 0.0, 30.0) });
    assert!((cut - 0.02).abs() < 1e-6 && cut < base);
}

#[test]
fn an_upright_bike_with_the_stick_centred_asks_for_nothing() {
    let steering = BikeSpec::example().steering;
    let plan = Lean::new().plan(&steering, &input(0.0, 0.0, 20.0));
    assert!(plan.acceleration.abs() < 1e-4 && plan.front_force.abs() < 1e-2, "{plan:?}");
}

#[test]
fn leaning_right_asks_first_for_a_push_to_the_left_and_nothing_once_the_lean_is_held() {
    let steering = BikeSpec::example().steering;
    let mut lean = Lean::new();
    let first = lean.plan(&steering, &input(1.0, 0.0, 20.0));
    assert!(first.acceleration > 0.0 && first.front_force < 0.0, "{first:?}");
    // Held at the target lean the acceleration is gone and so is the force: the reference asks the front for no force of the
    // turn, the lean settles a little off its target and the error asks for it (`ride6-chassis-round7.md`, section 2).
    let mut settled = first;
    for _ in 0..600 {
        let at = settled.target;
        settled = lean.plan(&steering, &input(1.0, at, 20.0));
    }
    assert!(settled.acceleration.abs() < 0.5, "{settled:?}");
    assert!(settled.front_force.abs() < 50.0, "{settled:?}");
}

#[test]
fn a_bike_leaning_past_its_target_is_pushed_back() {
    let steering = BikeSpec::example().steering;
    let plan = Lean::new().plan(&steering, &input(0.0, 0.3, 20.0));
    assert!(plan.acceleration < 0.0 && plan.front_force > 0.0, "{plan:?}");
}

#[test]
fn the_slip_for_a_force_has_the_sign_of_the_countersteer() {
    let tyre = BikeSpec::example().front.tyre;
    let left = slip_for_force(&tyre, 1.0, 0.0, -300.0);
    let right = slip_for_force(&tyre, 1.0, 0.0, 300.0);
    assert!(left > 0.0 && right < 0.0, "{left} {right}");
    // The formula gives back the force asked for.
    let back = tyre.lateral(1.0, left.to_degrees(), 0.0);
    assert!((back - 300.0).abs() < 5.0, "{back}");
}

#[test]
fn the_slip_for_a_force_beyond_the_peak_is_the_slip_of_the_peak() {
    let tyre = BikeSpec::example().front.tyre;
    let a = slip_for_force(&tyre, 1.0, 0.0, -1.0e6);
    let b = slip_for_force(&tyre, 1.0, 0.0, -2.0e6);
    assert!((a - b).abs() < 1e-4 && a > 0.0);
}

#[test]
fn the_closed_form_handlebar_steers_the_wheel_to_the_direction_of_travel_less_the_slip() {
    let axis = Vec3::new(0.0, 26.0_f32.to_radians().cos(), -26.0_f32.to_radians().sin());
    // Straight ahead with no slip: no steering. A push to the left (slip) first steers left, and a patch moving to the right
    // steers right.
    assert!(handlebar_angle(Vec3::new(0.0, 0.0, 20.0), axis, 0.0).abs() < 1e-5);
    assert!(handlebar_angle(Vec3::new(0.0, 0.0, 20.0), axis, 0.05) < -0.03);
    assert!(handlebar_angle(Vec3::new(1.0, 0.0, 20.0), axis, 0.0) > 0.03);
    // Mirrored.
    let right = handlebar_angle(Vec3::new(1.0, 0.0, 20.0), axis, 0.02);
    let left = handlebar_angle(Vec3::new(-1.0, 0.0, 20.0), axis, -0.02);
    assert!((right + left).abs() < 1e-5);
    assert_eq!(handlebar_angle(Vec3::ZERO, axis, 0.1), 0.0);
}

#[test]
fn the_handlebar_goes_to_the_stick_at_a_walking_pace_and_to_the_controller_at_speed() {
    let steering = BikeSpec::example().steering;
    assert_eq!(blend_to_stick(&steering, 0.5, 0.5, -0.3), 0.5 * steering.max_angle);
    assert!((blend_to_stick(&steering, 10.0, 0.5, -0.3) + 0.3).abs() < 1e-6);
    let half = blend_to_stick(&steering, 3.0, 0.0, 0.4);
    assert!((half - 0.2).abs() < 1e-6);
}

#[test]
fn the_block_fall_cap_holds_the_target_lean_back() {
    let steering = BikeSpec::example().steering;
    let mut lean = Lean::new();
    let mut plan = LeanPlan::default();
    for _ in 0..300 {
        plan = lean.plan(&steering, &LeanInput { target_cap: 0.2, ..input(1.0, 0.0, 20.0) });
    }
    assert!(plan.capped && (plan.target - 0.2).abs() < 1e-6, "{plan:?}");
    let free = Lean::new().plan(&steering, &input(0.0, 0.0, 20.0));
    assert!(!free.capped);
}

#[test]
fn the_help_factor_is_full_at_a_standstill_and_the_tuned_value_beyond_the_stopped_range() {
    use super::help::{HelpInput, help_factor};
    let lean = BikeSpec::example().steering.lean;
    let at = |speed: f32| help_factor(&lean, &HelpInput { speed, heading_error: 0.0, pedals: 0.0 });
    assert!((at(0.0) - 1.0).abs() < 1e-6);
    assert!(at(1.0) < 1.0 && at(1.0) > 0.2, "{}", at(1.0));
    assert!((at(10.0 / 3.6) - 0.2).abs() < 1e-5 && (at(30.0) - 0.2).abs() < 1e-5);
    // Without the stopped blend the tuned curve stands alone.
    let mut plain = lean;
    plain.help_extra.stopped_range = [0.0, 0.0];
    assert!(help_factor(&plain, &HelpInput { speed: 0.0, heading_error: 0.0, pedals: 0.0 }) < 1e-6);
}

#[test]
fn a_skid_cuts_the_help_factor_unless_the_rider_brakes() {
    use super::help::{HelpInput, help_factor};
    let lean = BikeSpec::example().steering.lean;
    let skid =
        |pedals: f32| help_factor(&lean, &HelpInput { speed: 10.0, heading_error: 50.0_f32.to_radians(), pedals });
    assert!(skid(0.2) < 1e-5, "{}", skid(0.2));
    assert!((skid(-0.5) - 0.2).abs() < 1e-5, "{}", skid(-0.5));
    let slight = help_factor(&lean, &HelpInput { speed: 10.0, heading_error: 20.0_f32.to_radians(), pedals: 0.2 });
    assert!(slight > 0.1 && slight < 0.2, "{slight}");
}

#[test]
fn the_help_is_a_couple_when_slow_and_a_force_when_fast() {
    use super::help::{balance, couple};
    let lean = BikeSpec::example().steering.lean;
    assert_eq!(balance(&lean, 0.5), 0.0);
    assert_eq!(balance(&lean, 2.0), 0.0);
    assert!((balance(&lean, 0.5 * (10.0 + 30.0) / 3.6) - 0.5).abs() < 1e-5);
    assert_eq!(balance(&lean, 30.0), 1.0);
    // The couple is the moment of the whole force about the centre of gravity: all of it at balance 0, none at 1.
    let (point, cog, force) = (Vec3::new(0.0, 0.0, 0.7), Vec3::new(0.0, 0.8, 0.0), Vec3::new(100.0, 0.0, 0.0));
    let moment = couple(point, cog, force, 0.0);
    assert!((moment - (point - cog).cross(force)).length() < 1e-4);
    assert!((moment.z - 80.0).abs() < 1e-3 && (moment.y - 70.0).abs() < 1e-3, "{moment}");
    assert_eq!(couple(point, cog, force, 1.0), Vec3::ZERO);
    assert!((couple(point, cog, force, 0.25) - moment * 0.75).length() < 1e-4);
}

#[test]
fn a_standing_bike_leaned_over_is_stood_up_by_the_help_alone() {
    use crate::FIXED_STEP;
    use crate::bike::{Bike, BikeInput};
    use crate::ground::FlatGround;
    let ground = FlatGround::new(0.0);
    let mut bike = Bike::new(BikeSpec::example());
    assert!(bike.place_on_ground(&ground, 0.0, 0.0, 2.0, 0.0));
    bike.body_mut().set_orientation(glam::Quat::from_rotation_z(0.08));
    let mut worst: f32 = 0.0;
    for _ in 0..(3.0 / FIXED_STEP) as usize {
        bike.step(FIXED_STEP, &BikeInput::default(), &ground);
        worst = worst.max(bike.lean().abs());
    }
    // No roll torque exists but the moment of the lean help (below 10 km/h it is all moment): it takes the bike from 4.6 degrees to
    // upright without jitter and without moving it (the 60 Hz chatter and the 1.5 degree residue of the first build were the step).
    assert!(!bike.fallen() && worst < 0.09, "worst {worst}");
    assert!(bike.lean().abs() < 0.002, "{}", bike.lean());
    assert!(bike.position().x.abs() < 0.05 && bike.velocity().length() < 0.01);
    // With no help at a standstill the same bike is not stood up.
    let mut spec = BikeSpec::example();
    spec.steering.lean.help_extra.stopped_range = [0.0, 0.0];
    let mut helpless = Bike::new(spec);
    assert!(helpless.place_on_ground(&ground, 0.0, 0.0, 2.0, 0.0));
    helpless.body_mut().set_orientation(glam::Quat::from_rotation_z(0.08));
    for _ in 0..(3.0 / FIXED_STEP) as usize {
        helpless.step(FIXED_STEP, &BikeInput::default(), &ground);
    }
    assert!(helpless.lean().abs() > 0.02 || helpless.fallen(), "{}", helpless.lean());
}

fn road_stability() -> crate::bike::StabilitySpec {
    // The default, naked and sport handlers: 0.05 up to 80 km/h falling to 0 at 120, less with the lean, cut by the throttle.
    crate::bike::StabilitySpec {
        speed: vec![(0.0, 0.05), (80.0 / 3.6, 0.05), (120.0 / 3.6, 0.0)],
        lean_influence: -1.0,
        lean_sensitivity: 1.0,
        throttle_reduction: vec![(0.0, 0.0), (0.05 / 3.6, 1.0)],
        ..Default::default()
    }
}

#[test]
fn the_stability_factor_follows_the_speed_the_lean_and_the_throttle() {
    use super::help::{StabilityInput, desired_yaw_rate, stability_acceleration, stability_factor};
    let spec = road_stability();
    let at = |speed: f32, normalised_lean: f32, throttle: f32| {
        stability_factor(&spec, &StabilityInput { speed, lean: 0.3, normalised_lean, throttle, brake: 0.0 })
    };
    assert!((at(10.0, 0.0, 0.0) - 0.05).abs() < 1e-6);
    assert!((at(10.0, 0.5, 0.0) - 0.025).abs() < 1e-6, "less with the lean");
    assert!(at(10.0, 1.0, 0.0) < 1e-6);
    assert!((at(10.0, 0.0, 0.5) - 0.025).abs() < 1e-6, "half the throttle takes half away");
    assert_eq!(at(10.0, 0.0, 1.0), 0.0, "the full throttle takes it all");
    assert!(at(33.0, 0.0, 0.0) > 0.0 && at(33.0, 0.0, 0.0) < 0.05 && at(40.0, 0.0, 0.0) == 0.0);
    assert_eq!(at(10.0, 0.0, 0.0), at(10.0, 0.0, 0.0));
    // The yaw rate of a steady turn is g tan(lean) / v; to the right for a lean to the right.
    let wanted = desired_yaw_rate(&spec, 0.4, 20.0);
    assert!((wanted - 9.81 * 0.4_f32.tan() / 20.0).abs() < 1e-5 && wanted > 0.0);
    let state = StabilityInput { speed: 20.0, lean: 0.4, normalised_lean: 0.0, throttle: 0.0, brake: 0.0 };
    // Below the speed range there is none; inside it the factor pulls the yaw rate to the wanted one.
    assert_eq!(stability_acceleration(&spec, 0.05, &StabilityInput { speed: 2.0, ..state }, 0.0, 1.0 / 60.0), 0.0);
    let pull = stability_acceleration(&spec, 0.05, &state, 0.0, 1.0 / 60.0);
    assert!((pull - 0.05 * wanted * 60.0).abs() < 1e-4, "{pull}");
    assert!(stability_acceleration(&spec, 0.05, &state, wanted * 2.0, 1.0 / 60.0) < 0.0);
}

#[test]
fn the_stability_help_holds_the_yaw_rate_of_a_coasting_turn_nearer_to_the_one_the_lean_needs() {
    use crate::FIXED_STEP;
    use crate::bike::{Bike, BikeInput};
    use crate::ground::FlatGround;
    let ground = FlatGround::new(0.0);
    let error_after = |stability: bool| {
        let mut spec = BikeSpec::example();
        if stability {
            spec.steering.lean.stability = road_stability();
            spec.steering.lean.stability.uses_terrain_normal = true;
        }
        let mut bike = Bike::new(spec);
        assert!(bike.place_moving(&ground, 0.0, 0.0, 2.0, 0.0, 20.0));
        let mut worst: f32 = 0.0;
        for i in 0..(5.0 / FIXED_STEP) as usize {
            bike.step(FIXED_STEP, &BikeInput { steer: 0.5, ..Default::default() }, &ground);
            assert!(!bike.fallen());
            if i > 150 {
                let yaw_right = bike.body().angular_velocity.y;
                worst = worst.max((yaw_right - 9.81 * bike.lean().tan() / bike.forward_speed().max(1.0)).abs());
            }
        }
        worst
    };
    let (without, with) = (error_after(false), error_after(true));
    assert!(with < without, "with {with} without {without}");
}
