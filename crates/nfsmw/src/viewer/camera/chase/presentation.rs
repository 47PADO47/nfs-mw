//! A drawable chase-camera pose, separate from the fixed-tick follow state.

use glam::{Mat4, Vec3};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ChaseView {
    pub position: Vec3,
    pub(super) target: Vec3,
    pub(super) fov_degrees: f32,
    pub(super) shaken: Option<Mat4>,
}

use super::super::{Camera, Lens};

/// The near plane of the chase lens, metres.
const NEAR_PLANE: f32 = 0.3;

impl Camera for ChaseView {
    fn lens(&self, aspect: f32) -> Lens {
        self.lens_with_fov(aspect, self.fov_degrees)
    }

    fn selectable(&self) -> bool {
        true
    }
}

impl ChaseView {
    /// The field of view the chase camera asks for, degrees (before any high-speed kick).
    pub fn fov_degrees(&self) -> f32 {
        self.fov_degrees
    }

    /// The point the view looks at.
    pub fn target(&self) -> Vec3 {
        self.target
    }

    /// The view re-aimed by a pursuit shot. It has no shake: the shake was for the chase pose.
    pub fn reframed(position: Vec3, target: Vec3, fov_degrees: f32) -> Self {
        Self { position, target, fov_degrees, shaken: None }
    }

    /// The lens with a given field of view; a shaken view keeps its shake.
    pub fn lens_with_fov(&self, aspect: f32, fov_y_degrees: f32) -> Lens {
        let mut lens = Lens::look_at(self.position, self.target, fov_y_degrees, NEAR_PLANE, aspect);
        if let Some(view) = self.shaken {
            lens.view = view;
        }
        lens.forward = self.forward();
        lens
    }

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

    #[cfg(test)]
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
