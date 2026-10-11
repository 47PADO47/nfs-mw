//! A drawable chase-camera pose, separate from the fixed-tick follow state.

use glam::{Mat4, Vec3};

use crate::viewer::camera::view_proj;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ChaseView {
    pub position: Vec3,
    pub(super) target: Vec3,
    pub(super) fov_degrees: f32,
}

impl ChaseView {
    pub(super) fn lerp(self, next: Self, alpha: f32) -> Self {
        Self {
            position: self.position.lerp(next.position, alpha),
            target: self.target.lerp(next.target, alpha),
            fov_degrees: self.fov_degrees + (next.fov_degrees - self.fov_degrees) * alpha,
        }
    }

    pub fn forward(&self) -> Vec3 {
        (self.target - self.position).normalize_or_zero()
    }

    pub fn fov_degrees(&self) -> f32 {
        self.fov_degrees
    }

    pub fn view_proj(&self, aspect: f32) -> Mat4 {
        view_proj(self.position, self.target, self.fov_degrees, aspect, 0.3)
    }
}
