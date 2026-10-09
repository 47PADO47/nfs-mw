use blackbox_vehicle::ground::Ground;
use blackbox_vehicle::{FlatGround, VehicleSpec};
use glam::Quat;
use nfsmw_data::car::physics::{CarBounds, CarPhysics, WallSpec, no_engine, no_transmission};

use super::*;

fn physics(spec: VehicleSpec) -> CarPhysics {
    let bounds = CarBounds { half_dimensions: spec.dimension, pivot: Vec3::new(0.0, spec.dimension.y, 0.0) };
    CarPhysics { spec, bounds, walls: WallSpec::default() }
}

fn tractor() -> CarSim {
    let mut sim = CarSim::new(physics(VehicleSpec::example()), [0.2; 4]);
    sim.configure_ai();
    sim
}

/// A long body on a tandem axle at its back, with no engine.
fn trailer() -> CarSim {
    let mut spec = VehicleSpec::example();
    spec.dimension = Vec3::new(0.9, 0.62, 3.0);
    spec.chassis.front_axle = -0.6;
    spec.chassis.wheel_base = 1.0;
    spec.chassis.front_weight_bias = 50.0;
    spec.engine = no_engine();
    spec.transmission = no_transmission();
    let mut sim = CarSim::new(physics(spec), [0.2; 4]);
    sim.configure_trailer();
    sim
}

fn spawn() -> Spawn {
    Spawn { position: Vec3::new(100.0, 50.0, 5.0), heading: 0.0 }
}

type Pair = (CarSim, CarSim, Coupling);

/// A tractor and a trailer behind it, hitched and rolling at `speed`.
fn rig(ground: &dyn Ground, speed: f32) -> Pair {
    let (mut tractor, mut trailer) = (tractor(), trailer());
    assert!(tractor.place_moving(ground, spawn(), speed));
    let back = tractor.half_dimensions().z + trailer.half_dimensions().z;
    let behind = Spawn { position: spawn().position - Vec3::new(back, 0.0, 0.0), heading: 0.0 };
    assert!(trailer.place_moving(ground, behind, speed));
    let coupling = Coupling::new(&tractor, &trailer);
    coupling.align(&tractor, &mut trailer);
    (tractor, trailer, coupling)
}

/// Steps the pair for `seconds` and returns the largest gap between the hitch and the kingpin.
fn run(pair: &mut Pair, input: &DriveInput, brakes: Brakes, seconds: f32) -> f32 {
    let ground = FlatGround::new(5.0);
    let (tractor, trailer, coupling) = pair;
    let towed = DriveInput { brake: brakes.brake, handbrake: brakes.handbrake, ..DriveInput::default() };
    let mut worst = 0.0_f32;
    for _ in 0..(seconds / STEP) as usize {
        let _ = tractor.step(input, &ground, None);
        let _ = trailer.step(&towed, &ground, None);
        coupling.hold(tractor, trailer);
        worst = worst.max(coupling.gap(tractor, trailer));
    }
    worst
}

fn heading(sim: &CarSim) -> f32 {
    let forward = sim.body().rotation().z_axis;
    forward.x.atan2(forward.z)
}

#[test]
fn the_fifth_wheel_is_at_the_back_of_the_tractor_and_the_front_of_the_trailer() {
    let joint = fifth_wheel(Vec3::new(1.3, 1.33, 3.6), Vec3::new(1.4, 1.2, 5.5));
    assert_eq!(joint.anchor_a, Vec3::new(0.0, FIFTH_WHEEL_HEIGHT - 1.33, -3.6));
    assert_eq!(joint.anchor_b, Vec3::new(0.0, FIFTH_WHEEL_HEIGHT - 1.2, 5.5));
}

#[test]
fn a_trailer_starts_hitched() {
    let ground = FlatGround::new(5.0);
    let (tractor, trailer, coupling) = rig(&ground, 10.0);
    assert!(coupling.is_hitched() && coupling.gap(&tractor, &trailer) < 1e-4);
    assert!((tractor.body().linear_velocity - trailer.body().linear_velocity).length() < 1e-3);
}

#[test]
fn a_tractor_drags_its_trailer_straight_and_round_a_bend() {
    let ground = FlatGround::new(5.0);
    let mut pair = rig(&ground, 8.0);
    let drive = DriveInput { throttle: 0.8, ..DriveInput::default() };
    let straight = run(&mut pair, &drive, Brakes::default(), 5.0);
    let (tractor_speed, trailer_speed) = (pair.0.telemetry().speed_mps, pair.1.telemetry().speed_mps);
    assert!(tractor_speed > 10.0, "the tractor speeds up: {tractor_speed}");
    assert!((tractor_speed - trailer_speed).abs() < 1.0, "the trailer keeps up: {trailer_speed}");
    let start = heading(&pair.0);
    let bend = DriveInput { throttle: 0.5, steer: 0.5, ..DriveInput::default() };
    let round = run(&mut pair, &bend, Brakes::default(), 6.0);
    assert!(straight < 0.05 && round < 0.05, "the hitch came apart by {straight} and {round} m");
    assert!((heading(&pair.0) - start).abs() > 0.3, "the tractor turned");
    assert!(heading(&pair.1).abs() > 0.1, "the trailer was dragged round");
    assert!(pair.2.is_hitched(), "an ordinary bend does not release the joint");
    assert!(pair.0.is_finite() && pair.1.is_finite());
}

#[test]
fn the_trailer_brakes_with_the_tractor() {
    let ground = FlatGround::new(5.0);
    let mut pair = rig(&ground, 15.0);
    let stop = DriveInput { brake: 1.0, ..DriveInput::default() };
    let gap = run(&mut pair, &stop, Brakes { brake: 1.0, handbrake: false }, 4.0);
    assert!(gap < 0.1, "the hitch held while braking: {gap} m");
    assert!(pair.0.telemetry().speed_mps < 3.0 && pair.1.telemetry().speed_mps < 3.0);
}

#[test]
fn a_trailer_that_tips_is_let_go_and_left_behind() {
    let ground = FlatGround::new(5.0);
    let mut pair = rig(&ground, 10.0);
    let drive = DriveInput { throttle: 1.0, ..DriveInput::default() };
    let _ = run(&mut pair, &drive, Brakes::default(), 1.0);
    // Roll the trailer onto its side.
    let (position, orientation) = (pair.1.body().position, pair.1.body().orientation);
    let velocity = pair.1.body().linear_velocity;
    pair.1.body_mut().place(position, Quat::from_rotation_z(1.2) * orientation);
    pair.1.body_mut().linear_velocity = velocity;
    let _ = run(&mut pair, &drive, Brakes::default(), 0.1);
    assert!(!pair.2.is_hitched(), "the joint is released");
    let _ = run(&mut pair, &drive, Brakes::default(), 3.0);
    assert!(pair.2.gap(&pair.0, &pair.1) > 3.0, "the tractor drives away from it");
}
