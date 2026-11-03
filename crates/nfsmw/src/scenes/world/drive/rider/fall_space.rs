//! The two spaces the fall is worked in: the rider is posed in the render space (x forward, y left, z up) and the ragdoll lives in
//! the physics space (x right, y up, z forward), which is its mirror image: `(x, y, z) -> (-y, z, x)`. A vector is mapped by that;
//! a rotation is conjugated by it, which keeps its angle and turns the axis into `-P axis` (the axis of a rotation is a
//! pseudovector, and the mapping reverses the sense of rotation).

use glam::{Quat, Vec3};

/// A vector or point from render space to physics space.
pub fn to_physics(v: Vec3) -> Vec3 {
    Vec3::new(-v.y, v.z, v.x)
}

/// A vector or point from physics space to render space.
pub fn to_render(v: Vec3) -> Vec3 {
    Vec3::new(v.z, -v.x, v.y)
}

/// A rotation (or an orientation, or a frame given in a body's own mirrored axes) from render space to physics space.
pub fn quat_to_physics(q: Quat) -> Quat {
    let axis = to_physics(Vec3::new(q.x, q.y, q.z));
    Quat::from_xyzw(-axis.x, -axis.y, -axis.z, q.w)
}

/// A rotation from physics space to render space.
pub fn quat_to_render(q: Quat) -> Quat {
    let axis = to_render(Vec3::new(q.x, q.y, q.z));
    Quat::from_xyzw(-axis.x, -axis.y, -axis.z, q.w)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn points_and_rotations_agree_across_the_spaces() {
        let q = Quat::from_euler(glam::EulerRot::XYZ, 0.4, -1.1, 0.7);
        let v = Vec3::new(0.3, -2.0, 1.5);
        // Rotating then mapping is mapping then rotating the mapped rotation.
        assert!((to_physics(q * v) - quat_to_physics(q) * to_physics(v)).length() < 1e-5);
        assert!((to_render(to_physics(v)) - v).length() < 1e-6);
        assert!(quat_to_render(quat_to_physics(q)).dot(q).abs() > 0.99999);
        // A turn to the left (about +z of the render space) is a turn to the left in the physics space: forward z swings to -x.
        let left = quat_to_physics(Quat::from_rotation_z(0.5));
        let turned = left * Vec3::Z;
        assert!(turned.x < -0.4 && turned.z > 0.8, "{turned:?}");
        // The composition is kept.
        let (a, b) = (Quat::from_rotation_x(0.3), Quat::from_rotation_y(-0.8));
        assert!(quat_to_physics(a * b).dot(quat_to_physics(a) * quat_to_physics(b)).abs() > 0.99999);
    }
}
