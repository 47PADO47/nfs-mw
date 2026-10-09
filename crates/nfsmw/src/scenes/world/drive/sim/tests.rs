use blackbox_vehicle::ground::NoGround;
use blackbox_vehicle::{FlatGround, VehicleSpec};
use nfsmw_data::car::physics::CarBounds;

use super::*;

fn sim() -> CarSim {
    let spec = VehicleSpec::example();
    let bounds = CarBounds { half_dimensions: spec.dimension, pivot: Vec3::new(0.0, spec.dimension.y, 0.1) };
    CarSim::new(CarPhysics { spec, bounds, walls: WallSpec::default() }, [0.2; 4])
}

fn spawn_at(heading: f32) -> Spawn {
    Spawn { position: Vec3::new(100.0, 50.0, 5.0), heading }
}

fn drive(sim: &mut CarSim, ground: &dyn Ground, input: DriveInput, seconds: f32) {
    for _ in 0..(seconds * 60.0) as usize {
        let _ = sim.step(&input, ground, None);
    }
}

#[test]
fn placing_puts_the_car_on_the_road_facing_the_heading() {
    // The road at render z = 5 is physics y = 5.
    let ground = FlatGround::new(5.0);
    let mut sim = sim();
    assert!(sim.place(&ground, spawn_at(0.0)));
    let pose = sim.pose();
    assert!((pose.position.x - 100.0 + 0.1).abs() < 0.5 && (pose.position.y - 50.0).abs() < 0.5, "{:?}", pose.position);
    assert!(pose.position.z > 5.0 && pose.position.z < 6.0, "{:?}", pose.position);
    // Facing +X in the world: the model's forward axis is the world's x axis.
    let forward = pose.rotation * Vec3::X;
    assert!((forward - Vec3::X).length() < 1e-4, "{forward:?}");
    assert!(((pose.rotation * Vec3::Z) - Vec3::Z).length() < 1e-4, "up is up");

    // Headed along +Y (a quarter turn to the left), the car points along +Y.
    assert!(sim.place(&ground, spawn_at(std::f32::consts::FRAC_PI_2)));
    let forward = sim.pose().rotation * Vec3::X;
    assert!((forward - Vec3::Y).length() < 1e-4, "{forward:?}");
}

#[test]
fn full_throttle_accelerates_forward_and_up_through_the_gears() {
    let ground = FlatGround::new(5.0);
    let mut sim = sim();
    assert!(sim.place(&ground, spawn_at(0.0)));
    let start = sim.pose().position;
    drive(&mut sim, &ground, DriveInput { throttle: 1.0, ..DriveInput::default() }, 8.0);
    let t = sim.telemetry();
    let moved = sim.pose().position - start;
    assert!(t.speed_mps > 20.0, "speed {} m/s", t.speed_mps);
    assert!(moved.x > 50.0 && moved.y.abs() < 2.0 && moved.z.abs() < 0.5, "moved {moved:?}");
    assert!(t.gear >= 2, "gear {}", t.gear);
    assert!(sim.is_finite());
    assert_eq!(t.wheels_on_ground, 4);
    // The wheels have rolled forward: the spin grew.
    assert!(sim.pose().wheels.iter().all(|w| w.spin > 10.0), "{:?}", sim.pose().wheels);
}

#[test]
fn steering_right_turns_the_car_to_the_right() {
    let ground = FlatGround::new(5.0);
    let mut sim = sim();
    assert!(sim.place(&ground, spawn_at(0.0)));
    drive(&mut sim, &ground, DriveInput { throttle: 0.6, ..DriveInput::default() }, 3.0);
    drive(&mut sim, &ground, DriveInput { throttle: 0.5, steer: 1.0, ..DriveInput::default() }, 2.0);
    let pose = sim.pose();
    // Right of +X is -Y.
    assert!(pose.position.y < 50.0 - 1.0, "went {:?}", pose.position);
    let forward = pose.rotation * Vec3::X;
    assert!(forward.y < -0.1, "{forward:?}");
    // The front wheels show a right turn as a negative (left-positive) angle; the rear wheels none.
    assert!(pose.wheels[0].steer < 0.0 && pose.wheels[1].steer < 0.0);
    assert!(pose.wheels[2].steer == 0.0 && pose.wheels[3].steer == 0.0);
}

#[test]
fn braking_stops_the_car_and_reverse_follows() {
    let ground = FlatGround::new(5.0);
    let mut sim = sim();
    assert!(sim.place(&ground, spawn_at(0.0)));
    drive(&mut sim, &ground, DriveInput { throttle: 1.0, ..DriveInput::default() }, 4.0);
    drive(&mut sim, &ground, DriveInput { brake: 1.0, ..DriveInput::default() }, 8.0);
    assert!(sim.telemetry().speed_mps < -0.5, "the held brake pedal reverses: {}", sim.telemetry().speed_mps);
    assert_eq!(sim.telemetry().gear, -1);
}

#[test]
fn nothing_below_means_no_car() {
    assert!(!sim().place(&NoGround, spawn_at(0.0)));
}

#[test]
fn the_model_follows_the_body_not_the_other_way_round() {
    // Wheel travel is small on flat ground at rest and the car does not sink through the road.
    let ground = FlatGround::new(5.0);
    let mut sim = sim();
    assert!(sim.place(&ground, spawn_at(0.0)));
    drive(&mut sim, &ground, DriveInput::default(), 3.0);
    let pose = sim.pose();
    assert!(pose.position.z > 5.0 && pose.position.z < 5.6, "height {}", pose.position.z);
    assert!(pose.wheels.iter().all(|w| w.travel.abs() < MAX_VISIBLE_TRAVEL), "{:?}", pose.wheels);
}

#[test]
fn a_car_driving_into_a_standing_one_pushes_it_and_loses_speed() {
    let ground = FlatGround::new(5.0);
    let (mut chaser, mut target) = (sim(), sim());
    assert!(chaser.place_moving(&ground, spawn_at(0.0), 20.0));
    // 12 m ahead along the road (+x in the world), standing.
    let ahead = Spawn { position: Vec3::new(112.0, 50.0, 5.0), heading: 0.0 };
    assert!(target.place(&ground, ahead));
    let mut hits = 0;
    let (mut start, mut end) = (chaser.telemetry().speed_mps, 0.0);
    for _ in 0..180 {
        let _ = chaser.step(&DriveInput::default(), &ground, None);
        let _ = target.step(&DriveInput::default(), &ground, None);
        if chaser.collide_with(&mut target).is_some() {
            hits += 1;
        }
        end = chaser.telemetry().speed_mps;
        start = start.max(end);
    }
    assert!(hits > 0, "the cars never touched");
    assert!(target.telemetry().speed_mps > 2.0 || (target.pose().position - ahead.position).length() > 3.0);
    assert!(end < start - 3.0, "speed {start} -> {end}");
    assert!(chaser.is_finite() && target.is_finite());
    // Afterwards the boxes do not overlap.
    let (a, b) = (chaser.collision_box(), target.collision_box());
    assert!(blackbox_vehicle::rigid_body::obb_contact(&a, &b).is_none_or(|c| c.overlap < 0.05));
}
