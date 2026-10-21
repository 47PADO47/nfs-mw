//! Synthetic reference measurements. See docs/specs/pc-object-contact.md.
#![allow(clippy::excessive_precision, reason = "Preserve the measured reference values before f32 rounding")]
use super::*;
use glam::{Mat3, Vec3};

fn sample(mass: f32, velocity: Vec3) -> ContactMoment {
    let inertia = box_inertia(mass, Vec3::new(1.0, 1.0, 2.0));
    ContactMoment {
        mass,
        inverse_inertia: Mat3::from_diagonal(inertia.recip()),
        centre: Vec3::ZERO,
        linear_velocity: velocity,
        angular_velocity: Vec3::ZERO,
        elasticity: 0.2,
        immobile: false,
        breaking_impulse: 0.0,
        fixed_centre: false,
    }
}

fn close(actual: Vec3, expected: Vec3) {
    assert!((actual - expected).abs().max_element() < 0.003, "{actual:?} != {expected:?}");
}

fn plane(point: Vec3) -> PlaneContact {
    PlaneContact { point, normal: Vec3::NEG_Z, friction_static: 0.6, friction_kinetic: 0.5 }
}

#[test]
fn reference_head_on() {
    let mut a = sample(1000.0, Vec3::new(0.0, 0.0, 20.0));
    let mut b = sample(1000.0, Vec3::new(0.0, 0.0, -20.0));
    let mut contact = plane(Vec3::new(0.0, 0.0, 0.0));
    contact.friction_kinetic = 0.5;
    contact.friction_static = 0.6;
    let hit = react_moments(&mut a, &mut b, &contact, 32).unwrap();
    assert!((hit.impulse - 20000.0).abs() < 0.03, "{hit:?}");
    assert!((hit.closing_speed - 40.0).abs() < 0.003);
    assert!((hit.sliding_speed - 0.0).abs() < 0.003);
    assert_eq!(hit.friction, FrictionState::Static);
    close(a.linear_velocity, Vec3::new(0.0, 0.0, -3.99999809));
    close(a.angular_velocity, Vec3::new(0.0, 0.0, 0.0));
    assert!(!a.immobile);
    close(b.linear_velocity, Vec3::new(0.0, 0.0, 3.99999809));
    close(b.angular_velocity, Vec3::new(0.0, 0.0, 0.0));
    assert!(!b.immobile);
}

#[test]
fn reference_unequal_mass() {
    let mut a = sample(1000.0, Vec3::new(0.0, 0.0, 30.0));
    let mut b = sample(3000.0, Vec3::new(0.0, 0.0, 0.0));
    let mut contact = plane(Vec3::new(0.0, 0.0, 0.0));
    contact.friction_kinetic = 0.5;
    contact.friction_static = 0.6;
    let hit = react_moments(&mut a, &mut b, &contact, 32).unwrap();
    assert!((hit.impulse - 22500.0).abs() < 0.03, "{hit:?}");
    assert!((hit.closing_speed - 30.0).abs() < 0.003);
    assert!((hit.sliding_speed - 0.0).abs() < 0.003);
    assert_eq!(hit.friction, FrictionState::Static);
    close(a.linear_velocity, Vec3::new(0.0, 0.0, 2.99999809));
    close(a.angular_velocity, Vec3::new(0.0, 0.0, 0.0));
    assert!(!a.immobile);
    close(b.linear_velocity, Vec3::new(0.0, 0.0, 8.99999905));
    close(b.angular_velocity, Vec3::new(0.0, 0.0, 0.0));
    assert!(!b.immobile);
}

#[test]
fn reference_side_glance() {
    let mut a = sample(1000.0, Vec3::new(15.0, 0.0, 30.0));
    let mut b = sample(1000.0, Vec3::new(0.0, 0.0, 0.0));
    let mut contact = plane(Vec3::new(0.8, 0.3, 1.0));
    contact.friction_kinetic = 0.5;
    contact.friction_static = 0.6;
    let hit = react_moments(&mut a, &mut b, &contact, 32).unwrap();
    assert!((hit.impulse - 12746.8408).abs() < 0.03, "{hit:?}");
    assert!((hit.closing_speed - 30.0).abs() < 0.003);
    assert!((hit.sliding_speed - 15.0).abs() < 0.003);
    assert_eq!(hit.friction, FrictionState::Static);
    close(a.linear_velocity, Vec3::new(8.57981491, -1.37927997, 14.7037916));
    close(a.angular_velocity, Vec3::new(-1.92574978, 3.49006939, 1.23394692));
    assert!(!a.immobile);
    close(b.linear_velocity, Vec3::new(6.42018461, 1.37927997, 15.2962084));
    close(b.angular_velocity, Vec3::new(1.92574978, -3.49006939, -1.23394692));
    assert!(!b.immobile);
}

