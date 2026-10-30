use std::f32::consts::{FRAC_PI_2, PI};

use glam::{Quat, Vec3};

use super::*;
use crate::{FIXED_STEP, FlatGround};

const HALF: f32 = 0.15;
const RADIUS: f32 = 0.08;
const REACH: f32 = HALF + RADIUS;

fn segment(position: Vec3, orientation: Quat) -> Segment {
    Segment { mass: 10.0, radius: RADIUS, half_length: HALF, position, orientation }
}

/// Two segments joined end to end at the point `(0, height + REACH, 0)`: the parent stands below it along +Y, the child's frame is
/// turned by `bend` about Z about that point. Every constraint frame is the identity.
fn chain(limits: JointLimits, height: f32, bend: f32) -> RagdollSpec {
    let joint = Vec3::new(0.0, height + REACH, 0.0);
    let turn = Quat::from_rotation_z(bend);
    RagdollSpec {
        segments: vec![
            segment(Vec3::new(0.0, height, 0.0), Quat::IDENTITY),
            segment(joint + turn * Vec3::Y * REACH, turn),
        ],
        joints: vec![Joint {
            parent: 0,
            child: 1,
            anchor_parent: Vec3::Y * REACH,
            anchor_child: Vec3::NEG_Y * REACH,
            frame_parent: Quat::IDENTITY,
            frame_child: Quat::IDENTITY,
            limits,
        }],
        ..RagdollSpec::default()
    }
}

const FREE: JointLimits = JointLimits { swing_z: PI, swing_y: PI, twist: PI };

fn run(body: &mut Ragdoll, seconds: f32, ground: &FlatGround) {
    for _ in 0..(seconds / FIXED_STEP) as usize {
        body.step(FIXED_STEP, ground);
    }
}

#[test]
fn a_body_thrown_along_the_ground_inherits_the_velocity_and_slides_to_a_stop() {
    let ground = FlatGround::new(0.0);
    let flat = Quat::from_rotation_x(FRAC_PI_2);
    let spec = RagdollSpec { segments: vec![segment(Vec3::new(0.0, 0.1, 0.0), flat)], ..RagdollSpec::default() };
    let mut body = Ragdoll::new(&spec, Vec3::new(0.0, 0.0, 12.0), Vec3::ZERO);
    assert!((body.centre_velocity() - Vec3::new(0.0, 0.0, 12.0)).length() < 1e-5, "the velocity is inherited");
    body.step(FIXED_STEP, &ground);
    assert!(body.velocity(0).z > 11.0, "{:?}", body.velocity(0));
    run(&mut body, 8.0, &ground);
    assert!(body.at_rest(), "it stops: {:?}", body.velocity(0));
    // Coulomb friction (mu 0.6) stops it in v^2 / (2 mu g) = 12.2 m; the sub-steps make it a little different.
    let slid = body.position(0).z;
    assert!((8.0..16.0).contains(&slid), "{slid}");
    assert!(body.position(0).y >= 0.05 && body.is_finite());
}

#[test]
fn a_body_dropped_comes_to_rest_on_the_ground_not_in_it() {
    let ground = FlatGround::new(0.0);
    let spec = RagdollSpec {
        segments: vec![segment(Vec3::new(0.0, 2.0, 0.0), Quat::from_rotation_z(0.4))],
        ..RagdollSpec::default()
    };
    let mut body = Ragdoll::new(&spec, Vec3::ZERO, Vec3::ZERO);
    run(&mut body, 5.0, &ground);
    assert!(body.at_rest());
    // On its side a capsule's centre is one radius above the ground.
    assert!((body.position(0).y - RADIUS).abs() < 0.02, "{}", body.position(0).y);
    assert!(body.position(0).x.abs() < 1.0 && body.position(0).z.abs() < 1.0);
}

