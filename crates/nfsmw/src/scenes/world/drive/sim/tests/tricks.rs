//! Wheelies and stoppies belong to bikes: the rider's weight reaches a bike's dynamics, and a car's never sees it.

use std::sync::Arc;

use blackbox_vehicle::bike::BikeSpec;
use nfsmw_data::car::physics::BikePhysics;

use super::*;

/// A bike on the same terms as `sim()`.
fn bike_sim() -> CarSim {
    let spec = VehicleSpec::example();
    let bounds = CarBounds { half_dimensions: Vec3::new(0.42, 0.64, 1.06), pivot: Vec3::new(0.0, 0.64, 0.05) };
    CarSim::new(
        CarPhysics {
            effect_source: 0,
            tire_integration: Default::default(),
            jump_config: Default::default(),
            spec,
            bounds,
            walls: WallSpec::default(),
            geometry: None,
            events: None,
            reactions: None,
            bike: Some(BikePhysics { spec: Arc::new(BikeSpec::example()), origin_x: 0.0 }),
        },
        [0.2; 4],
    )
}

fn pitch_after(weight: f32, seconds: f32) -> f32 {
    let ground = FlatGround::new(5.0);
    let mut bike = bike_sim();
    assert!(bike.is_bike() && bike.place_moving(&ground, spawn_at(0.2), 6.0));
    let input = DriveInput { throttle: 1.0, weight, ..Default::default() };
    let mut highest: f32 = 0.0;
    for _ in 0..(seconds * 60.0) as usize {
        let _ = bike.step(&input, &ground, None);
        highest = highest.max(bike.rider_sample().expect("a bike").pitch_up);
    }
    highest
}

#[test]
fn the_riders_weight_back_with_the_throttle_lifts_a_bikes_front_wheel() {
    assert!(pitch_after(-1.0, 3.0) > 0.3, "weight back: {}", pitch_after(-1.0, 3.0));
    assert!(pitch_after(0.0, 3.0) < 0.3, "weight neutral: {}", pitch_after(0.0, 3.0));
}

#[test]
fn a_car_ignores_the_riders_weight_and_never_wheelies_by_it() {
    let ground = FlatGround::new(5.0);
    let run = |weight: f32| {
        let mut car = sim();
        assert!(!car.is_bike() && car.rider_sample().is_none());
        assert!(car.place_moving(&ground, spawn_at(0.2), 6.0));
        for _ in 0..240 {
            let _ = car.step(&DriveInput { throttle: 1.0, weight, ..Default::default() }, &ground, None);
        }
        let body = car.vehicle.body();
        (body.position, body.orientation, body.linear_velocity, body.angular_velocity, car.vehicle.rpm())
    };
    assert_eq!(run(-1.0), run(0.0));
    assert_eq!(run(1.0), run(0.0));
}

#[test]
fn the_weight_never_reaches_the_vehicle_input() {
    let with = DriveInput { throttle: 0.4, steer: 0.2, weight: -1.0, ..Default::default() };
    let without = DriveInput { weight: 0.0, ..with };
    assert_eq!(format!("{:?}", with.to_vehicle()), format!("{:?}", without.to_vehicle()));
}

#[test]
fn the_riders_abs_level_cycles_on_a_bike_and_a_car_has_none() {
    let mut bike = bike_sim();
    let level = |sim: &CarSim| sim.ride.as_ref().expect("a bike").bike.abs_level();
    assert_eq!(level(&bike), 1.0);
    let seen: Vec<f32> = (0..5).filter_map(|_| bike.cycle_abs_level()).collect();
    assert_eq!(seen, vec![0.75, 0.5, 0.25, 0.0, 1.0]);
    assert_eq!(level(&bike), 1.0);
    assert_eq!(sim().cycle_abs_level(), None, "a car has no ABS setting to cycle");
}

#[test]
fn a_bikes_abs_level_changes_how_hard_the_front_brake_can_work() {
    let ground = FlatGround::new(5.0);
    let slowed = |cycles: usize| {
        let mut bike = bike_sim();
        for _ in 0..cycles {
            bike.cycle_abs_level();
        }
        assert!(bike.place_moving(&ground, spawn_at(0.2), 25.0));
        let input = DriveInput { brake: 1.0, ..Default::default() };
        for _ in 0..60 {
            let _ = bike.step(&input, &ground, None);
        }
        25.0 - bike.rider_sample().expect("a bike").speed
    };
    assert!(slowed(4) > slowed(0) + 1.0, "level 0 brakes harder than level 1");
}
