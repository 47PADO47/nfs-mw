//! The ball joint on free bodies (no ground): what the impulses conserve and what they remove.

use glam::{Quat, Vec3};

use super::*;
use crate::FIXED_STEP;

fn free_body() -> RigidBody {
    let spec = RigidBodySpec { gravity: 0.0, ..RigidBodySpec::default() };
    RigidBody::new(1000.0, Vec3::new(1.0, 0.5, 2.0), Vec3::ONE, spec)
}

/// Two bodies in a row along z, joined at the point between them.
fn pair() -> (RigidBody, RigidBody, BallJoint) {
    let (mut a, mut b) = (free_body(), free_body());
    a.place(Vec3::ZERO, Quat::IDENTITY);
    b.place(Vec3::new(0.0, 0.0, -5.0), Quat::IDENTITY);
    (a, b, BallJoint { anchor_a: Vec3::new(0.0, 0.0, -2.5), anchor_b: Vec3::new(0.0, 0.0, 2.5) })
}

#[test]
fn the_anchors_of_a_placed_pair_coincide() {
    let (a, b, joint) = pair();
    assert!(joint.gap(&a, &b) < 1e-6);
}

#[test]
fn solving_removes_the_relative_velocity_of_the_anchors_and_keeps_the_momentum() {
    let (mut a, mut b, joint) = pair();
    a.linear_velocity = Vec3::new(2.0, 0.0, 10.0);
    a.angular_velocity = Vec3::new(0.0, 0.5, 0.0);
    let momentum = |a: &RigidBody, b: &RigidBody| a.linear_velocity * a.mass() + b.linear_velocity * b.mass();
    let before = momentum(&a, &b);
    joint.solve(&mut a, &mut b, FIXED_STEP, 1);
    let (pa, pb) = joint.world_anchors(&a, &b);
    let relative = a.point_velocity(pa) - b.point_velocity(pb);
    assert!(relative.length() < 1e-3, "anchors still move apart at {relative:?}");
    assert!((momentum(&a, &b) - before).length() < 1e-2, "momentum {before:?} became {:?}", momentum(&a, &b));
}

#[test]
fn a_pulled_body_follows_and_the_anchors_stay_together() {
    let (mut a, mut b, joint) = pair();
    let mut worst = 0.0_f32;
    for step in 0..600 {
        // Pull the first body forward and, after two seconds, steer it round.
        let turn = if step > 120 { 0.4 } else { 0.0 };
        a.apply_force(Vec3::new(0.0, 0.0, 4000.0));
        a.apply_torque(Vec3::new(0.0, turn * 3000.0, 0.0));
        a.begin_frame(FIXED_STEP);
        b.begin_frame(FIXED_STEP);
        joint.solve(&mut a, &mut b, FIXED_STEP, 4);
        worst = worst.max(joint.gap(&a, &b));
    }
    assert!(worst < 0.05, "the anchors drifted {worst} m apart");
    assert!(a.linear_velocity.length() > 1.0, "the pulling body moves");
    let apart = (a.linear_velocity - b.linear_velocity).length();
    assert!(apart < a.linear_velocity.length() * 0.5, "the pulled body follows: {apart} m/s apart");
}

#[test]
fn a_gap_closes_gently() {
    let (mut a, mut b, joint) = pair();
    b.position.z -= 1.0;
    let start = joint.gap(&a, &b);
    for _ in 0..120 {
        joint.solve(&mut a, &mut b, FIXED_STEP, 4);
        a.begin_frame(FIXED_STEP);
        b.begin_frame(FIXED_STEP);
        assert!(a.linear_velocity.length() <= MAX_CORRECTION_SPEED + 1e-3);
    }
    assert!(start > 0.9 && joint.gap(&a, &b) < 0.05, "gap {start} -> {}", joint.gap(&a, &b));
}

#[test]
fn a_frozen_body_does_not_move() {
    let (mut a, mut b, joint) = pair();
    b.state = BodyState::Frozen;
    a.linear_velocity = Vec3::new(0.0, 0.0, -5.0);
    joint.solve(&mut a, &mut b, FIXED_STEP, 2);
    assert_eq!(b.linear_velocity, Vec3::ZERO);
    assert!(a.linear_velocity.length() < 0.5, "the free body is held back: {:?}", a.linear_velocity);
}

#[test]
fn two_sleeping_bodies_stay_asleep() {
    let (mut a, mut b, joint) = pair();
    a.state = BodyState::Asleep;
    b.state = BodyState::Asleep;
    joint.solve(&mut a, &mut b, FIXED_STEP, 2);
    assert_eq!((a.state, b.state), (BodyState::Asleep, BodyState::Asleep));
}

#[test]
fn an_impulse_at_a_point_turns_the_body() {
    let mut body = free_body();
    body.apply_impulse_at(Vec3::new(1000.0, 0.0, 0.0), Vec3::new(0.0, 0.0, 2.0));
    assert!((body.linear_velocity.x - 1.0).abs() < 1e-5);
    // r = +z, J = +x: r x J = +y.
    assert!(body.angular_velocity.y > 0.0 && body.angular_velocity.x.abs() < 1e-6);
}
