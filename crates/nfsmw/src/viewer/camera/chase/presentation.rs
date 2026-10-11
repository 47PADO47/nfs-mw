//! A drawable chase-camera pose, separate from the fixed-tick follow state.

use glam::{Mat4, Vec3};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ChaseView {
    pub position: Vec3,
    pub(super) target: Vec3,
    pub(super) fov_degrees: f32,
    pub(super) shaken: Option<Mat4>,
}

impl ChaseView {
    pub(super) fn lerp(self, next: Self, alpha: f32) -> Self {
        Self {
            position: self.position.lerp(next.position, alpha),
            target: self.target.lerp(next.target, alpha),
            fov_degrees: self.fov_degrees + (next.fov_degrees - self.fov_degrees) * alpha,
            shaken: None,
        }
    }

    pub fn forward(&self) -> Vec3 {
        if let Some(view) = self.shaken {
            return -view.inverse().z_axis.truncate();
        }
        (self.target - self.position).normalize_or_zero()
    }

    pub fn fov_degrees(&self) -> f32 {
        self.fov_degrees
    }

    pub fn view_proj(&self, aspect: f32) -> Mat4 {
        let projection =
            glam::camera::rh::proj::directx::perspective_infinite_reverse(self.fov_degrees.to_radians(), aspect, 0.3);
        projection
            * self.shaken.unwrap_or_else(|| glam::camera::rh::view::look_at_mat4(self.position, self.target, Vec3::Z))
    }

    pub(super) fn shake(mut self, translation: Vec3) -> Self {
        if translation == Vec3::ZERO {
            return self;
        }
        let view = glam::camera::rh::view::look_at_mat4(self.position, self.target, Vec3::Z);
        let shaken = super::shake::apply(view, translation);
        self.position = shaken.inverse().w_axis.truncate();
        self.shaken = Some(shaken);
        self
    }
}
