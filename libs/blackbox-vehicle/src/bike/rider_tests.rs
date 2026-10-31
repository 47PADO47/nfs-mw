//! The rider as a part of the dynamics: his place follows the lean, the weight and the speed, the centre of gravity follows his
//! place, his body lags the bike's attitude, and the held-turn lean still reaches its target with him hanging off.

use glam::Vec3;

use super::{Bike, BikeInput, BikeSpec, RiderSpec};
use crate::FIXED_STEP;
use crate::ground::FlatGround;

/// The supermoto-like placement: the lateral offset puts the rider to the outside of the lean, 12 cm, and the lean alone asks
/// for it; the rider's body has the angular spring of the data.
fn hanging_spec(lateral: Vec3) -> BikeSpec {
    let mut spec = BikeSpec::example();
    spec.rider = RiderSpec {
        lateral_offset: lateral,
        lean_influence: 1.0,
        weight_influence: 0.0,
        correct_gravity: true,
        inertia_scale: Vec3::new(0.23, 0.28, 0.19),
        angular_k: 2500.0,
        angular_damp: 250.0,
        angular_damp_chassis_factor: 0.0,
        ..RiderSpec::default()
    };
    spec
}

fn ride(spec: BikeSpec, speed: f32, steer: f32, seconds: f32) -> Bike {
    let ground = FlatGround::new(0.0);
    let mut bike = Bike::new(spec);
    assert!(bike.place_moving(&ground, 0.0, 0.0, 2.0, 0.0, speed));
    let input = BikeInput { throttle: 0.15, steer, ..Default::default() };
    for _ in 0..(seconds / FIXED_STEP) as usize {
        bike.step(FIXED_STEP, &input, &ground);
    }
    bike
}

#[test]
fn leaning_into_a_turn_moves_the_combined_centre_of_gravity_sideways_by_the_riders_share() {
    let lateral = Vec3::new(-0.12, 0.0, 0.0);
    let straight = ride(hanging_spec(lateral), 20.0, 0.0, 2.0);
    let right = ride(hanging_spec(lateral), 20.0, 0.5, 3.0);
    let left = ride(hanging_spec(lateral), 20.0, -0.5, 3.0);
    let share = right.spec().chassis.rider_mass / right.spec().total_mass();
    let asked = right.rider_offset().x;
    assert!(asked < -0.02, "the rider is to the left of a right lean (the outside): {asked}");
    assert!((right.body().cog().x - straight.body().cog().x - asked * share).abs() < 5e-3, "{}", right.body().cog().x);
    assert!(right.body().cog().x < straight.body().cog().x - 0.005);
    // The mirror image.
    assert!((left.body().cog().x + right.body().cog().x - 2.0 * straight.body().cog().x).abs() < 8e-3);
    assert!((left.rider_offset().x + right.rider_offset().x).abs() < 8e-3);
    assert!((straight.rider_offset().x).abs() < 1e-3);
}

#[test]
fn the_lean_still_reaches_its_target_with_the_rider_hung_off_the_bike() {
    for side in [-1.0_f32, 1.0] {
        let bike = ride(hanging_spec(Vec3::new(-0.12, -0.04, 0.05)), 20.0, 0.5 * side, 4.0);
        assert!(bike.target_lean().abs() > 0.3, "{}", bike.target_lean());
        assert!((bike.lean() - bike.target_lean()).abs() < 0.06, "{} {}", bike.lean(), bike.target_lean());
        assert!(!bike.fallen());
        // He also sits a little lower and forward, the same on both sides.
        assert!(bike.rider_offset().y < -0.005 && bike.rider_offset().z > 0.0);
    }
}

#[test]
fn an_inside_hang_off_leans_the_rider_toward_the_turn_and_the_bike_stays_up() {
    let bike = ride(hanging_spec(Vec3::new(0.2, -0.05, 0.0)), 20.0, 0.5, 4.0);
    assert!(bike.rider_offset().x > 0.04, "{:?}", bike.rider_offset());
    assert!((bike.lean() - bike.target_lean()).abs() < 0.06 && !bike.fallen());
}

