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