#[test]
fn reference_sliding() {
    let mut a = sample(1000.0, Vec3::new(30.0, 0.0, 5.0));
    let mut b = sample(1000.0, Vec3::new(0.0, 0.0, 0.0));
    let mut contact = plane(Vec3::new(0.0, 0.0, 0.0));
    contact.friction_kinetic = 0.2;
    contact.friction_static = 0.3;
    let hit = react_moments(&mut a, &mut b, &contact, 32).unwrap();
    assert!((hit.impulse - 2499.99976).abs() < 0.03, "{hit:?}");
    assert!((hit.closing_speed - 5.0).abs() < 0.003);
    assert!((hit.sliding_speed - 30.0).abs() < 0.003);
    assert_eq!(hit.friction, FrictionState::Dynamic);
    close(a.linear_velocity, Vec3::new(29.5, 0.0, 2.0));
    close(a.angular_velocity, Vec3::new(0.0, 0.0, 0.0));
    assert!(!a.immobile);
    close(b.linear_velocity, Vec3::new(0.49999997, 0.0, 3.0));
    close(b.angular_velocity, Vec3::new(0.0, 0.0, 0.0));
    assert!(!b.immobile);
}

#[test]
fn reference_static_friction() {
    let mut a = sample(1000.0, Vec3::new(1.0, 0.0, 20.0));
    let mut b = sample(1000.0, Vec3::new(0.0, 0.0, 0.0));
    let mut contact = plane(Vec3::new(0.0, 0.0, 0.0));
    contact.friction_kinetic = 0.3;
    contact.friction_static = 0.6;
    let hit = react_moments(&mut a, &mut b, &contact, 32).unwrap();
    assert!((hit.impulse - 9999.99902).abs() < 0.03, "{hit:?}");
    assert!((hit.closing_speed - 20.0).abs() < 0.003);
    assert!((hit.sliding_speed - 1.0).abs() < 0.003);
    assert_eq!(hit.friction, FrictionState::Static);
    close(a.linear_velocity, Vec3::new(0.5, 0.0, 8.0));
    close(a.angular_velocity, Vec3::new(0.0, 0.0, 0.0));
    assert!(!a.immobile);
    close(b.linear_velocity, Vec3::new(0.49999997, 0.0, 12.0));
    close(b.angular_velocity, Vec3::new(0.0, 0.0, 0.0));
    assert!(!b.immobile);
}

#[test]
fn reference_asymmetric_bounce() {
    let mut a = sample(1000.0, Vec3::new(0.0, 0.0, 20.0));
    a.elasticity = 0.8;
    let mut b = sample(1000.0, Vec3::new(0.0, 0.0, 0.0));
    b.elasticity = 0.1;
    let mut contact = plane(Vec3::new(0.0, 0.0, 0.0));
    contact.friction_kinetic = 0.5;
    contact.friction_static = 0.6;
    let hit = react_moments(&mut a, &mut b, &contact, 32).unwrap();
    assert!((hit.impulse - 9999.99902).abs() < 0.03, "{hit:?}");
    assert!((hit.closing_speed - 20.0).abs() < 0.003);
    assert!((hit.sliding_speed - 0.0).abs() < 0.003);
    assert_eq!(hit.friction, FrictionState::Static);
    close(a.linear_velocity, Vec3::new(0.0, 0.0, 2.0));
    close(a.angular_velocity, Vec3::new(0.0, 0.0, 0.0));
    assert!(!a.immobile);
    close(b.linear_velocity, Vec3::new(0.0, 0.0, 11.0));
    close(b.angular_velocity, Vec3::new(0.0, 0.0, 0.0));
    assert!(!b.immobile);
}

#[test]
fn reference_immobile() {
    let mut a = sample(1000.0, Vec3::new(0.0, 0.0, 20.0));
    let mut b = sample(1000.0, Vec3::new(0.0, 0.0, 0.0));
    b.immobile = true;
    let mut contact = plane(Vec3::new(0.0, 0.0, 0.0));
    contact.friction_kinetic = 0.5;
    contact.friction_static = 0.6;
    let hit = react_moments(&mut a, &mut b, &contact, 32).unwrap();
    assert!((hit.impulse - 20000.0).abs() < 0.03, "{hit:?}");
    assert!((hit.closing_speed - 20.0).abs() < 0.003);
    assert!((hit.sliding_speed - 0.0).abs() < 0.003);
    assert_eq!(hit.friction, FrictionState::Static);
    close(a.linear_velocity, Vec3::new(0.0, 0.0, -3.99999809));
    close(a.angular_velocity, Vec3::new(0.0, 0.0, 0.0));
    assert!(!a.immobile);
    close(b.linear_velocity, Vec3::new(0.0, 0.0, 0.0));
    close(b.angular_velocity, Vec3::new(0.0, 0.0, 0.0));
    assert!(b.immobile);
}

