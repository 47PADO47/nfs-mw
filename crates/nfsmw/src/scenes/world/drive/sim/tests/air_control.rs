//! The player's air control (`docs/specs/air-control.md`) through the car sim: a car and a bike with no wheel on the ground turn by the stick when
//! the `air_control` setting is on, for the player only, and never with a wheel on the road.

use std::sync::Arc;

use blackbox_vehicle::air_control::AirAxes;
use blackbox_vehicle::bike::BikeSpec;
use nfsmw_data::car::physics::BikePhysics;

use super::*;
use crate::scenes::world::bike_assist::set_air_control;

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

/// The angular velocity of the body in its own axes.
fn local_spin(sim: &CarSim) -> Vec3 {
    let body = sim.vehicle.body();
    body.rotation().transpose() * body.angular_velocity
}

/// A vehicle rolling at 12 m/s that loses the ground for `seconds` with the air control `axes` held: its spin in its own axes at the end.
fn spin_in_the_air(mut sim: CarSim, setting: bool, player: bool, axes: AirAxes, seconds: f32) -> Vec3 {
    set_air_control(setting);
    sim.collision_role.player = player;
    let road = FlatGround::new(5.0);
    assert!(sim.place_moving(&road, spawn_at(0.2), 12.0));
    for _ in 0..30 {
        let _ = sim.step(&DriveInput { throttle: 0.4, ..Default::default() }, &road, None);
    }
    for _ in 0..(seconds * 60.0) as usize {
        let _ = sim.step(&DriveInput { air: axes, ..Default::default() }, &NoGround, None);
    }
    set_air_control(false);
    local_spin(&sim)
}

fn pitch_forward() -> AirAxes {
    AirAxes { pitch: 1.0, ..AirAxes::ZERO }
}

#[test]
fn a_car_in_the_air_pitches_by_the_stick_when_the_setting_is_on() {
    let on = spin_in_the_air(sim(), true, true, pitch_forward(), 0.5);
    let off = spin_in_the_air(sim(), false, true, pitch_forward(), 0.5);
    assert!(on.x > off.x + 0.4, "nose down by the stick: {} against {}", on.x, off.x);
    let rolled = spin_in_the_air(sim(), true, true, AirAxes { roll: 1.0, ..AirAxes::ZERO }, 0.5);
    assert!(rolled.z < off.z - 0.4, "the right side down: {}", rolled.z);
    let spun = spin_in_the_air(sim(), true, true, AirAxes { yaw: 1.0, ..AirAxes::ZERO }, 0.5);
    assert!(spun.y > off.y + 0.4, "a spin to the right: {}", spun.y);
}

#[test]
fn the_air_control_is_the_players_and_off_by_default() {
    let plain = spin_in_the_air(sim(), false, true, pitch_forward(), 0.5);
    let computer = spin_in_the_air(sim(), true, false, pitch_forward(), 0.5);
    assert_eq!(plain, computer, "a computer driver's car flies as it always did");
}

#[test]
fn a_car_with_its_wheels_on_the_road_does_not_answer_the_air_stick() {
    let road = FlatGround::new(5.0);
    let ride = |setting: bool| {
        set_air_control(setting);
        let mut car = sim();
        assert!(car.place_moving(&road, spawn_at(0.2), 12.0));
        for _ in 0..120 {
            let _ = car.step(
                &DriveInput { throttle: 0.4, air: AirAxes { pitch: 1.0, roll: 1.0, yaw: 1.0 }, ..Default::default() },
                &road,
                None,
            );
        }
        set_air_control(false);
        (car.vehicle.body().orientation, car.vehicle.body().position)
    };
    assert_eq!(ride(true), ride(false));
}

#[test]
fn a_bike_in_the_air_pitches_by_the_stick_when_the_setting_is_on() {
    let on = spin_in_the_air(bike_sim(), true, true, pitch_forward(), 0.5);
    let off = spin_in_the_air(bike_sim(), false, true, pitch_forward(), 0.5);
    assert!(on.x > off.x + 0.5, "nose down by the stick: {} against {}", on.x, off.x);
    let computer = spin_in_the_air(bike_sim(), true, false, pitch_forward(), 0.5);
    assert_eq!(off, computer, "a computer driver's bike flies as it always did");
}
