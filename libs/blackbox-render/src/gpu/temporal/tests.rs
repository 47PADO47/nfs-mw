//! The jitter maths of the temporal frame, without a GPU.

use blackbox_gfx::Projection;
use glam::{Mat4, UVec2, Vec2, Vec3, Vec4};

use super::jitter_matrix;

fn camera() -> Mat4 {
    let projection = Projection::PerspectiveInfiniteReverse { fov_y: 1.0, aspect: 16.0 / 9.0, near: 0.1 };
    projection.matrix()
        * glam::camera::rh::view::look_at_mat4(Vec3::new(0.0, -5.0, 1.0), Vec3::new(0.0, 5.0, 1.0), Vec3::Z)
}

fn ndc(clip: Vec4) -> Vec3 {
    clip.truncate() / clip.w
}

#[test]
fn the_jitter_moves_a_point_by_the_offset_and_leaves_its_depth_alone() {
    let point = Vec3::new(1.5, 7.0, 2.0).extend(1.0);
    let still = ndc(camera() * point);
    let moved = ndc(jitter_matrix(Vec2::new(0.01, -0.02)) * camera() * point);
    assert!((moved.x - still.x - 0.01).abs() < 1e-6 && (moved.y - still.y + 0.02).abs() < 1e-6, "{still} {moved}");
    assert_eq!(moved.z, still.z, "depth does not move, so the depth buffer and the colour agree");
}

#[test]
fn a_pixel_jitter_shifts_the_image_right_and_down_by_that_many_render_pixels() {
    let render = UVec2::new(1280, 720);
    let jitter = Vec2::new(0.25, 0.4);
    let point = Vec3::new(-2.0, 9.0, 0.5).extend(1.0);
    let still = ndc(camera() * point);
    let moved = ndc(jitter_matrix(fsr3_wgpu::jitter::projection_offset_ndc(jitter, render)) * camera() * point);
    // Normalised device x runs right and y up, a pixel is 2 / size wide, and pixel rows run down.
    let shift = Vec2::new((moved.x - still.x) * render.x as f32 / 2.0, -(moved.y - still.y) * render.y as f32 / 2.0);
    assert!((shift - jitter).abs().max_element() < 1e-3, "{shift} for {jitter}");
}

#[test]
fn no_jitter_is_the_identity() {
    assert_eq!(jitter_matrix(Vec2::ZERO), Mat4::IDENTITY);
}
