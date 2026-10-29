//! The rider leaving the bike in a fall (`release.rs`).

use glam::{Quat, Vec3};

use super::{Bike, BikeInput, BikeSpec, FallKind, VELOCITY_FRAMES};
use crate::FIXED_STEP;
use crate::ground::FlatGround;

fn bike_at(speed: f32) -> (Bike, FlatGround) {
    let ground = FlatGround::new(0.0);
    let mut bike = Bike::new(BikeSpec::example());
    assert!(bike.place_moving(&ground, 0.0, 0.0, 2.0, 0.0, speed));
    (bike, ground)
}

fn run(bike: &mut Bike, ground: &FlatGround, seconds: f32) {
    for _ in 0..(seconds / FIXED_STEP) as usize {
        bike.step(FIXED_STEP, &BikeInput::default(), ground);
    }
}

/// Step until the rider is off (the lean on the ground must last two steps to count).
fn until_off(bike: &mut Bike, ground: &FlatGround) {
    for _ in 0..4 {
        if bike.rider_off() {
            return;
        }
        bike.step(FIXED_STEP, &BikeInput::default(), ground);
    }
}

/// Settle at `speed`, then roll the bike over onto its side.
fn fall_at(speed: f32) -> (Bike, FlatGround) {
    let (mut bike, ground) = bike_at(speed);
    run(&mut bike, &ground, 1.0);
    assert!(!bike.fallen() && !bike.rider_off());
    bike.body_mut().set_orientation(Quat::from_rotation_z(1.4));
    until_off(&mut bike, &ground);
    (bike, ground)
}

#[test]
fn the_rider_is_handed_over_once_at_the_fall_and_the_bike_loses_his_mass() {
    let (mut bike, ground) = bike_at(20.0);
    run(&mut bike, &ground, 1.0);
    let total = bike.body().mass();
    assert!((total - bike.spec().total_mass()).abs() < 1e-3);
    assert!(bike.take_rider_release().is_none() && bike.fall_kind().is_none());
    bike.body_mut().set_orientation(Quat::from_rotation_z(1.4));
    until_off(&mut bike, &ground);
    assert!(bike.fallen() && bike.rider_off());
    let release = bike.take_rider_release().expect("the rider is released");
    assert!(bike.take_rider_release().is_none(), "once");
    assert_eq!(bike.fall_kind(), Some(release.kind));
    // The bike goes on with its own mass and its own centre of gravity.
    assert!((bike.body().mass() - bike.spec().chassis.mass).abs() < 1e-3);
    assert!(bike.body().mass() < total - bike.spec().chassis.rider_mass * 0.9);
    let cog = super::rider::bike_cog(&bike.spec().chassis);
    assert!(bike.body().cog().distance(cog) < 1e-6);
}

#[test]
fn the_rider_inherits_the_speed_the_bike_had_a_few_steps_before_the_fall() {
    let (mut bike, ground) = bike_at(25.0);
    run(&mut bike, &ground, 2.0);
    let before = bike.velocity();
    bike.body_mut().set_orientation(Quat::from_rotation_z(1.4));
    until_off(&mut bike, &ground);
    let release = bike.take_rider_release().expect("released");
    assert_eq!(VELOCITY_FRAMES, 2);
    // He moves with the bike: along its heading at about its speed, and he is near the bike (his place is above its origin).
    assert!((release.velocity - before).length() < 3.0, "{:?} vs {before:?}", release.velocity);
    assert!(release.velocity.z > 20.0);
    assert!(release.position.distance(bike.position()) < 2.0, "{:?} {:?}", release.position, bike.position());
}

#[test]
fn the_bike_alone_keeps_the_velocity_of_its_own_centre_of_gravity_through_the_release() {
    let (mut bike, ground) = bike_at(20.0);
    run(&mut bike, &ground, 1.0);
    bike.body_mut().set_orientation(Quat::from_rotation_z(1.4));
    bike.body_mut().angular_velocity = Vec3::new(0.0, 0.0, 1.0);
    let point =
        |bike: &Bike| bike.body().position + bike.body().rotation() * super::rider::bike_cog(&bike.spec().chassis);
    // The fall is noticed in the response half of a step: nothing has moved in that call, so the point and its velocity are as they
    // were at its start.
    for _ in 0..8 {
        let (at, moving) = (point(&bike), bike.body().point_velocity(point(&bike)));
        // One sub-step: a longer response moves the body between its sub-steps (`choices::MAX_SUBSTEP`).
        bike.respond(super::choices::MAX_SUBSTEP, &BikeInput::default(), &ground);
        if !bike.rider_off() {
            continue;
        }
        assert!(point(&bike).distance(at) < 1e-5);
        let now = bike.body().point_velocity(point(&bike));
        assert!(now.distance(moving) < 1e-3, "{now:?} vs {moving:?}");
        return;
    }
    panic!("the rider never left");
}