#[test]
fn a_joint_keeps_its_two_anchors_together_while_the_body_falls_and_tumbles() {
    let ground = FlatGround::new(0.0);
    let mut body = Ragdoll::new(&chain(FREE, 1.5, 0.3), Vec3::new(1.0, 0.0, 6.0), Vec3::new(2.0, 1.0, 0.0));
    let mut worst = 0.0_f32;
    for _ in 0..300 {
        body.step(FIXED_STEP, &ground);
        worst = worst.max(body.joint_gap(0));
    }
    assert!(worst < 0.02, "{worst}");
    assert!(body.is_finite());
}

#[test]
fn a_swing_limit_holds_the_child_to_its_angle() {
    // The parent is an anchor (very heavy, far above the ground); the child starts straight and is given a spin that would carry
    // it through half a turn. The joint may swing 30 degrees about Z only.
    let ground = FlatGround::new(-100.0);
    let limit = 30.0_f32.to_radians();
    let mut spec = chain(JointLimits { swing_z: limit, swing_y: 0.0, twist: 0.0 }, 5.0, 0.0);
    spec.segments[0].mass = 1.0e6;
    let mut body = Ragdoll::new(&spec, Vec3::ZERO, Vec3::ZERO);
    body.kick(1, Vec3::new(-REACH * 6.0, 0.0, 0.0), Vec3::new(0.0, 0.0, 6.0));
    let mut worst = 0.0_f32;
    for _ in 0..60 {
        body.step(FIXED_STEP, &ground);
        worst = worst.max(body.joint_excess(0));
    }
    assert!(worst < 0.1, "{worst}");
    let swing = (body.orientation(1) * Vec3::Y).angle_between(Vec3::Y);
    assert!(swing > 0.2 && swing < limit + 0.1, "it swings to its limit: {swing}");
}

#[test]
fn a_joint_started_beyond_its_limit_eases_back_instead_of_snapping() {
    let ground = FlatGround::new(0.0);
    let limit = JointLimits { swing_z: 0.2, swing_y: 0.2, twist: 0.2 };
    let mut body = Ragdoll::new(&chain(limit, 5.0, 1.0), Vec3::ZERO, Vec3::ZERO);
    assert!(body.joint_excess(0) > 0.7, "{}", body.joint_excess(0));
    body.step(FIXED_STEP, &ground);
    assert!(body.joint_excess(0) > 0.5, "one step does not snap it: {}", body.joint_excess(0));
    for _ in 0..120 {
        body.step(FIXED_STEP, &ground);
    }
    assert!(body.joint_excess(0) < 0.25, "{}", body.joint_excess(0));
}

#[test]
fn a_free_joint_lets_the_child_hang_down() {
    // Hinged at the top of an anchor, with the joint free, the child lying sideways swings down under gravity.
    let ground = FlatGround::new(-100.0);
    let mut spec = chain(FREE, 3.0, FRAC_PI_2);
    spec.segments[0].mass = 1.0e6;
    let mut body = Ragdoll::new(&spec, Vec3::ZERO, Vec3::ZERO);
    let start = body.position(1).y;
    run(&mut body, 0.6, &ground);
    assert!(body.position(1).y < start - 0.1, "{start} {:?}", body.position(1));
}

#[test]
fn friction_scales_with_the_coefficient() {
    let ground = FlatGround::new(0.0);
    let flat = Quat::from_rotation_x(FRAC_PI_2);
    let slide = |mu: f32| {
        let spec = RagdollSpec {
            segments: vec![segment(Vec3::new(0.0, 0.1, 0.0), flat)],
            friction: mu,
            ..RagdollSpec::default()
        };
        let mut body = Ragdoll::new(&spec, Vec3::new(0.0, 0.0, 10.0), Vec3::ZERO);
        run(&mut body, 8.0, &ground);
        body.position(0).z
    };
    assert!(slide(0.3) > slide(0.9) * 1.5, "{} {}", slide(0.3), slide(0.9));
}

