//! The depth contract that docs/reshade.md promises: reverse Z, 1 at the near plane, `near / z`
//! further away, no far plane. Injected effects (ReShade, vkBasalt) rely on it.

use glam::{Vec3, Vec4};

use super::{projection, view};

/// The depth the renderer writes for a point `distance` metres straight ahead of the camera.
fn depth_ahead(near: f32, distance: f32) -> f32 {
    let eye = Vec3::ZERO;
    let view_proj = projection(60.0, 16.0 / 9.0, near).matrix() * view(eye, Vec3::X);
    let clip = view_proj * Vec4::new(distance, 0.0, 0.0, 1.0);
    clip.z / clip.w
}

#[test]
fn depth_is_one_at_the_near_plane_and_falls_as_near_over_distance() {
    for near in [0.3_f32, 0.5, 1.0] {
        assert!((depth_ahead(near, near) - 1.0).abs() < 1e-5, "near plane at {near} m");
        for distance in [near * 2.0, near * 10.0, near * 1000.0] {
            let expected = near / distance;
            let got = depth_ahead(near, distance);
            assert!((got - expected).abs() < 1e-5, "near {near}, distance {distance}: {got} vs {expected}");
        }
    }
}

#[test]
fn there_is_no_far_plane() {
    assert!(depth_ahead(0.3, 1.0e6) > 0.0, "a point a thousand kilometres away is still in front of the far end");
}
