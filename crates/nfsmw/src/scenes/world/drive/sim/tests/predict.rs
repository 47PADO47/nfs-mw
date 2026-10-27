//! The prediction's view of the car (`sim/predict.rs`): a restore makes the next steps exactly what they were, and a
//! motion written into the car reads back the same.

use super::{DriveInput, FlatGround, Spawn, Vec3, sim};

fn input(tick: u32) -> DriveInput {
    DriveInput { throttle: 1.0, steer: ((tick % 90) as f32 / 45.0 - 1.0) * 0.5, ..DriveInput::default() }
}

#[test]
fn a_restored_car_steps_exactly_as_it_did() {
    let ground = FlatGround::new(5.0);
    let mut car = sim();
    assert!(car.place_moving(&ground, Spawn { position: Vec3::new(100.0, 50.0, 5.0), heading: 0.0 }, 15.0));
    for tick in 0..60 {
        let _ = car.step(&input(tick), &ground, None);
    }
    let saved = car.save();
    let run = |car: &mut super::CarSim| {
        (60..180)
            .map(|tick| {
                let _ = car.step(&input(tick), &ground, None);
                car.motion()
            })
            .collect::<Vec<_>>()
    };
    let first = run(&mut car);
    car.restore(&saved);
    let second = run(&mut car);
    assert_eq!(first, second, "the same inputs from the same save give the same motion, bit for bit");
}

#[test]
fn restore_keeps_the_delta_used_for_the_saved_acceleration() {
    let ground = FlatGround::new(5.0);
    let mut car = sim();
    assert!(car.place_moving(&ground, Spawn { position: Vec3::new(100.0, 50.0, 5.0), heading: 0.0 }, 15.0));
    car.body_mut().apply_force(Vec3::X * 1000.0);
    car.step_with_delta(1.0 / 144.0, &input(0), &ground, None);
    let acceleration = car.camera_acceleration();
    assert!(acceleration.length() > 0.0, "the saved step must exercise acceleration");
    let saved = car.save();
    car.step_with_delta(1.0 / 576.0, &input(1), &ground, None);
    car.restore(&saved);
    assert_eq!(car.camera_acceleration(), acceleration);
}

#[test]
fn a_motion_written_into_the_car_reads_back() {
    let ground = FlatGround::new(5.0);
    let mut car = sim();
    assert!(car.place_moving(&ground, Spawn { position: Vec3::new(100.0, 50.0, 5.0), heading: 0.7 }, 20.0));
    for tick in 0..30 {
        let _ = car.step(&input(tick), &ground, None);
    }
    let mut motion = car.motion();
    motion.position += Vec3::new(3.0, -2.0, 0.25);
    motion.rotation = glam::Quat::from_rotation_z(1.1) * motion.rotation;
    motion.linear = Vec3::new(4.0, 5.0, 0.0);
    motion.angular = Vec3::new(0.0, 0.0, 0.5);
    car.set_motion(&motion);
    let read = car.motion();
    assert!(read.position.distance(motion.position) < 1e-3, "{:?} {:?}", read.position, motion.position);
    assert!(read.rotation.angle_between(motion.rotation) < 1e-3);
    assert!(read.linear.distance(motion.linear) < 1e-4 && read.angular.distance(motion.angular) < 1e-4);
}

#[test]
fn a_motion_that_is_not_finite_is_ignored() {
    let ground = FlatGround::new(5.0);
    let mut car = sim();
    assert!(car.place_moving(&ground, Spawn { position: Vec3::new(100.0, 50.0, 5.0), heading: 0.0 }, 0.0));
    let before = car.motion();
    let mut wild = before;
    wild.position.x = f32::NAN;
    car.set_motion(&wild);
    assert_eq!(car.motion(), before);
}
