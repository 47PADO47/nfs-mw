//! Cameras. The games' worlds are Z-up.

mod chase;
mod cut;
#[cfg(test)]
mod depth_tests;
mod fly;
mod orbit;

pub use chase::{ChaseCamera, Followed};
pub use cut::CameraCut;
pub use fly::FlyCamera;
pub use orbit::OrbitCamera;

use blackbox_gfx::Projection;
use glam::{Mat4, Vec3};

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
