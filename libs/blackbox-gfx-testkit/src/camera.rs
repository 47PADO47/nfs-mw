//! A look-at camera that produces the reverse-Z frame parameters the renderers expect.

use blackbox_gfx::FrameParams;
use glam::{Mat4, Vec3};

/// A perspective camera in the games' z-up world, with an infinite far plane and reverse Z.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Camera {
    pub eye: Vec3,
    pub target: Vec3,
    pub fov_y_degrees: f32,
    pub near: f32,
}

impl Camera {
    pub fn new(eye: Vec3, target: Vec3) -> Self {
        Self { eye, target, fov_y_degrees: 60.0, near: 0.1 }
    }

    /// World to clip space for a surface of the given width over height.
    pub fn view_proj(&self, aspect: f32) -> Mat4 {
        let view = glam::camera::rh::view::look_at_mat4(self.eye, self.target, Vec3::Z);
        let projection = glam::camera::rh::proj::directx::perspective_infinite_reverse(
            self.fov_y_degrees.to_radians(),
            aspect,
            self.near,
        );
        projection * view
    }

    /// Frame parameters for this camera. `fog` is the (start, end) distance of the linear fog towards
    /// `clear_color`, or `None` for no fog.
    pub fn frame(&self, aspect: f32, clear_color: [f32; 3], fog: Option<(f32, f32)>) -> FrameParams {
        let (fog_start, fog_end) = fog.unwrap_or((f32::MAX, f32::MAX));
        FrameParams {
            view_proj: self.view_proj(aspect),
            camera_position: self.eye,
            light_dir: Vec3::new(-0.4, 0.3, -0.85).normalize(),
            clear_color,
            fog_start,
            fog_end,
        }
    }

    /// The camera's right and up vectors, for building billboards that face it.
    pub fn right_up(&self) -> (Vec3, Vec3) {
        let forward = (self.target - self.eye).normalize_or_zero();
        let right = forward.cross(Vec3::Z).normalize_or_zero();
        (right, right.cross(forward))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_point_in_front_maps_to_the_screen_centre_with_reverse_z() {
        let camera = Camera::new(Vec3::new(0.0, -5.0, 1.0), Vec3::new(0.0, 5.0, 1.0));
        let near = camera.view_proj(1.0) * Vec3::new(0.0, -4.0, 1.0).extend(1.0);
        let far = camera.view_proj(1.0) * Vec3::new(0.0, 5000.0, 1.0).extend(1.0);
        let (near, far) = (near / near.w, far / far.w);
        assert!(near.x.abs() < 1e-5 && near.y.abs() < 1e-5);
        assert!(near.z > far.z && far.z > 0.0, "reverse Z: nearer is larger, never past zero");
    }

    #[test]
    fn no_fog_uses_the_max_convention() {
        let frame = Camera::new(Vec3::ZERO, Vec3::Y).frame(1.0, [0.0; 3], None);
        assert_eq!((frame.fog_start, frame.fog_end), (f32::MAX, f32::MAX));
        let fogged = Camera::new(Vec3::ZERO, Vec3::Y).frame(1.0, [0.0; 3], Some((5.0, 9.0)));
        assert_eq!((fogged.fog_start, fogged.fog_end), (5.0, 9.0));
    }

    #[test]
    fn billboard_axes_are_orthonormal() {
        let (right, up) = Camera::new(Vec3::new(3.0, -4.0, 2.0), Vec3::new(0.0, 6.0, 1.0)).right_up();
        assert!((right.length() - 1.0).abs() < 1e-5 && (up.length() - 1.0).abs() < 1e-5);
        assert!(right.dot(up).abs() < 1e-5);
    }
}
