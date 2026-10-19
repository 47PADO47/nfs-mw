//! The GPS arrow's placement. The mesh is the game's own `MARKER_DIRECTION_AID` (see `gps/mod.rs`).
//!
//! The model is stored in physics axes (x right, y up, z forward): it is thin along y, so its face is the x-z plane, and
//! its point runs along -x (measured on the installed model). It lies flat, face up, with its point along the route.

use glam::{Mat3, Mat4, Quat, Vec3};

/// The model's axes in the world's frame (x forward, y left, z up): model -x (the point) is forward, model y (the thin
/// axis) is up, and model z is left.
const MODEL_TO_FLAT: Mat3 = Mat3::from_cols(Vec3::NEG_X, Vec3::Z, Vec3::Y);

/// The arrow's transform at `origin` (render space): turned to `yaw` (radians about up, the route's direction), scaled by
/// `scale` (the pulse).
pub fn transform(origin: Vec3, yaw: f32, scale: f32) -> Mat4 {
    let orientation = Quat::from_rotation_z(yaw) * Quat::from_mat3(&MODEL_TO_FLAT);
    Mat4::from_scale_rotation_translation(Vec3::splat(scale), orientation, origin)
}
