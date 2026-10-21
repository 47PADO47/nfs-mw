//! Measured original-helper outputs for independently chosen synthetic inputs.
use super::*;

fn orientation() -> Mat3 {
    Mat3::from_cols(
        Vec3::new(0.66, 0.33776388, -0.67105556),
        Vec3::new(-0.017763888, 0.9, 0.43552777),
        Vec3::new(0.75105554, -0.27552778, 0.6),
    )
}

#[test]
fn rotated_centre_preserves_measured_accumulation_order() {
    let result = transform_vector(orientation(), Vec3::new(0.37, -0.19, 0.61));
    assert_eq!(result.to_array().map(f32::to_bits), [1060416001, 3193650392, 1024405852]);
}

#[test]
fn rotated_inverse_tensor_preserves_both_products_and_component_orders() {
    let tensor = inverse_world_inertia(orientation(), Vec3::new(333.33334, 333.33334, 133.33333));
    assert_eq!(
        tensor.to_cols_array().map(f32::to_bits),
        [1001749325, 3128171720, 990176686, 3128171723, 995819284, 3124954106, 990176687, 3124954104, 999777119,]
    );
}

#[test]
fn rounded_centre_keeps_the_original_near_tangent_reaction_branch() {
    use crate::rigid_body::{ContactMoment, FrictionState, PlaneContact, react_moments};
    let rotation = orientation();
    let cog = Vec3::new(0.37, -0.19, 0.61);
    let mut a = ContactMoment {
        mass: 200.0,
        inverse_inertia: inverse_world_inertia(rotation, Vec3::new(333.33334, 333.33334, 133.33333)),
        centre: transform_vector(rotation, cog),
        linear_velocity: Vec3::new(0.034959197, 0.0, 0.0),
        angular_velocity: Vec3::Y,
        elasticity: 0.0,
        immobile: false,
        breaking_impulse: 0.0,
        fixed_centre: false,
    };
    let mut b = ContactMoment {
        mass: 500.0,
        inverse_inertia: Mat3::ZERO,
        centre: Vec3::ZERO,
        linear_velocity: Vec3::ZERO,
        angular_velocity: Vec3::ZERO,
        elasticity: 0.0,
        immobile: true,
        breaking_impulse: 0.0,
        fixed_centre: false,
    };
    let contact = PlaneContact { point: Vec3::ZERO, normal: Vec3::NEG_X, friction_static: 0.5, friction_kinetic: 0.3 };
    let mut reordered = a;
    reordered.centre = rotation * cog;
    assert!(react_moments(&mut reordered, &mut b, &contact, 32).is_none());
    let hit = react_moments(&mut a, &mut b, &contact, 32).unwrap();
    assert_eq!(hit.friction, FrictionState::Dynamic);
    assert!((hit.impulse - 0.000005052425).abs() < 1e-10);
    assert_eq!(hit.closing_speed.to_bits(), (1.4901161e-8_f32).to_bits());
}
