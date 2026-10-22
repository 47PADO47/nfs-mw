//! The front end's camera poses: how the original puts the camera round a car on a stage, from the `frontend` attribute
//! collections `angle*` (spec: `docs/specs/menu-backdrop-looks.md`, section 3).
//!
//! A pose is eleven numbers: a pitch and a yaw that place the eye, a distance, a roll, a field of view and the point the
//! camera looks at. The eye sits `distance` from the stage origin, where the car stands, along the direction the two
//! angles give; the camera then looks at the target, with its up vector turned by the roll. The angles are degrees.

use glam::{Mat3, Vec3};
use nfsmw_data::frontend_camera::CameraAngle;

use crate::viewer::camera::Lens;

/// Where the camera is and what it looks at.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CameraPose {
    /// Degrees. 90 is level with the stage origin; more puts the eye above it.
    pub pitch: f32,
    /// Degrees round the vertical axis.
    pub yaw: f32,
    /// Metres from the stage origin to the eye.
    pub distance: f32,
    /// Degrees the up vector is turned about the view direction.
    pub roll: f32,
    /// Field of view in degrees, as the attributes store it. [unverified] The original does not say which side it spans;
    /// it is read as the horizontal angle of a 4:3 picture (`FOV_ASPECT`), see [`Self::fov_y`].
    pub fov: f32,
    /// The point the camera looks at, metres.
    pub target: Vec3,
}

impl CameraPose {
    /// The eye: `distance` from the origin against the direction the pitch and yaw give (the original takes it from the
    /// inverse of the camera's rotation, which puts it opposite the camera's third axis).
    pub fn eye(&self) -> Vec3 {
        let (sp, cp) = self.pitch.to_radians().sin_cos();
        let (sy, cy) = self.yaw.to_radians().sin_cos();
        -self.distance * Vec3::new(sy * sp, cy * sp, cp)
    }

    /// The up vector: world up turned `roll` degrees about the view direction (right-handed, as a rotation of the
    /// engine's row vectors by a positive angle).
    pub fn up(&self) -> Vec3 {
        let forward = (self.target - self.eye()).normalize_or_zero();
        Mat3::from_axis_angle(forward, self.roll.to_radians()) * Vec3::Z
    }

    /// The pose an attribute collection stores.
    pub fn from_angle(angle: &CameraAngle) -> Self {
        Self {
            pitch: angle.pitch,
            yaw: angle.yaw,
            distance: angle.distance,
            roll: angle.roll,
            fov: angle.fov,
            target: Vec3::from(angle.target),
        }
    }

    /// The vertical field of view the stored angle gives: the stored angle spans the width of a `FOV_ASPECT` picture, and
    /// the vertical angle it implies is kept at every window shape.
    pub fn fov_y(&self) -> f32 {
        let half_x = (self.fov.to_radians() / 2.0).tan();
        2.0 * (half_x / FOV_ASPECT).atan().to_degrees()
    }

    /// The lens for a window of `aspect` (width over height).
    pub fn lens(&self, aspect: f32) -> Lens {
        let eye = self.eye();
        let mut lens = Lens::look_at(eye, self.target, self.fov_y(), NEAR, aspect);
        lens.view = glam::camera::rh::view::look_at_mat4(eye, self.target, self.up());
        lens
    }
}

/// The near plane, metres. [ours]
const NEAR: f32 = 0.05;

/// The shape of the picture the original's field of view angle is read across. [unverified] The safe house's 65 degrees
/// read this way make the car about 1.33 times as tall as a vertical reading does, which fits the original's picture,
/// where it measures about 1.4 times.
const FOV_ASPECT: f32 = 4.0 / 3.0;

/// The pose of the safe house menu (`MainMenu.fng`'s sub menu in a career): the collection `angle10`, which is also
/// `angle_final_career_safehouse`, the camera the safe house move ends on. [exe + verified] The move itself is read from
/// the install (`Stage::camera`); this is the pose it ends on, kept for an install whose attributes cannot be read.
pub const SAFE_HOUSE: CameraPose =
    CameraPose { pitch: 97.3, yaw: 110.0, distance: 4.25, roll: 16.0, fov: 65.0, target: Vec3::new(-0.2, -0.5, 0.3) };

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: Vec3, b: Vec3) -> bool {
        (a - b).length() < 1e-3
    }

    #[test]
    fn the_eye_is_the_distance_from_the_origin() {
        assert!((SAFE_HOUSE.eye().length() - 4.25).abs() < 1e-4);
    }

    #[test]
    fn the_safe_house_eye_is_low_and_behind_the_origin_on_the_far_side_of_the_yaw() {
        // Pitch 97.3 puts the eye a little above the origin, yaw 110 puts it at -x and +y.
        let eye = SAFE_HOUSE.eye();
        assert!(close(eye, Vec3::new(-3.961, 1.442, 0.540)), "{eye:?}");
    }

    #[test]
    fn a_level_pose_has_the_eye_at_the_height_of_the_origin() {
        let level = CameraPose { pitch: 90.0, yaw: 0.0, ..SAFE_HOUSE };
        assert!(level.eye().z.abs() < 1e-4);
        assert!(close(level.eye(), Vec3::new(0.0, -4.25, 0.0)));
    }

    #[test]
    fn no_roll_keeps_the_up_vector_vertical_and_a_roll_turns_it_to_one_side() {
        let upright = CameraPose { roll: 0.0, ..SAFE_HOUSE };
        assert!((upright.up() - Vec3::Z).length() < 1e-5);
        let rolled = SAFE_HOUSE.up();
        assert!((rolled.length() - 1.0).abs() < 1e-5);
        assert!(rolled.dot(Vec3::Z) < 1.0 && rolled.dot(Vec3::Z) > 0.9, "16 degrees of roll leave it mostly up");
    }

    #[test]
    fn the_field_of_view_is_the_horizontal_angle_of_a_4_to_3_picture() {
        // tan(32.5 deg) * 3/4 = tan(25.5 deg): 65 degrees across become 51.1 degrees up.
        assert!((SAFE_HOUSE.fov_y() - 51.1).abs() < 0.1, "{}", SAFE_HOUSE.fov_y());
        let lens = SAFE_HOUSE.lens(16.0 / 9.0);
        assert!((lens.fov_y_degrees - SAFE_HOUSE.fov_y()).abs() < 1e-4);
    }

    #[test]
    fn the_lens_looks_from_the_eye_to_the_target() {
        let lens = SAFE_HOUSE.lens(16.0 / 9.0);
        assert!(close(lens.position, SAFE_HOUSE.eye()));
        let ahead = lens.view.transform_point3(SAFE_HOUSE.target);
        assert!(
            ahead.x.abs() < 1e-3 && ahead.y.abs() < 1e-3 && ahead.z < 0.0,
            "the target is straight ahead: {ahead:?}"
        );
    }
}
