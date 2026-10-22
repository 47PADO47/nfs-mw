//! Where a scene sits in the world: the scene frame, the render frame and the world frame, and the transform between
//! them. Rules: `docs/specs/nis-player.md`, section 5.
//!
//! Three frames appear. The **scene frame** is the scene's authored coordinates, with `+z` up. The **render frame** is
//! the same up axis (`+z`) and is where a marker origin is given. The **world frame** has `+y` up. The map from a render
//! point `(X, Y, Z)` to a world point is `(-Y, Z, X)`. That map has determinant -1, so the two frames differ in
//! handedness, and the map is a reflection. This is the spec's rule, kept as it is.
//!
//! Angles: a heading is a 16-bit unit count, 65,536 to a turn. The turn applied to a scene at a marker is the heading
//! minus a quarter turn. The spec does not give the sign of that rotation about `+z`; this module uses the right-handed
//! rotation of `glam::Mat4::from_rotation_z` [unconfirmed sign].

use std::f32::consts::TAU;

use glam::{Mat4, Vec3, Vec4};

use crate::SceneType;

/// 16-bit angle units in one turn.
pub const TURN_UNITS: u32 = 0x1_0000;
/// A quarter turn in 16-bit angle units.
const QUARTER_TURN: u16 = (TURN_UNITS / 4) as u16;

/// The map from render coordinates to world coordinates: `(X, Y, Z) -> (-Y, Z, X)`. Its columns are the images of the
/// render axes.
pub fn render_to_world() -> Mat4 {
    Mat4::from_cols(Vec4::Z, Vec4::new(-1.0, 0.0, 0.0, 0.0), Vec4::Y, Vec4::W)
}

/// The transform from scene coordinates to render coordinates for a scene at a marker.
///
/// For types 0, 2 and 3 the marker origin is a point in the render frame (its height already set to the ground, which
/// the caller finds). The scene is turned about `+z` by the marker heading minus a quarter turn, then shifted to the
/// origin. For types 1 and 4 the marker is ignored and the transform is the identity.
pub fn scene_to_render(scene_type: SceneType, marker_origin: Vec3, marker_heading: u16) -> Mat4 {
    if !scene_type.places_at_marker() {
        return Mat4::IDENTITY;
    }
    let turn_units = marker_heading.wrapping_sub(QUARTER_TURN);
    let angle = f32::from(turn_units) * (TAU / TURN_UNITS as f32);
    Mat4::from_translation(marker_origin) * Mat4::from_rotation_z(angle)
}

/// The transform from scene coordinates to world coordinates: [`scene_to_render`] followed by [`render_to_world`].
pub fn scene_to_world(scene_type: SceneType, marker_origin: Vec3, marker_heading: u16) -> Mat4 {
    render_to_world() * scene_to_render(scene_type, marker_origin, marker_heading)
}

/// The inverse of [`scene_to_world`]: world coordinates to scene coordinates, for the same marker.
pub fn world_to_scene(scene_type: SceneType, marker_origin: Vec3, marker_heading: u16) -> Mat4 {
    scene_to_world(scene_type, marker_origin, marker_heading).inverse()
}