#[test]
fn a_ragdoll_with_no_segments_or_a_bad_step_does_nothing() {
    let ground = FlatGround::new(0.0);
    let mut empty = Ragdoll::new(&RagdollSpec::default(), Vec3::X, Vec3::ZERO);
    empty.step(FIXED_STEP, &ground);
    assert!(empty.is_empty() && empty.is_finite());
    let spec =
        RagdollSpec { segments: vec![segment(Vec3::new(0.0, 1.0, 0.0), Quat::IDENTITY)], ..RagdollSpec::default() };
    let mut body = Ragdoll::new(&spec, Vec3::ZERO, Vec3::ZERO);
    body.step(f32::NAN, &ground);
    body.step(-1.0, &ground);
    assert_eq!(body.position(0), Vec3::new(0.0, 1.0, 0.0));
    assert_eq!(body.len(), 1);
}

#[test]
fn a_damped_joint_stops_spinning_a_free_one_does_not() {
    // In free fall (nothing to hang from) the child is given a spin about the joint; damping takes it away.
    let ground = FlatGround::new(-10_000.0);
    let spin = |damping: f32| {
        let mut spec = chain(FREE, 3.0, 0.0);
        spec.segments[0].mass = 1.0e6;
        spec.joint_damping = damping;
        let mut body = Ragdoll::new(&spec, Vec3::ZERO, Vec3::ZERO);
        body.kick(1, Vec3::new(-REACH * 5.0, 0.0, 0.0), Vec3::new(0.0, 0.0, 5.0));
        run(&mut body, 3.0, &ground);
        (body.velocity(1) - body.velocity(0)).length()
    };
    assert!(spin(4.0) < 0.05, "{}", spin(4.0));
    assert!(spin(0.0) > 0.5, "{}", spin(0.0));
}

#[test]
fn a_driven_joint_holds_its_child_at_the_target_against_gravity_and_a_limp_one_does_not() {
    let ground = FlatGround::new(0.0);
    // The child stands on the parent's end, high in the air, with a bend of 60 degrees the drive has to undo.
    let build = |driven: bool| {
        let mut body = Ragdoll::new(&chain(FREE, 5.0, 1.0), Vec3::ZERO, Vec3::ZERO);
        if driven {
            body.set_drive_gain(0.5);
            body.drive(0, Quat::from_rotation_z(0.5));
        }
        body
    };
    let mut driven = build(true);
    run(&mut driven, 0.25, &ground);
    assert!(driven.drive_error(0) < 0.03, "the child is at its target: {}", driven.drive_error(0));
    assert!(driven.joint_gap(0) < 0.01 && driven.is_finite());
    let mut limp = build(false);
    run(&mut limp, 0.25, &ground);
    assert!(limp.drive_error(0) == 0.0, "a limp joint has no target");
    // The parent does not stay still either: it turns by its share of the move, the pair together keeping their total momentum.
    assert!(driven.orientation(0).angle_between(Quat::IDENTITY) > 0.01);
}

#[test]
fn a_drive_follows_a_moving_target_and_lets_go_when_released() {
    let ground = FlatGround::new(0.0);
    let mut body = Ragdoll::new(&chain(FREE, 5.0, 0.0), Vec3::ZERO, Vec3::ZERO);
    body.set_drive_gain(0.5);
    assert_eq!(body.joint_count(), 1);
    for step in 0..60 {
        body.drive(0, Quat::from_rotation_z(step as f32 * 0.01));
        body.step(FIXED_STEP, &ground);
    }
    assert!(body.drive_error(0) < 0.05, "{}", body.drive_error(0));
    body.release_drives();
    assert_eq!(body.drive_error(0), 0.0);
    body.drive(7, Quat::IDENTITY);
    body.drive(0, Quat::from_xyzw(f32::NAN, 0.0, 0.0, 1.0));
    assert_eq!(body.drive_error(0), 0.0, "a target that is not a rotation is no target");
}

#[test]
fn with_the_gain_at_zero_a_drive_does_nothing() {
    let ground = FlatGround::new(0.0);
    let mut body = Ragdoll::new(&chain(FREE, 5.0, 1.0), Vec3::ZERO, Vec3::ZERO);
    body.drive(0, Quat::IDENTITY);
    run(&mut body, 0.1, &ground);
    assert!(body.drive_error(0) > 0.5, "{}", body.drive_error(0));
}
