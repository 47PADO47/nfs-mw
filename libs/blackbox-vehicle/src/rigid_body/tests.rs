use glam::{Quat, Vec3};

use super::*;
use crate::FIXED_STEP;

fn body() -> RigidBody {
    RigidBody::new(1000.0, Vec3::new(1.0, 0.5, 2.0), Vec3::ONE, RigidBodySpec::default())
}

#[test]
fn free_fall_matches_gravity() {
    let mut b = body();
    b.position = Vec3::new(0.0, 100.0, 0.0);
    let steps = 60;
    for _ in 0..steps {
        b.begin_frame(FIXED_STEP);
    }
    // Forces from step N are integrated at step N+1, so the first begin_frame carries no gravity yet.
    let t = (steps - 1) as f32 * FIXED_STEP;
    assert!((b.linear_velocity.y + 9.81 * t).abs() < 1e-3, "v = {}", b.linear_velocity.y);
    let fallen = 100.0 - b.position.y;
    assert!((fallen - 0.5 * 9.81 * 1.0).abs() < 0.2, "fallen = {fallen}");
}

#[test]
fn speed_is_clamped() {
    let mut b = body();
    b.linear_velocity = Vec3::new(1000.0, 0.0, -1000.0);
    b.angular_velocity = Vec3::new(100.0, 0.0, 0.0);
    b.begin_frame(FIXED_STEP);
    assert_eq!(b.linear_velocity.x, MAX_LINEAR_SPEED);
    assert_eq!(b.linear_velocity.z, -MAX_LINEAR_SPEED);
    assert_eq!(b.angular_velocity.x, MAX_ANGULAR_SPEED);
}

#[test]
fn spin_keeps_orientation_normalised_and_rotates() {
    let mut b = body();
    b.angular_velocity = Vec3::new(0.0, 1.0, 0.0);
    for _ in 0..60 {
        b.begin_frame(FIXED_STEP);
        b.linear_velocity = Vec3::ZERO;
        b.force = Vec3::ZERO;
    }
    assert!((b.orientation.length() - 1.0).abs() < 1e-5);
    let yaw = (b.rotation() * Vec3::Z).x.atan2((b.rotation() * Vec3::Z).z);
    assert!((yaw - 1.0).abs() < 0.02, "yaw = {yaw}");
}

#[test]
fn force_at_point_makes_torque() {
    let mut b = body();
    b.apply_force_at(Vec3::new(0.0, 0.0, 10.0), Vec3::new(1.0, 0.0, 0.0));
    // r = +x, F = +z: r x F = (0*10 - 0*0, 0*0 - 1*10, 0) = -y.
    assert!((b.pending_torque() - Vec3::new(0.0, -10.0, 0.0)).length() < 1e-5);
    assert_eq!(b.pending_force(), Vec3::new(0.0, 0.0, 10.0));
}

#[test]
fn rotates_about_the_cog() {
    let spec = RigidBodySpec { cg: Vec3::new(0.0, 0.0, 1.0), ..Default::default() };
    let mut b = RigidBody::new(1000.0, Vec3::new(1.0, 0.5, 2.0), Vec3::ONE, spec);
    b.angular_velocity = Vec3::new(0.0, 2.0, 0.0);
    let cog0 = b.world_cog();
    b.begin_frame(FIXED_STEP);
    assert!((b.world_cog() - cog0).length() < 1e-4, "cog moved {}", (b.world_cog() - cog0).length());
}

#[test]
fn quadratic_drag_slows_the_body() {
    let spec = RigidBodySpec { gravity: 0.0, drag: Vec3::splat(1.0), ..Default::default() };
    let mut b = RigidBody::new(1000.0, Vec3::new(1.0, 0.5, 2.0), Vec3::ONE, spec);
    b.linear_velocity = Vec3::new(0.0, 0.0, 30.0);
    for _ in 0..600 {
        b.begin_frame(FIXED_STEP);
        b.apply_drag();
    }
    assert!(b.linear_velocity.z < 30.0 && b.linear_velocity.z > 0.0);
    assert!(b.linear_velocity.x.abs() < 1e-4);
}