#[test]
fn the_lateral_weight_hangs_the_rider_off_without_any_lean() {
    let mut spec = hanging_spec(Vec3::new(0.2, 0.0, 0.0));
    spec.rider.weight_influence = 1.0;
    let ground = FlatGround::new(0.0);
    let mut bike = Bike::new(spec);
    assert!(bike.place_moving(&ground, 0.0, 0.0, 2.0, 0.0, 20.0));
    bike.set_weight_lateral(1.0);
    for _ in 0..(2.0 / FIXED_STEP) as usize {
        bike.step(FIXED_STEP, &BikeInput { throttle: 0.15, ..Default::default() }, &ground);
    }
    assert!(bike.rider_offset().x > 0.15, "{:?}", bike.rider_offset());
    bike.set_weight_lateral(f32::NAN);
    for _ in 0..(2.0 / FIXED_STEP) as usize {
        bike.step(FIXED_STEP, &BikeInput { throttle: 0.15, ..Default::default() }, &ground);
    }
    assert!(bike.rider_offset().x.abs() < 0.02);
}

#[test]
fn the_riders_body_lags_the_bikes_roll_and_the_reaction_does_not_throw_the_bike_over() {
    let ground = FlatGround::new(0.0);
    let mut bike = Bike::new(hanging_spec(Vec3::ZERO));
    assert!(bike.place_moving(&ground, 0.0, 0.0, 2.0, 0.0, 20.0));
    let mut most = 0.0_f32;
    for _ in 0..(1.0 / FIXED_STEP) as usize {
        bike.step(FIXED_STEP, &BikeInput { throttle: 0.15, steer: 1.0, ..Default::default() }, &ground);
        most = most.max(bike.rider_rotation().roll_right().abs());
    }
    // He is turned against the roll while it changes, and the bike still leans the way it was asked to.
    assert!(most > 0.005, "{most}");
    assert!(bike.lean() > 0.15 && !bike.fallen(), "{}", bike.lean());
    for _ in 0..(4.0 / FIXED_STEP) as usize {
        bike.step(FIXED_STEP, &BikeInput { throttle: 0.15, steer: 1.0, ..Default::default() }, &ground);
    }
    assert!(bike.rider_rotation().roll_right().abs() < 0.03, "settled: {}", bike.rider_rotation().roll_right());
}

#[test]
fn a_bike_without_rider_data_behaves_as_before_and_placing_it_resets_the_rider() {
    let ground = FlatGround::new(0.0);
    let mut bike = Bike::new(BikeSpec::example());
    assert!(bike.place_moving(&ground, 0.0, 0.0, 2.0, 0.0, 20.0));
    for _ in 0..120 {
        bike.step(FIXED_STEP, &BikeInput { throttle: 0.15, steer: 0.5, ..Default::default() }, &ground);
    }
    // The rider help may move his weight along the bike at a start; nothing in the data moves him sideways.
    assert!(bike.rider_offset().x.abs() < 0.02, "no lateral data: {:?}", bike.rider_offset());
    assert_eq!(*bike.rider_rotation(), super::RiderRotation::default());
    bike.set_weight_lateral(1.0);
    assert!(bike.place_moving(&ground, 0.0, 0.0, 2.0, 0.0, 0.0));
    assert_eq!(bike.rider_offset(), Vec3::ZERO);
}

#[test]
fn tucking_in_lowers_the_air_drag_by_the_riders_drag_curve() {
    let drift = |tuck: f32| {
        let mut spec = BikeSpec::example();
        spec.rider.drag_curve = vec![(0.0, 1.2), (1.0, 0.65)];
        spec.rider.drag_weight_curve = vec![(0.0, 0.1)];
        spec.rider.drag_scale = 1.25;
        let ground = FlatGround::new(0.0);
        let mut bike = Bike::new(spec);
        assert!(bike.place_moving(&ground, 0.0, 0.0, 2.0, 0.0, 40.0));
        bike.set_tuck(tuck);
        for _ in 0..60 {
            bike.step(FIXED_STEP, &BikeInput::default(), &ground);
        }
        bike.forward_speed()
    };
    assert!(drift(1.0) > drift(0.0) + 0.05, "{} {}", drift(1.0), drift(0.0));
    assert_eq!(drift(f32::NAN), drift(0.0));
}
