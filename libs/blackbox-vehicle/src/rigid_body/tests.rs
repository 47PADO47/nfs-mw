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

mod bodies {
    use glam::{Mat3, Quat, Vec3};

    use super::super::{BodyContact, BodyHit, Obb, RigidBody, RigidBodySpec, obb_contact, react_bodies, separate};

    fn car_box(centre: Vec3, yaw: f32) -> Obb {
        Obb { centre, axes: Mat3::from_rotation_y(yaw), half: Vec3::new(0.9, 0.7, 2.2) }
    }

    fn body(position: Vec3, velocity: Vec3, mass: f32) -> RigidBody {
        let mut b = RigidBody::new(mass, Vec3::new(0.9, 0.7, 2.2), Vec3::ONE, RigidBodySpec::default());
        b.place(position, Quat::IDENTITY);
        b.linear_velocity = velocity;
        b
    }

    #[test]
    fn boxes_apart_do_not_touch_and_overlapping_boxes_report_the_shallowest_axis() {
        assert!(obb_contact(&car_box(Vec3::ZERO, 0.0), &car_box(Vec3::new(0.0, 0.0, 6.0), 0.0)).is_none());
        // Nose to tail, overlapping by 0.4 m along z: the normal points from the second box to the first.
        let c = obb_contact(&car_box(Vec3::new(0.0, 0.0, 4.0), 0.0), &car_box(Vec3::ZERO, 0.0)).unwrap();
        assert!((c.overlap - 0.4).abs() < 1e-4, "{c:?}");
        assert!(c.normal.distance(Vec3::Z) < 1e-4);
        // Swapping the boxes flips the normal.
        let c = obb_contact(&car_box(Vec3::ZERO, 0.0), &car_box(Vec3::new(0.0, 0.0, 4.0), 0.0)).unwrap();
        assert!(c.normal.distance(-Vec3::Z) < 1e-4);
    }

    #[test]
    fn a_side_swipe_at_an_angle_overlaps_sideways() {
        // The second car is turned 30 degrees and half a metre too close sideways.
        let c = obb_contact(&car_box(Vec3::new(1.7, 0.0, 0.0), 0.0), &car_box(Vec3::ZERO, 0.5)).unwrap();
        assert!(c.overlap > 0.0 && c.overlap < 2.0);
        assert!(c.normal.dot(Vec3::X) > 0.5, "{:?}", c.normal);
    }

    #[test]
    fn a_head_on_hit_trades_momentum_and_separates_the_cars() {
        let mut a = body(Vec3::new(0.0, 0.0, 4.0), Vec3::new(0.0, 0.0, -10.0), 1500.0);
        let mut b = body(Vec3::ZERO, Vec3::new(0.0, 0.0, 10.0), 1500.0);
        let boxes = (car_box(a.position, 0.0), car_box(b.position, 0.0));
        let hit = obb_contact(&boxes.0, &boxes.1).unwrap();
        separate(&mut a, &mut b, hit.normal, hit.overlap);
        assert!(a.position.z - b.position.z >= 4.4 - 1e-3);
        let contact = BodyContact {
            point: hit.point,
            normal: hit.normal,
            overlap: hit.overlap,
            friction_static: 0.5,
            friction_kinetic: 0.4,
        };
        let h = BodyHit { restitution: 0.2, inertia_scale: Vec3::ONE };
        let before = a.linear_velocity * a.mass() + b.linear_velocity * b.mass();
        let reaction = react_bodies(&mut a, &mut b, &contact, &h, &h).unwrap();
        let after = a.linear_velocity * a.mass() + b.linear_velocity * b.mass();
        assert!((after - before).length() < 1.0, "momentum {before:?} -> {after:?}");
        assert!(reaction.closing_speed > 19.0);
        // At the contact point they no longer approach each other (the corner contact also spins them).
        let closing = (a.point_velocity(hit.point) - b.point_velocity(hit.point)).dot(hit.normal);
        assert!(closing >= -1e-3, "still closing at {closing}");
        // Not approaching: no reaction.
        assert!(react_bodies(&mut a, &mut b, &contact, &h, &h).is_none());
    }

    #[test]
    fn the_lighter_car_gets_the_bigger_change_of_speed() {
        let mut heavy = body(Vec3::new(0.0, 0.0, 4.0), Vec3::new(0.0, 0.0, -5.0), 5000.0);
        let mut light = body(Vec3::ZERO, Vec3::new(0.0, 0.0, 5.0), 1000.0);
        let contact = BodyContact {
            point: Vec3::new(0.0, 0.0, 2.2),
            normal: Vec3::Z,
            overlap: 0.0,
            friction_static: 0.5,
            friction_kinetic: 0.4,
        };
        let h = BodyHit { restitution: 0.1, inertia_scale: Vec3::ONE };
        react_bodies(&mut heavy, &mut light, &contact, &h, &h).unwrap();
        assert!(light.linear_velocity.z < -3.0, "{:?}", light.linear_velocity);
        assert!(heavy.linear_velocity.z.abs() < 4.0);
    }
}