#[test]
fn reference_detach() {
    let mut a = sample(1000.0, Vec3::new(0.0, 0.0, 20.0));
    let mut b = sample(100.0, Vec3::new(0.0, 0.0, 0.0));
    b.immobile = true;
    b.breaking_impulse = 500.0;
    let mut contact = plane(Vec3::new(0.0, 0.0, 0.0));
    contact.friction_kinetic = 0.5;
    contact.friction_static = 0.6;
    let hit = react_moments(&mut a, &mut b, &contact, 32).unwrap();
    assert!((hit.impulse - 1818.18127).abs() < 0.03, "{hit:?}");
    assert!((hit.closing_speed - 20.0).abs() < 0.003);
    assert!((hit.sliding_speed - 0.0).abs() < 0.003);
    assert_eq!(hit.friction, FrictionState::Static);
    close(a.linear_velocity, Vec3::new(0.0, 0.0, 17.818182));
    close(a.angular_velocity, Vec3::new(0.0, 0.0, 0.0));
    assert!(!a.immobile);
    close(b.linear_velocity, Vec3::new(0.0, 0.0, 21.8181782));
    close(b.angular_velocity, Vec3::new(0.0, 0.0, 0.0));
    assert!(!b.immobile);
}

#[test]
fn reference_attached() {
    let mut a = sample(1000.0, Vec3::new(0.0, 0.0, 0.1));
    let mut b = sample(100.0, Vec3::new(0.0, 0.0, 0.0));
    b.immobile = true;
    b.breaking_impulse = 500.0;
    let mut contact = plane(Vec3::new(0.0, 0.0, 0.0));
    contact.friction_kinetic = 0.5;
    contact.friction_static = 0.6;
    let hit = react_moments(&mut a, &mut b, &contact, 32).unwrap();
    assert!((hit.impulse - 99.9999924).abs() < 0.03, "{hit:?}");
    assert!((hit.closing_speed - 0.100000001).abs() < 0.003);
    assert!((hit.sliding_speed - 0.0).abs() < 0.003);
    assert_eq!(hit.friction, FrictionState::Static);
    close(a.linear_velocity, Vec3::new(0.0, 0.0, -0.0200000033));
    close(a.angular_velocity, Vec3::new(0.0, 0.0, 0.0));
    assert!(!a.immobile);
    close(b.linear_velocity, Vec3::new(0.0, 0.0, 0.0));
    close(b.angular_velocity, Vec3::new(0.0, 0.0, 0.0));
    assert!(b.immobile);
}

#[test]
fn reference_fixed() {
    let mut a = sample(1000.0, Vec3::new(0.0, 0.0, 20.0));
    let mut b = sample(1000.0, Vec3::new(0.0, 0.0, 0.0));
    b.fixed_centre = true;
    let mut contact = plane(Vec3::new(0.5, 0.2, 1.0));
    contact.friction_kinetic = 0.5;
    contact.friction_static = 0.6;
    let hit = react_moments(&mut a, &mut b, &contact, 32).unwrap();
    assert!((hit.impulse - 17267.2559).abs() < 0.03, "{hit:?}");
    assert!((hit.closing_speed - 20.0).abs() < 0.003);
    assert!((hit.sliding_speed - 0.0).abs() < 0.003);
    assert_eq!(hit.friction, FrictionState::Static);
    close(a.linear_velocity, Vec3::new(-4.70898056, -1.88359225, -0.72070694));
    close(a.angular_velocity, Vec3::new(-1.35632956, 3.3908236, -1.23491205e-07));
    assert!(!a.immobile);
    close(b.linear_velocity, Vec3::new(0.0, 0.0, 0.0));
    close(b.angular_velocity, Vec3::new(1.35632956, -3.3908236, 1.23491205e-07));
    assert!(!b.immobile);
}

