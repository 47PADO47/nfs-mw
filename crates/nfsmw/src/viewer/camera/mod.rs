//! Cameras. The games' worlds are Z-up.

mod chase;
#[cfg(test)]
mod depth_tests;
mod fly;
mod orbit;

pub use chase::{ChaseCamera, Followed};
pub use fly::FlyCamera;
pub use orbit::OrbitCamera;

use glam::{Mat4, Vec3};

/// View-projection for wgpu's clip space (Y up) with reverse Z and no far plane,
/// as `blackbox-render` expects.
pub fn view_proj(eye: Vec3, target: Vec3, fov_y_degrees: f32, aspect: f32, near: f32) -> Mat4 {
    glam::camera::rh::proj::directx::perspective_infinite_reverse(fov_y_degrees.to_radians(), aspect, near)
        * glam::camera::rh::view::look_at_mat4(eye, target, Vec3::Z)
}

/// Unit direction for a heading (radians from +X, counter-clockwise) and pitch (up positive).
pub fn direction(yaw: f32, pitch: f32) -> Vec3 {
    Vec3::new(yaw.cos() * pitch.cos(), yaw.sin() * pitch.cos(), pitch.sin())
}
