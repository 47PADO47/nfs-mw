//! Cameras. The games' worlds are Z-up.

mod chase;
mod cut;
#[cfg(test)]
mod depth_tests;
mod dynamic_fov;
mod fly;
mod orbit;

pub use chase::{ChaseCamera, ChaseView, Followed};
pub use cut::CameraCut;
pub use dynamic_fov::DynamicFov;
pub use fly::FlyCamera;
pub use orbit::OrbitCamera;

use blackbox_gfx::Projection;
use glam::{Mat4, Vec3};

/// Everything a frame needs from a camera: where it is, what way it looks, and how it projects.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Lens {
    pub view: Mat4,
    pub projection: Projection,
    pub position: Vec3,
    /// Unit direction the camera looks along.
    pub forward: Vec3,
    pub fov_y_degrees: f32,
}

impl Lens {
    /// A lens at `eye` looking at `target`, with the reverse-Z projection that renderers expect.
    pub fn look_at(eye: Vec3, target: Vec3, fov_y_degrees: f32, near: f32, aspect: f32) -> Self {
        Self {
            view: view(eye, target),
            projection: projection(fov_y_degrees, aspect, near),
            position: eye,
            forward: (target - eye).normalize_or_zero(),
            fov_y_degrees,
        }
    }

    /// World to clip space, for culling.
    pub fn view_proj(&self) -> Mat4 {
        self.projection.matrix() * self.view
    }
}

/// A camera a scene can show. Each kind has its own inputs (a free fly, an orbit about a model, a
/// chase behind a car), so it is driven through its own methods; this is the one thing every scene
/// needs from whichever camera is active.
pub trait Camera {
    fn lens(&self, aspect: f32) -> Lens;

    /// Whether the gameplay camera setting may choose this camera. A camera with its own toggle (the free
    /// and orbit cameras) is not; a new camera is hidden from the setting until it says otherwise.
    fn selectable(&self) -> bool {
        false
    }
}

/// World to view space for a camera at `eye` looking at `target` (the worlds are Z-up).
pub fn view(eye: Vec3, target: Vec3) -> Mat4 {
    glam::camera::rh::view::look_at_mat4(eye, target, Vec3::Z)
}

/// The reverse-Z projection with no far plane that renderers expect.
pub fn projection(fov_y_degrees: f32, aspect: f32, near: f32) -> Projection {
    Projection::PerspectiveInfiniteReverse { fov_y: fov_y_degrees.to_radians(), aspect, near }
}

/// Unit direction for a heading (radians from +X, counter-clockwise) and pitch (up positive).
pub fn direction(yaw: f32, pitch: f32) -> Vec3 {
    Vec3::new(yaw.cos() * pitch.cos(), yaw.sin() * pitch.cos(), pitch.sin())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::viewer::camera::{ChaseCamera, FlyCamera, OrbitCamera};

    #[test]
    fn only_the_chase_camera_is_chosen_from_settings() {
        let fly = FlyCamera { position: Vec3::ZERO, yaw: 0.0, pitch: 0.0, speed: 1.0 };
        let orbit = OrbitCamera { target: Vec3::ZERO, distance: 1.0, yaw: 0.0, pitch: 0.0 };
        assert!(ChaseCamera::default().selectable());
        assert!(!fly.selectable(), "the free camera has its own toggle");
        assert!(!orbit.selectable(), "the orbit camera has its own toggle");
    }
}
