//! A bike in the car's simulation: it stands, drives, turns and stops on a flat road through the same `CarSim` a car does.

use blackbox_vehicle::FlatGround;
use blackbox_vehicle::bike::BikeSpec;
use nfsmw_data::car::physics::CarBounds;

use super::*;

const BOUNDS: CarBounds = CarBounds { half_dimensions: Vec3::new(0.42, 0.64, 1.06), pivot: Vec3::new(0.0, 0.64, 0.0) };

fn bike_sim() -> CarSim {
    CarSim::new(CarPhysics::from_bike(Arc::new(BikeSpec::example()), 0.0, BOUNDS), [0.32; 4])
}

fn spawn() -> Spawn {
    Spawn { position: Vec3::new(100.0, 50.0, 5.0), heading: 0.0 }
}

fn ride(sim: &mut CarSim, ground: &dyn Ground, input: DriveInput, seconds: f32) {
    for _ in 0..(seconds * 60.0) as usize {
        let _ = sim.step(&input, ground, None);
    }
}

#[test]
fn a_bike_sim_is_a_bike() {
    assert!(bike_sim().is_bike());
    let car = CarSim::new(CarPhysics::from_stats(&Default::default(), BOUNDS), [0.2; 4]);
    assert!(!car.is_bike());
}

#[test]
fn a_bike_stands_on_its_two_wheels_and_reports_them_as_four() {
    let ground = FlatGround::new(5.0);
    let mut sim = bike_sim();
    assert!(sim.place(&ground, spawn()));
    ride(&mut sim, &ground, DriveInput::default(), 3.0);
    let t = sim.telemetry();
    // Each wheel counts twice: both down is "all four".
    assert_eq!(t.wheels_on_ground, 4);
    assert!(t.speed_mps.abs() < 0.3 && t.gear == 1, "{t:?}");
    assert!(sim.is_finite() && sim.up_dot() > 0.99);
    // The model stands on the road, the wheels it has are drawn and the corner it has none for is left alone.
    let pose = sim.pose();
    assert!((pose.position.z - 5.0).abs() < 0.1, "height {}", pose.position.z);
    assert_eq!(pose.wheels[1], pose.wheels[0], "corner 1 is the handlebar group, which steers like the front wheel");
    assert_eq!(pose.wheels[3], WheelPose::default());
    assert!(pose.wheels[0].travel.abs() < 0.1 && pose.wheels[2].travel.abs() < 0.1, "{:?}", pose.wheels);
}

#[test]
fn the_wheels_of_a_bike_are_reported_front_front_rear_rear() {
    let ground = FlatGround::new(5.0);
    let mut sim = bike_sim();
    assert!(sim.place(&ground, spawn()));
    ride(&mut sim, &ground, DriveInput::default(), 2.0);
    let patches = sim.wheel_patches();
    assert!(patches.iter().all(|patch| patch.on_ground));
    assert_eq!(patches[0].position, patches[1].position);
    assert_eq!(patches[2].position, patches[3].position);
    // The front patch is ahead of the rear one, a wheelbase away along the road (+x in the world is +z in physics).
    let wheelbase = BikeSpec::example().chassis.wheelbase;
    assert!(((patches[0].position - patches[2].position).length() - wheelbase).abs() < 0.1);
    assert_eq!(sim.wheel_radii(), [BikeSpec::example().front.radius; 4]);
    assert_eq!(sim.tyre_hits().count(), 2, "two wheels, two hits");
}

#[test]
fn full_throttle_accelerates_the_bike_and_turns_both_wheels() {
    let ground = FlatGround::new(5.0);
    let mut sim = bike_sim();
    assert!(sim.place(&ground, spawn()));
    let start = sim.pose().position;
    ride(&mut sim, &ground, DriveInput { throttle: 1.0, ..DriveInput::default() }, 8.0);
    let t = sim.telemetry();
    let moved = sim.pose().position - start;
    assert!(t.speed_mps > 20.0, "speed {} m/s", t.speed_mps);
    assert!(moved.x > 50.0 && moved.y.abs() < 2.0, "moved {moved:?}");
    assert!(t.gear >= 2 && t.rpm > 2000.0, "{t:?}");
    assert!(sim.is_finite());
    let pose = sim.pose();
    assert!(pose.wheels[0].spin > 10.0 && pose.wheels[2].spin > 10.0, "{:?}", pose.wheels);
    // The rear wheel never steers.
    assert_eq!(pose.wheels[2].steer, 0.0);
}

