//! The bike's ride: the two bodies, the levers and the fall.

use std::sync::Arc;

use blackbox_vehicle::bike::BikeSpec;
use blackbox_vehicle::{FIXED_STEP, FlatGround};
use glam::Quat;
use nfsmw_data::car::physics::{BikePhysics, CarBounds};

use super::*;

const BOUNDS: CarBounds = CarBounds { half_dimensions: Vec3::new(0.42, 0.64, 1.06), pivot: Vec3::new(0.0, 0.64, 0.05) };

fn physics() -> CarPhysics {
    let mut physics = CarPhysics::from_stats(&Default::default(), BOUNDS);
    physics.bike = Some(BikePhysics { spec: Arc::new(BikeSpec::example()), origin_x: -0.02 });
    physics
}

fn ride_and_double() -> (BikeRide, Vehicle) {
    let mut physics = physics();
    let ride = BikeRide::new(&mut physics).expect("a bike");
    let mut vehicle = Vehicle::new(physics.spec);
    ride.attach(&mut vehicle);
    (ride, vehicle)
}

#[test]
fn a_car_has_no_ride() {
    let mut physics = CarPhysics::from_stats(&Default::default(), BOUNDS);
    assert!(BikeRide::new(&mut physics).is_none());
}

#[test]
fn the_double_has_the_mass_and_the_inertia_of_the_bike() {
    let (ride, vehicle) = ride_and_double();
    let (bike, double) = (ride.bike.body(), vehicle.body());
    assert!((bike.mass() - double.mass()).abs() < 1e-3);
    let (a, b) = (bike.inertia(), double.inertia());
    assert!(((a - b) / a).abs().max_element() < 1e-4, "{a} {b}");
}

#[test]
fn the_two_bodies_share_their_centre_of_gravity_and_their_velocities() {
    let (mut ride, mut vehicle) = ride_and_double();
    let ground = FlatGround::new(0.0);
    assert!(ride.place(&ground, (3.0, -2.0, 2.0), 0.7, 12.0));
    ride.push(&mut vehicle);
    let (bike, double) = (ride.bike.body(), vehicle.body());
    assert!((bike.world_cog() - double.world_cog()).length() < 1e-4);
    assert!((bike.linear_velocity - double.linear_velocity).length() < 1e-6);
    assert_eq!(bike.orientation, double.orientation);
    // The double's origin is the bike's plus the offset between the model's origin and the box centre.
    let expected = bike.position + bike.rotation() * (BOUNDS.pivot - Vec3::new(0.0, 0.0, -0.02));
    assert!((double.position - expected).length() < 1e-4);
}

#[test]
fn what_a_contact_does_to_the_double_the_bike_takes() {
    let (mut ride, mut vehicle) = ride_and_double();
    let ground = FlatGround::new(0.0);
    assert!(ride.place(&ground, (0.0, 0.0, 2.0), 0.0, 10.0));
    ride.push(&mut vehicle);
    // A wall stops the double and turns it.
    vehicle.body_mut().linear_velocity = Vec3::new(0.0, 0.0, 2.0);
    vehicle.body_mut().angular_velocity = Vec3::new(0.0, 1.5, 0.0);
    vehicle.body_mut().position += Vec3::new(0.1, 0.0, 0.0);
    let cog = vehicle.body().world_cog();
    ride.pull(&vehicle);
    let bike = ride.bike.body();
    assert_eq!(bike.linear_velocity, Vec3::new(0.0, 0.0, 2.0));
    assert_eq!(bike.angular_velocity, Vec3::new(0.0, 1.5, 0.0));
    assert!((bike.world_cog() - cog).length() < 1e-4, "the bike moved with the double");
}

#[test]
fn the_pedals_and_the_stick_become_the_riders_levers() {
    let input = InputState { throttle: 0.5, brake: 0.3, steer: -2.0, ..Default::default() };
    let rider = command(&input);
    assert_eq!(rider, BikeInput { throttle: 0.5, front_brake: 0.3, rear_brake: 0.3, steer: -0.8 });
    // The handbrake is the rear brake alone.
    let handbrake = command(&InputState { handbrake: 1.0, ..Default::default() });
    assert_eq!((handbrake.front_brake, handbrake.rear_brake), (0.0, 1.0));
    // A harder brake pedal than the handbrake wins at the rear.
    let both = command(&InputState { brake: 0.8, handbrake: 0.5, ..Default::default() });
    assert_eq!((both.front_brake, both.rear_brake), (0.8, 0.8));
    // Shift requests and the clutch are not read, and a number that is not a number is no input.
    let odd = command(&InputState { throttle: f32::NAN, shift_up: true, clutch: 1.0, ..Default::default() });
    assert_eq!(odd, BikeInput::default());
}

#[test]
fn a_rider_who_lay_long_enough_stands_the_bike_up_where_it_is() {
    let (mut ride, _) = ride_and_double();
    let ground = FlatGround::new(0.0);
    assert!(ride.place(&ground, (4.0, 6.0, 2.0), 0.3, 0.0));
    // Tip it over: the next response lets go of the bike.
    ride.bike.body_mut().set_orientation(Quat::from_rotation_z(-1.5) * Quat::from_rotation_y(0.3));
    ride.bike.respond(FIXED_STEP, &BikeInput::default(), &ground);
    assert!(ride.bike.fallen());
    ride.lie(GET_UP_AFTER, &ground);
    assert!(!ride.bike.fallen(), "the rider got up");
    assert!(ride.bike.lean().abs() < 0.05);
    let at = ride.bike.position();
    assert!((at.x - 4.0).abs() < 0.5 && (at.z - 6.0).abs() < 0.5, "{at}");
    assert_eq!(ride.down, 0.0);
}

#[test]
fn a_rider_who_has_just_fallen_stays_down() {
    let (mut ride, _) = ride_and_double();
    let ground = FlatGround::new(0.0);
    assert!(ride.place(&ground, (0.0, 0.0, 2.0), 0.0, 0.0));
    ride.bike.body_mut().set_orientation(Quat::from_rotation_z(-1.5));
    ride.bike.respond(FIXED_STEP, &BikeInput::default(), &ground);
    assert!(ride.bike.fallen());
    ride.lie(0.5, &ground);
    assert!(ride.bike.fallen());
    assert!(ride.down > 0.4);
}

#[test]
fn a_bike_in_the_air_reports_the_wheels_it_has_off_the_ground() {
    let (ride, _) = ride_and_double();
    assert_eq!(ride.wheels_down(), 0);
    assert_eq!(ride.wheel_surface(0), None);
    assert_eq!(ride.wheel_hit(2), None);
    let front = ride.wheel_state(1);
    assert!(!front.on_ground && front.radius == BikeSpec::example().front.radius);
}
