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

#[test]
fn moved_runs_are_the_consecutive_flagged_instances_of_a_mesh_run() {
    use super::motion::moved_runs;
    let moved = [false, true, true, false, true, true, true];
    assert_eq!(moved_runs(&moved, 0, 7).collect::<Vec<_>>(), [1..3, 4..7]);
    assert_eq!(moved_runs(&moved, 2, 5).collect::<Vec<_>>(), [2..3, 4..5], "clipped to the mesh run");
    assert_eq!(moved_runs(&moved, 3, 4).count(), 0);
    assert_eq!(moved_runs(&[false; 3], 0, 3).count(), 0);
    let runs: Vec<_> = moved_runs(&moved[..2], 0, 4).collect();
    assert_eq!(runs.len(), 1, "instances past the flags are still");
    assert_eq!(runs[0], 1..2);
}

/// What the fill draw does for one pixel, on the CPU: the world point from the depth the jittered projection
/// wrote, then the vector between the unjittered previous and current projections.
fn fill_vector(jittered: Mat4, current: Mat4, previous: Mat4, world: Vec3) -> Vec2 {
    use super::motion::inverse;
    let clip = jittered * world.extend(1.0);
    let ndc = clip.truncate() / clip.w;
    let h = Mat4::from_cols_array(&inverse(jittered)) * ndc.extend(1.0);
    let rebuilt = h / h.w;
    assert!((rebuilt.truncate() - world).length() < 1e-3 * world.length().max(1.0), "{rebuilt} vs {world}");
    let (cur, prev) = (current * rebuilt, previous * rebuilt);
    prev.truncate().truncate() / prev.w - cur.truncate().truncate() / cur.w
}

#[test]
fn the_fill_draw_gives_the_camera_motion_of_a_still_point_whatever_the_jitter() {
    let current = camera();
    let jitter = jitter_matrix(Vec2::new(0.003, -0.002));
    let point = Vec3::new(1.5, 7.0, 2.0);
    assert_eq!(fill_vector(jitter * current, current, current, point), Vec2::ZERO, "a still camera");
    // The camera moved one metre to the right since the last frame.
    let previous = Projection::PerspectiveInfiniteReverse { fov_y: 1.0, aspect: 16.0 / 9.0, near: 0.1 }.matrix()
        * glam::camera::rh::view::look_at_mat4(Vec3::new(-1.0, -5.0, 1.0), Vec3::new(-1.0, 5.0, 1.0), Vec3::Z);
    let direct = ndc(previous * point.extend(1.0)).truncate() - ndc(current * point.extend(1.0)).truncate();
    let filled = fill_vector(jitter * current, current, previous, point);
    assert!((filled - direct).abs().max_element() < 1e-4, "{filled} vs {direct}");
}

#[test]
fn sky_direction_tracks_rotation_but_not_translation() {
    let projection = Projection::PerspectiveInfiniteReverse { fov_y: 1.0, aspect: 16.0 / 9.0, near: 0.1 }.matrix();
    let current = camera();
    let jittered = jitter_matrix(Vec2::new(0.003, -0.002)) * current;
    let direction = Mat4::from_cols_array(&super::motion::inverse(jittered)) * Vec4::new(0.2, -0.1, 0.0, 1.0);
    let direction = direction.truncate().extend(0.0);
    let moved =
        projection * glam::camera::rh::view::look_at_mat4(Vec3::new(3.0, -5.0, 1.0), Vec3::new(3.0, 5.0, 1.0), Vec3::Z);
    let turned =
        projection * glam::camera::rh::view::look_at_mat4(Vec3::new(0.0, -5.0, 1.0), Vec3::new(1.0, 5.0, 1.0), Vec3::Z);
    let at = ndc(current * direction);
    assert!((ndc(moved * direction) - at).length() < 1e-6);
    assert!((ndc(turned * direction) - at).length() > 0.01);
}
