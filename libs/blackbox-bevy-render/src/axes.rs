//! The fixed basis change between the games' world and Bevy's.
//!
//! The games' world is right-handed with **z up** (x right, y forward). Bevy's is right-handed with **y up** (and
//! forward is -z). Bevy features such as the atmosphere and ray-traced sky assume y up, so the backend applies one
//! rotation to every transform, the view and the light directions. Callers never see it.
//!
//! `(x, y, z)` becomes `(x, z, -y)`: a rotation of -90 degrees about x, so winding and handedness are kept.

use glam::{Mat4, Vec3};

/// Game world to Bevy world.
pub const Z_UP_TO_Y_UP: Mat4 = Mat4::from_cols_array(&[
    1.0, 0.0, 0.0, 0.0, // x stays x
    0.0, 0.0, -1.0, 0.0, // game y (forward) becomes Bevy -z
    0.0, 1.0, 0.0, 0.0, // game z (up) becomes Bevy y
    0.0, 0.0, 0.0, 1.0,
]);

/// A point or direction of the game world, in Bevy's axes.
pub fn point(p: Vec3) -> Vec3 {
    Vec3::new(p.x, p.z, -p.y)
}

/// An object-to-world transform of the game world, as an object-to-world transform of Bevy's.
pub fn model(m: Mat4) -> Mat4 {
    Z_UP_TO_Y_UP * m
}

/// The camera's own placement in the Bevy world, from the game's world-to-view matrix.
///
/// Bevy views down -z like the games' right-handed look-at, so the view space needs no change; only the world
/// it sits in does.
pub fn camera_world(view: Mat4) -> Mat4 {
    Z_UP_TO_Y_UP * view.inverse()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn up_becomes_up_and_forward_becomes_minus_z() {
        assert_eq!(point(Vec3::Z), Vec3::Y);
        assert_eq!(point(Vec3::Y), Vec3::NEG_Z);
        assert_eq!(point(Vec3::X), Vec3::X);
    }

    #[test]
    fn the_basis_change_is_a_proper_rotation() {
        assert!((Z_UP_TO_Y_UP.determinant() - 1.0).abs() < 1e-6);
        assert!(((Z_UP_TO_Y_UP * Z_UP_TO_Y_UP.transpose()) - Mat4::IDENTITY).abs_diff_eq(Mat4::ZERO, 1e-6));
    }

    #[test]
    fn the_matrix_agrees_with_the_point_helper() {
        for p in [Vec3::new(1.0, 2.0, 3.0), Vec3::new(-4.0, 0.5, 9.0)] {
            assert_eq!(Z_UP_TO_Y_UP.transform_point3(p), point(p));
        }
    }

    #[test]
    fn a_camera_looking_along_game_y_looks_along_minus_z() {
        let view = glam::camera::rh::view::look_at_mat4(Vec3::new(1.0, 2.0, 3.0), Vec3::new(1.0, 12.0, 3.0), Vec3::Z);
        let world = camera_world(view);
        let forward = world.transform_vector3(Vec3::NEG_Z);
        assert!((forward - Vec3::NEG_Z).length() < 1e-5, "{forward:?}");
        let up = world.transform_vector3(Vec3::Y);
        assert!((up - Vec3::Y).length() < 1e-5, "{up:?}");
        assert!((world.transform_point3(Vec3::ZERO) - point(Vec3::new(1.0, 2.0, 3.0))).length() < 1e-5);
    }

    #[test]
    fn models_move_with_their_world() {
        let m = Mat4::from_translation(Vec3::new(0.0, 5.0, 2.0));
        assert!((model(m).transform_point3(Vec3::ZERO) - Vec3::new(0.0, 2.0, -5.0)).length() < 1e-6);
    }
}
