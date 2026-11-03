//! A stick let go in a corner does not snap the rider upright (`docs/specs/ride6-rider-knee-foot.md`, section 5): the real bike
//! corners, the stick is released, and every number the animation reads is followed frame by frame.

use blackbox_vehicle::bike::{Bike, BikeInput, BikeSpec};
use blackbox_vehicle::{FIXED_STEP, FlatGround};
use glam::Vec3;

use super::attach::Inertia;
use super::params::{RiderSample, Smoothed, Values};

fn inertia() -> Inertia {
    Inertia {
        bone: "pelvis".into(),
        min: [-0.06, -0.15, -0.04],
        max: [0.0, 0.15, 0.2],
        braking_m: 0.0,
        full_acceleration: 0.0,
        rate: 4.0,
    }
}

/// What `CarSim::rider_sample` makes of the bike (the fields the lean pose reads; the rest is 0).
fn sample(bike: &Bike, input: &BikeInput) -> RiderSample {
    let max_lean = bike.spec().steering.max_lean.max(f32::EPSILON);
    RiderSample {
        max_lean: bike.spec().steering.max_lean,
        bike_lean: (bike.lean() / max_lean).clamp(-1.0, 1.0),
        rider_lean: (bike.target_lean() / max_lean).clamp(-1.0, 1.0),
        speed: bike.forward_speed(),
        throttle: input.throttle,
        tuck_level: bike.tuck_level(),
        front_down: true,
        rear_down: true,
        weight: bike.rider_weight(),
        steer: input.steer,
        offset: bike.rider_offset(),
        rotation: Vec3::new(
            bike.rider_rotation().roll_right(),
            bike.rider_rotation().pitch_up(),
            bike.rider_rotation().yaw_right(),
        ),
        ..RiderSample::default()
    }
}

/// One frame of the run: the stick, the sample and what the animation reads.
struct Frame {
    steer: f32,
    sample: RiderSample,
    values: Values,
    shift: Vec3,
    rotation: Vec3,
}

/// A bike at 30 m/s that rides straight for a second, takes a corner with the stick held for `corner` seconds and lets it go.
fn corner_and_release(corner: f32, straight_after: f32) -> Vec<Frame> {
    let mut bike = Bike::new(BikeSpec::example());
    let ground = FlatGround::new(0.0);
    assert!(bike.place_moving(&ground, 0.0, 0.0, 2.0, 0.0, 30.0));
    let mut smoothed = Smoothed::default();
    smoothed.set_kind(Some(0), Some(0));
    let mut frames = Vec::new();
    let steps = ((1.0 + corner + straight_after) / FIXED_STEP) as usize;
    for step in 0..steps {
        let t = step as f32 * FIXED_STEP;
        let steer = if t >= 1.0 && t < 1.0 + corner { 1.0 } else { 0.0 };
        let input = BikeInput { throttle: 0.4, front_brake: 0.0, rear_brake: 0.0, steer };
        bike.step(FIXED_STEP, &input, &ground);
        let sample = sample(&bike, &input);
        smoothed.step(&sample, &inertia(), FIXED_STEP);
        frames.push(Frame {
            steer,
            sample,
            values: smoothed.values(),
            shift: smoothed.shift(),
            rotation: smoothed.rotation(),
        });
    }
    frames
}

/// The largest change of `read` between two frames, per second.
fn fastest(frames: &[Frame], read: impl Fn(&Values) -> f32) -> f32 {
    frames.windows(2).map(|w| (read(&w[1].values) - read(&w[0].values)).abs() / FIXED_STEP).fold(0.0, f32::max)
}

#[test]
fn no_number_of_the_animation_jumps_when_the_stick_is_let_go_in_a_corner() {
    let frames = corner_and_release(4.0, 4.0);
    // The rates the smoothers of the blueprint allow: the lean follows at 4 a second of what is left (at most 4 a second of a
    // full swing), the tuck at 6, the pose switches (`lean_out`, `straight`) at 6 and 2, the foot override at 4.
    assert!(fastest(&frames, |v| v.lean) <= 5.0, "lean {}", fastest(&frames, |v| v.lean));
    assert!(fastest(&frames, |v| v.tuck) <= 6.5, "tuck {}", fastest(&frames, |v| v.tuck));
    assert!(fastest(&frames, |v| v.lean_out) <= 6.5, "lean_out {}", fastest(&frames, |v| v.lean_out));
    assert!(fastest(&frames, |v| v.straight) <= 2.5, "straight {}", fastest(&frames, |v| v.straight));
    assert!(fastest(&frames, |v| v.foot_release[1]) <= 4.5);
    assert!(fastest(&frames, |v| v.wheelie_direction) <= 20.0, "the wheelie side is a ramp of the roll");
}

#[test]
fn the_riders_body_moves_smoothly_too() {
    let frames = corner_and_release(4.0, 4.0);
    let rate = |read: &dyn Fn(&Frame) -> Vec3| {
        frames.windows(2).map(|w| (read(&w[1]) - read(&w[0])).length() / FIXED_STEP).fold(0.0, f32::max)
    };
    // The hang-off shift follows at the data's 4 a second of the 0.35 m the ranges allow, the turn of the body is the physics'.
    assert!(rate(&|f| f.shift) < 1.5, "shift {} m/s", rate(&|f| f.shift));
    assert!(rate(&|f| f.rotation) < 3.0, "turn {} rad/s", rate(&|f| f.rotation));
}

#[test]
fn the_riders_lean_follows_the_bikes_roll_back_to_upright_and_does_not_drop_with_the_stick() {
    let frames = corner_and_release(4.0, 4.0);
    let release = frames.iter().position(|f| f.steer == 0.0 && f.sample.rider_lean.abs() > 0.1).unwrap();
    let held = frames[release].values.lean;
    assert!(held > 0.5, "a held corner leans the pose: {held}");
    // A tenth of a second after the stick is let go the pose has hardly moved (the bike's roll has not either), and the lean
    // never rises again on the way back to upright.
    let soon = &frames[release + (0.1 / FIXED_STEP) as usize];
    assert!(soon.values.lean > 0.8 * held, "{} of {held}", soon.values.lean);
    let after: Vec<f32> = frames[release..].iter().map(|f| f.values.lean).collect();
    assert!(after.windows(2).all(|w| w[1] <= w[0] + 1e-3), "monotonic back to upright");
    // And the pose takes about as long to come back as the bike's own roll does.
    let upright = frames[release..].iter().position(|f| f.values.lean < 0.1).unwrap() as f32 * FIXED_STEP;
    let rolled = frames[release..].iter().position(|f| f.sample.bike_lean.abs() < 0.1).unwrap() as f32 * FIXED_STEP;
    assert!(upright >= 0.5 * rolled && upright <= rolled + 0.6, "pose {upright} s, bike {rolled} s");
}

#[test]
fn a_gentle_release_and_a_quick_flick_are_as_smooth() {
    for (corner, label) in [(0.3, "a flick"), (1.5, "a short corner")] {
        let frames = corner_and_release(corner, 3.0);
        assert!(fastest(&frames, |v| v.lean) <= 5.0, "{label}");
        assert!(fastest(&frames, |v| v.straight) <= 2.5, "{label}");
    }
}
