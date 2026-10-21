//! Ordered single-precision body transforms. Spec: `docs/specs/pc-object-contact.md`.
use glam::{Mat3, Vec3};

use super::inverse_diagonal;

/// Transform a vector, retaining z/y before x accumulation for contact arithmetic.
pub fn transform_vector(rotation: Mat3, vector: Vec3) -> Vec3 {
    (rotation.z_axis * vector.z + rotation.y_axis * vector.y) + rotation.x_axis * vector.x
}

/// Rotate inverse principal inertia to world space with stable product/sum order.
pub fn inverse_world_inertia(rotation: Mat3, inertia: Vec3) -> Mat3 {
    let inverse = inverse_diagonal(inertia);
    let weighted = [rotation.x_axis * inverse.x, rotation.y_axis * inverse.y, rotation.z_axis * inverse.z];
    let column = |component: usize| {
        let x = weighted[0] * rotation.x_axis[component];
        let y = weighted[1] * rotation.y_axis[component];
        let z = weighted[2] * rotation.z_axis[component];
        Vec3::new((y.x + z.x) + x.x, (x.y + z.y) + y.y, (x.z + z.z) + y.z)
    };
    Mat3::from_cols(column(0), column(1), column(2))
}

#[cfg(test)]
mod tests;
