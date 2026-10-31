//! The rider leaving a bike in a fall, through the same `CarSim` a car uses.

use std::sync::Arc;

use blackbox_vehicle::FlatGround;
use blackbox_vehicle::bike::BikeSpec;
use glam::Vec3;
use nfsmw_data::car::physics::{CarBounds, CarPhysics};

use super::super::{CarSim, Spawn};
use crate::scenes::world::drive::input::DriveInput;

const BOUNDS: CarBounds = CarBounds { half_dimensions: Vec3::new(0.42, 0.64, 1.06), pivot: Vec3::new(0.0, 0.64, 0.0) };

fn bike_sim() -> CarSim {
    CarSim::new(CarPhysics::from_bike(Arc::new(BikeSpec::example()), 0.0, BOUNDS), [0.32; 4])
}

fn spawn() -> Spawn {
    Spawn { position: Vec3::new(100.0, 50.0, 5.0), heading: 0.0 }
}

fn step(sim: &mut CarSim, ground: &FlatGround, seconds: f32) {
    for _ in 0..(seconds * 60.0) as usize {
        let _ = sim.step(&DriveInput::default(), ground, None);
    }
}

/// A bike at 25 m/s that goes over onto its side.
fn falling() -> (CarSim, FlatGround) {
    let ground = FlatGround::new(5.0);
    let mut sim = bike_sim();
    assert!(sim.place_moving(&ground, spawn(), 25.0));
    step(&mut sim, &ground, 1.0);
    assert!(!sim.rider_off());
    // (4 rad/s no longer tips it: the lean help, run at the game's sub-step, rights the bike.)
    sim.vehicle.body_mut().angular_velocity = Vec3::new(0.0, 0.0, 10.0);
    (sim, ground)
}

#[test]
fn a_car_never_loses_a_rider() {
    let mut car = CarSim::new(CarPhysics::from_stats(&Default::default(), BOUNDS), [0.2; 4]);
    assert!(!car.rider_off() && car.take_rider_release().is_none());
}

#[test]
fn a_bike_that_goes_over_hands_its_rider_over_once_and_slides_on_without_him() {
    let (mut sim, ground) = falling();
    let mass_with_rider = sim.vehicle.body().mass();
    let mut release = None;
    let mut start = sim.pose().position;
    let mut speed_at_fall = 0.0;
    for _ in 0..240 {
        let _ = sim.step(&DriveInput::default(), &ground, None);
        if let Some(thrown) = sim.take_rider_release() {
            assert!(release.is_none(), "once");
            speed_at_fall = sim.vehicle.body().linear_velocity.length();
            start = sim.pose().position;
            release = Some(thrown);
        }
        if release.is_some() {
            break;
        }
    }
    let release = release.expect("the rider was thrown");
    assert!(sim.rider_off());
    assert!(release.velocity.length() > 15.0, "he leaves at the bike's speed: {:?}", release.velocity);
    // The double weighs what the bike does without him from the next step on.
    step(&mut sim, &ground, 0.1);
    assert!(sim.vehicle.body().mass() < mass_with_rider - 40.0, "{} {mass_with_rider}", sim.vehicle.body().mass());
    // The bike slides on, with nothing but the ground's friction to slow it: it covers metres on its side in the next second.
    step(&mut sim, &ground, 1.0);
    let travelled = sim.pose().position.truncate().distance(start.truncate());
    assert!(travelled > 8.0, "it slid {travelled} m from {speed_at_fall} m/s");
    assert!(sim.vehicle.body().linear_velocity.length() > 1.0 || speed_at_fall < 5.0, "it is still sliding");
    assert!(sim.is_finite());
}

#[test]
fn the_bike_is_stood_up_with_its_rider_when_it_has_lain_still() {
    let (mut sim, ground) = falling();
    step(&mut sim, &ground, 25.0);
    assert!(!sim.rider_off(), "stood up again");
    assert!(sim.up_dot() > 0.95);
    // The double has its rider's mass again.
    let total = BikeSpec::example().total_mass();
    assert!((sim.vehicle.body().mass() - total).abs() < 1.0, "{} {total}", sim.vehicle.body().mass());
    // ... and he can fall again.
    assert!(sim.place_moving(&ground, spawn(), 25.0));
    step(&mut sim, &ground, 1.0);
    sim.vehicle.body_mut().angular_velocity = Vec3::new(0.0, 0.0, 10.0);
    step(&mut sim, &ground, 2.0);
    assert!(sim.rider_off() || sim.take_rider_release().is_some());
}

#[test]
fn the_bike_stays_down_and_ignores_the_controls_until_the_rider_has_got_up() {
    let (mut sim, ground) = falling();
    // The rider is not ready (he is on the ground, getting up): the bike lies still, and waits.
    sim.set_rider_ready(false);
    step(&mut sim, &ground, 12.0);
    assert!(sim.rider_off(), "still down after twelve seconds");
    assert!(sim.bike_waits_for_rider(), "it has lain still and waits for him");
    // Full throttle and the stick: a fallen bike does not take them.
    let held = DriveInput { throttle: 1.0, steer: 1.0, ..DriveInput::default() };
    let at = sim.pose().position;
    for _ in 0..180 {
        let _ = sim.step(&held, &ground, None);
    }
    assert!(sim.rider_off(), "the controls did not stand the bike up");
    assert!(sim.pose().position.truncate().distance(at.truncate()) < 0.5, "it did not drive off");
    // He is up: the bike is stood up with him.
    sim.set_rider_ready(true);
    step(&mut sim, &ground, 0.5);
    assert!(!sim.rider_off() && !sim.bike_waits_for_rider(), "stood up");
    assert!(sim.up_dot() > 0.95);
}

#[test]
fn a_bike_put_back_by_hand_does_not_wait_for_the_rider() {
    let (mut sim, ground) = falling();
    sim.set_rider_ready(false);
    step(&mut sim, &ground, 6.0);
    assert!(sim.rider_off());
    // The reset button places the bike again.
    assert!(sim.place_moving(&ground, spawn(), 0.0));
    assert!(!sim.rider_off() && !sim.bike_waits_for_rider());
}