#[test]
fn a_rider_who_is_off_holds_nothing_the_bike_does_not_drive_or_steer() {
    let (mut bike, ground) = fall_at(20.0);
    let input = BikeInput { throttle: 1.0, front_brake: 1.0, rear_brake: 1.0, steer: 1.0 };
    for _ in 0..30 {
        bike.step(FIXED_STEP, &input, &ground);
    }
    assert!(bike.rider_off());
    // The bars swing back to the middle (nobody holds them) and the engine is not driven.
    assert!(bike.steer_angle().abs() < 0.05, "{}", bike.steer_angle());
    assert!(bike.body().linear_velocity.is_finite());
    // His weight and spring act no more: the centre of gravity stays where the bike's own is.
    let cog = super::rider::bike_cog(&bike.spec().chassis);
    assert!(bike.body().cog().distance(cog) < 1e-6);
}

#[test]
fn placing_the_bike_again_puts_the_rider_back_with_his_mass() {
    let (mut bike, ground) = fall_at(15.0);
    assert!(bike.rider_off());
    assert!(bike.place_on_ground(&ground, 0.0, 0.0, 2.0, 0.0));
    assert!(!bike.rider_off() && !bike.fallen() && bike.fall_kind().is_none());
    assert!((bike.body().mass() - bike.spec().total_mass()).abs() < 1e-3);
    let combined = super::rider::combined_cog(&bike.spec().chassis, bike.spec().rider.base_offset, Vec3::ZERO);
    assert!(bike.body().cog().distance(combined) < 1e-6);
    // ... and he can fall again, with a release of his own.
    run(&mut bike, &ground, 0.5);
    assert!(!bike.rider_off());
    bike.body_mut().set_orientation(Quat::from_rotation_z(1.4));
    until_off(&mut bike, &ground);
    assert!(bike.rider_off());
}

#[test]
fn a_lean_over_the_limit_is_a_low_side_and_a_velocity_against_the_heading_is_a_high_side() {
    // Rolled to 69 degrees (past the grip-following limit, about 66): the wheels stay on the road for the checks, which a roll
    // to 80 degrees does not leave them for once the step is divided into sub-steps.
    let (mut low, ground) = bike_at(20.0);
    run(&mut low, &ground, 1.0);
    low.body_mut().set_orientation(Quat::from_rotation_z(1.2));
    until_off(&mut low, &ground);
    assert_eq!(low.fall_kind(), Some(FallKind::LowSide));
    // Turn the bike across its velocity while it stays upright and fast enough to be tested.
    // (The heading test needs a speed along the bike over its minimum, so a fast bike turned to 86 degrees from its velocity.)
    let (mut bike, ground) = bike_at(60.0);
    run(&mut bike, &ground, 0.5);
    let turn = Quat::from_rotation_y(1.5);
    let turned = turn * bike.orientation();
    bike.body_mut().set_orientation(turned);
    until_off(&mut bike, &ground);
    assert!(bike.fallen());
    assert_eq!(bike.fall_kind(), Some(FallKind::HighSide));
    // Without a high side lean limit (0) the same fall is a low side.
    let mut spec = BikeSpec::example();
    spec.fall.extra.highside_lean = 0.0;
    let mut plain = Bike::new(spec);
    assert!(plain.place_moving(&ground, 0.0, 0.0, 2.0, 0.0, 60.0));
    run(&mut plain, &ground, 0.5);
    let turned = turn * plain.orientation();
    plain.body_mut().set_orientation(turned);
    until_off(&mut plain, &ground);
    assert_eq!(plain.fall_kind(), Some(FallKind::LowSide));
}