#[test]
fn reference_rotation() {
    let mut a = sample(1000.0, Vec3::new(5.0, 0.0, 20.0));
    let rotation = Mat3::from_rotation_y(0.7);
    let inertia = box_inertia(a.mass, Vec3::new(1.0, 1.0, 2.0)) * Vec3::new(1.0, 3.0, 2.0);
    a.inverse_inertia = rotation * Mat3::from_diagonal(inertia.recip()) * rotation.transpose();
    let mut b = sample(1500.0, Vec3::new(0.0, 0.0, 0.0));
    b.angular_velocity = Vec3::new(0.0, 0.5, 0.0);
    let rotation = Mat3::from_rotation_y(-0.4);
    let inertia = box_inertia(b.mass, Vec3::new(1.0, 1.0, 2.0)) * Vec3::new(1.0, 1.0, 1.0);
    b.inverse_inertia = rotation * Mat3::from_diagonal(inertia.recip()) * rotation.transpose();
    let mut contact = plane(Vec3::new(1.0, 0.4, 0.8));
    contact.friction_kinetic = 0.5;
    contact.friction_static = 0.6;
    let hit = react_moments(&mut a, &mut b, &contact, 32).unwrap();
    assert!((hit.impulse - 9192.75).abs() < 0.03, "{hit:?}");
    assert!((hit.closing_speed - 20.5).abs() < 0.003);
    assert!((hit.sliding_speed - 4.5999999).abs() < 0.003);
    assert_eq!(hit.friction, FrictionState::Static);
    close(a.linear_velocity, Vec3::new(1.69372964, -1.22397578, 8.96869946));
    close(a.angular_velocity, Vec3::new(-2.26645541, 1.67725635, -0.185988143));
    assert!(!a.immobile);
    close(b.linear_velocity, Vec3::new(2.20417976, 0.815983653, 7.35420036));
    close(b.angular_velocity, Vec3::new(1.70693362, -2.85451269, -0.828445554));
    assert!(!b.immobile);
}

#[test]
fn reference_separating() {
    let mut a = sample(1000.0, Vec3::new(0.0, 0.0, -5.0));
    let mut b = sample(1000.0, Vec3::new(0.0, 0.0, 0.0));
    let mut contact = plane(Vec3::new(0.0, 0.0, 0.0));
    contact.friction_kinetic = 0.5;
    contact.friction_static = 0.6;
    assert!(react_moments(&mut a, &mut b, &contact, 32).is_none());
    close(a.linear_velocity, Vec3::new(0.0, 0.0, -5.0));
    close(a.angular_velocity, Vec3::new(0.0, 0.0, 0.0));
    assert!(!a.immobile);
    close(b.linear_velocity, Vec3::new(0.0, 0.0, 0.0));
    close(b.angular_velocity, Vec3::new(0.0, 0.0, 0.0));
    assert!(!b.immobile);
}

#[test]
fn reference_stationary() {
    let mut a = sample(1000.0, Vec3::new(0.0, 0.0, 0.0));
    let mut b = sample(1000.0, Vec3::new(0.0, 0.0, 0.0));
    let mut contact = plane(Vec3::new(0.0, 0.0, 0.0));
    contact.friction_kinetic = 0.5;
    contact.friction_static = 0.6;
    assert!(react_moments(&mut a, &mut b, &contact, 32).is_none());
    close(a.linear_velocity, Vec3::new(0.0, 0.0, 0.0));
    close(a.angular_velocity, Vec3::new(0.0, 0.0, 0.0));
    assert!(!a.immobile);
    close(b.linear_velocity, Vec3::new(0.0, 0.0, 0.0));
    close(b.angular_velocity, Vec3::new(0.0, 0.0, 0.0));
    assert!(!b.immobile);
}

#[test]
fn reference_near_tangent_momentum_cancels_in_the_original_axis_order() {
    let velocity = Vec3::new(10.0, -10.0, -0.0000001);
    let mut a = sample(1000.0, velocity);
    let mut b = sample(1000.0, Vec3::ZERO);
    let contact = PlaneContact { point: Vec3::ZERO, normal: Vec3::ONE, friction_static: 0.0, friction_kinetic: 0.0 };
    // Original-machine-code measurement gives an exactly non-approaching
    // projection for these synthetic inputs.
    assert!(react_moments(&mut a, &mut b, &contact, 32).is_none());
    assert_eq!(a.linear_velocity, velocity);
    assert_eq!(b.linear_velocity, Vec3::ZERO);
}

#[test]
fn reference_near_tangent_projection_combines_x_z_before_y() {
    let velocity = Vec3::new(-0.0000001, 10.0, -10.0);
    let mut a = sample(1000.0, velocity);
    let mut b = sample(1000.0, Vec3::ZERO);
    let contact = PlaneContact { point: Vec3::ZERO, normal: Vec3::ONE, friction_static: 0.0, friction_kinetic: 0.0 };
    // The original returns false. Adding y/z before x leaves a tiny negative
    // projection and incorrectly reports a reaction.
    assert!(react_moments(&mut a, &mut b, &contact, 32).is_none());
    assert_eq!(a.linear_velocity, velocity);
    assert_eq!(b.linear_velocity, Vec3::ZERO);
}