#[test]
fn a_stick_to_the_right_takes_the_bike_to_the_right() {
    let ground = FlatGround::new(5.0);
    let mut sim = bike_sim();
    assert!(sim.place_moving(&ground, spawn(), 20.0));
    ride(&mut sim, &ground, DriveInput { throttle: 0.15, ..DriveInput::default() }, 1.0);
    ride(&mut sim, &ground, DriveInput { throttle: 0.15, steer: 0.5, ..DriveInput::default() }, 4.0);
    let pose = sim.pose();
    // Right of +X is -Y.
    assert!(pose.position.y < 50.0 - 3.0, "went {:?}", pose.position);
    assert!(sim.is_finite() && sim.up_dot() > 0.5);
    // The bike leans into the turn: its up axis tips towards the right of its heading (the model's -y). The right is the
    // horizontal one: the dot product of the bike's own up and right axes is 0 for any lean.
    let up = pose.rotation * Vec3::Z;
    let right = (pose.rotation * Vec3::X).cross(Vec3::Z).normalize_or_zero();
    assert!(up.dot(right) > 0.1, "lean {}", up.dot(right));
}

#[test]
fn braking_stops_a_bike_and_never_reverses_it() {
    let ground = FlatGround::new(5.0);
    let mut sim = bike_sim();
    assert!(sim.place_moving(&ground, spawn(), 25.0));
    ride(&mut sim, &ground, DriveInput { brake: 0.7, ..DriveInput::default() }, 8.0);
    let speed = sim.telemetry().speed_mps;
    assert!((-0.5..1.0).contains(&speed), "speed {speed}");
    assert!(sim.is_finite());
}

#[test]
fn the_handbrake_brakes_the_rear_wheel_alone() {
    let ground = FlatGround::new(5.0);
    let (mut rear, mut both) = (bike_sim(), bike_sim());
    assert!(rear.place_moving(&ground, spawn(), 20.0));
    assert!(both.place_moving(&ground, spawn(), 20.0));
    ride(&mut rear, &ground, DriveInput { handbrake: true, ..DriveInput::default() }, 1.0);
    ride(&mut both, &ground, DriveInput { brake: 1.0, ..DriveInput::default() }, 1.0);
    let (a, b) = (rear.telemetry().speed_mps, both.telemetry().speed_mps);
    assert!(a > b + 1.0, "the rear brake alone slows less: {a} {b}");
}

#[test]
fn the_lamps_and_readouts_follow_the_riders_levers() {
    let ground = FlatGround::new(5.0);
    let mut sim = bike_sim();
    assert!(sim.place_moving(&ground, spawn(), 10.0));
    let _ = sim.step(&DriveInput { throttle: 0.6, brake: 0.4, steer: 0.2, ..DriveInput::default() }, &ground, None);
    assert_eq!(sim.lamp_brake(), 1.0);
    assert!((sim.gas_pedal() - 0.6).abs() < 1e-6 && (sim.brake_pedal() - 0.4).abs() < 1e-6);
    assert!((sim.steering_input() - 0.2).abs() < 1e-6);
    let _ = sim.step(&DriveInput::default(), &ground, None);
    assert_eq!(sim.lamp_brake(), 0.0);
}

#[test]
fn a_bike_is_never_placed_as_traffic() {
    let ground = FlatGround::new(5.0);
    let mut sim = bike_sim();
    assert!(!sim.place_traffic_moving(&ground, spawn(), 10.0));
}

#[test]
fn a_bike_that_goes_over_stops_and_stays_above_the_road() {
    let ground = FlatGround::new(5.0);
    let mut sim = bike_sim();
    assert!(sim.place_moving(&ground, spawn(), 15.0));
    ride(&mut sim, &ground, DriveInput { throttle: 0.2, ..DriveInput::default() }, 1.0);
    // Throw the bike onto its side.
    sim.vehicle.body_mut().angular_velocity = Vec3::new(0.0, 0.0, 4.0);
    ride(&mut sim, &ground, DriveInput { throttle: 1.0, steer: 1.0, ..DriveInput::default() }, 8.0);
    assert!(sim.is_finite(), "a fall must not break the simulation");
    assert!(sim.pose().position.z > 4.0, "the bike went through the road: {}", sim.pose().position.z);
}

#[test]
fn a_saved_bike_is_restored_whole() {
    let ground = FlatGround::new(5.0);
    let mut sim = bike_sim();
    assert!(sim.place_moving(&ground, spawn(), 15.0));
    ride(&mut sim, &ground, DriveInput { throttle: 0.5, ..DriveInput::default() }, 1.0);
    let saved = sim.save();
    let (speed, gear) = (sim.telemetry().speed_mps, sim.telemetry().gear);
    ride(&mut sim, &ground, DriveInput { brake: 1.0, ..DriveInput::default() }, 2.0);
    assert!(sim.telemetry().speed_mps < speed - 3.0);
    sim.restore(&saved);
    assert_eq!((sim.telemetry().speed_mps, sim.telemetry().gear), (speed, gear));
}