#[test]
fn sleeps_when_slow_with_enough_contacts_and_wakes_on_force() {
    let mut b = body();
    b.linear_velocity = Vec3::new(0.0, 0.0, 0.01);
    assert_eq!(b.update_sleep(2), BodyState::Awake);
    assert_eq!(b.update_sleep(4), BodyState::Asleep);
    assert_eq!(b.linear_velocity, Vec3::ZERO);
    b.begin_frame(FIXED_STEP);
    assert_eq!(b.position, Vec3::ZERO);
    b.apply_force(Vec3::X);
    assert_eq!(b.state, BodyState::Awake);
}

#[test]
fn set_mass_rescales_inertia() {
    let mut b = body();
    let i0 = b.inertia();
    b.set_mass(2000.0);
    assert!((b.inertia() - i0 * 2.0).abs().max_element() < 1e-3);
}

fn floor_contact(b: &RigidBody) -> PlaneContact {
    PlaneContact {
        point: b.position + Vec3::new(0.0, -0.5, 0.0),
        normal: Vec3::Y,
        friction_static: 0.8,
        friction_kinetic: 0.6,
    }
}

#[test]
fn plane_contact_bounces_with_restitution() {
    let mut b = body();
    b.linear_velocity = Vec3::new(0.0, -10.0, 0.0);
    let c = floor_contact(&b);
    let params = ContactParams { restitution: 0.5, inertia_scale: Vec3::ONE };
    let r = b.react_plane(&c, &params).expect("approaching");
    assert!((b.linear_velocity.y - 5.0).abs() < 1e-3, "v = {}", b.linear_velocity.y);
    assert!((r.closing_speed - 10.0).abs() < 1e-4);
    assert!((r.impulse - 15.0 * 1000.0).abs() < 1.0);
}

#[test]
fn plane_contact_ignores_separating_bodies() {
    let mut b = body();
    b.linear_velocity = Vec3::new(0.0, 3.0, 0.0);
    let c = floor_contact(&b);
    let params = ContactParams { restitution: 0.0, inertia_scale: Vec3::ONE };
    assert!(b.react_plane(&c, &params).is_none());
}

#[test]
fn friction_slows_a_sliding_body_and_never_reverses_it() {
    let mut b = body();
    b.linear_velocity = Vec3::new(0.0, -1.0, 4.0);
    let c = floor_contact(&b);
    let params = ContactParams { restitution: 0.0, inertia_scale: Vec3::splat(1e6) };
    let r = b.react_plane(&c, &params).unwrap();
    assert!(b.linear_velocity.z >= 0.0 && b.linear_velocity.z < 4.0);
    assert_eq!(r.friction, FrictionState::Dynamic);
    // Kinetic friction removes mu * j / m = 0.6 * 1 = 0.6 m/s.
    assert!((b.linear_velocity.z - 3.4).abs() < 1e-3, "vz = {}", b.linear_velocity.z);
}

#[test]
fn contact_conserves_nothing_but_stays_finite_when_rotated() {
    let mut b = body();
    b.orientation = Quat::from_rotation_x(0.3);
    b.place(Vec3::new(0.0, 1.0, 0.0), Quat::from_rotation_x(0.3));
    b.linear_velocity = Vec3::new(1.0, -5.0, 2.0);
    let c = PlaneContact {
        point: b.position + Vec3::new(0.5, -0.5, 1.0),
        normal: Vec3::Y,
        friction_static: 0.8,
        friction_kinetic: 0.6,
    };
    let params = ContactParams { restitution: 0.2, inertia_scale: Vec3::ONE };
    b.react_plane(&c, &params).unwrap();
    assert!(b.linear_velocity.is_finite() && b.angular_velocity.is_finite());
    let vp = b.point_velocity(c.point);
    assert!(vp.y >= -1e-3, "contact point still approaching: {}", vp.y);
}
