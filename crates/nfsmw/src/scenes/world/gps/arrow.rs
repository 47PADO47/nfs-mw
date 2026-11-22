//! The GPS arrow's placement. The mesh is the game's own `MARKER_DIRECTION_AID` (see `gps/mod.rs`).
//!
//! The model is stored in physics axes (x right, y up, z forward). Measured on the installed model
//! (`GLOBAL/InGameA.bun`, 256 vertices): it is thin along y (0.31 m), symmetric across x (-1.11 to 1.11 m), and runs along
//! z from its point at -1.78 m (0.14 m wide), through the head's base (2.2 m wide), to the end of its shaft at 1.24 m
//! (0.85 m wide). So its face is the x-z plane and its point runs along -z (`docs/specs/gps-route.md`). It lies flat, face
//! up, with its point along the route.

use glam::{Mat3, Mat4, Quat, Vec3};

/// The model's point, in the model's own axes.
pub const MODEL_POINT: Vec3 = Vec3::NEG_Z;

/// The model's axes in the world's frame (x forward, y left, z up): model -z (the point) is forward, model y (the thin
/// axis) is up, and model x is right. A rotation, so the face keeps its side.
const MODEL_TO_FLAT: Mat3 = Mat3::from_cols(Vec3::NEG_Y, Vec3::Z, Vec3::NEG_X);

/// The arrow's transform at `origin` (render space): turned to `yaw` (radians about up, the route's direction), scaled by
/// `scale` (the pulse).
pub fn transform(origin: Vec3, yaw: f32, scale: f32) -> Mat4 {
    let orientation = Quat::from_rotation_z(yaw) * Quat::from_mat3(&MODEL_TO_FLAT);
    Mat4::from_scale_rotation_translation(Vec3::splat(scale), orientation, origin)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The direction on the ground the drawn point takes at `yaw`.
    fn point_at(yaw: f32) -> Vec3 {
        transform(Vec3::ZERO, yaw, 1.0).transform_vector3(MODEL_POINT)
    }

    #[test]
    fn the_point_runs_along_the_route() {
        for yaw in [0.0_f32, 0.5, -1.2, 2.8] {
            let point = point_at(yaw);
            let wanted = Vec3::new(yaw.cos(), yaw.sin(), 0.0);
            assert!(point.distance(wanted) < 1e-5, "yaw {yaw}: the point goes {point}, the route {wanted}");
        }
    }

    #[test]
    fn the_arrow_lies_flat_with_its_thin_axis_up() {
        let up = transform(Vec3::ZERO, 0.7, 1.0).transform_vector3(Vec3::Y);
        assert!(up.distance(Vec3::Z) < 1e-5, "the model's thin axis goes {up}");
        assert!((MODEL_TO_FLAT.determinant() - 1.0).abs() < 1e-6, "a rotation, not a mirror");
    }
}
