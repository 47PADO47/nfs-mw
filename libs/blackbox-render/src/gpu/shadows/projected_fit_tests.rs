//! Independent original-instruction matrices; synthetic view-space inputs only.
use super::*;

pub(in crate::gpu::shadows) fn parameters() -> ShadowProjection {
    ShadowProjection {
        near: 20.1,
        frustum: [0.01, 50.0],
        depth_bias: [4.0, 4.0],
        width_scale: 1.5,
        height_scale: 10.0,
        far_scale: 10.0,
        offset: [0.0; 2],
        post_transform: [[1.0, 0.0, 0.0, 0.0], [0.0, 0.0, -0.5, 0.0], [0.0, 2.0, 0.0, 0.0], [0.0, -1.0, 0.5, 1.0]],
    }
}

fn points() -> Vec<Vec3> {
    let mut points = Vec::new();
    for z in [0.01, 50.0] {
        for x in [-1.0, 1.0] {
            for y in [-1.0, 1.0] {
                points.push(Vec3::new(x * z * 0.8, y * z * 0.45, z));
            }
        }
    }
    points
}

#[test]
fn fitted_projection_matches_independent_original_instruction_matrices() {
    let cases = [
        (
            "baseline",
            [0.3094264, 0.7219949, -0.6188527],
            [1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0],
            [0.0, 0.0],
            20.1,
            [
                -0.7564, 0.2560414, 0.1114434, 0.243778, 0.3241714, 0.5974301, 0.2600345, 0.5688154, 0.0, 0.8250224,
                0.4136448, 0.785507, 0.0, -20.10351, 9.042925, 20.09665,
            ],
        ),
        (
            "tilted_sun",
            [-0.4444444, 0.1111111, -0.8888889],
            [1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0],
            [0.0, 0.0],
            20.1,
            [
                -0.1528278, -0.8981689, -0.4173708, -0.8623489, -0.6113113, 0.2245422, 0.1043427, 0.2155872, 0.0,
                0.4771522, 0.2566686, 0.4581228, 0.0, -2.978657, 17.26411, 36.53852,
            ],
        ),
        (
            "rotated_view",
            [0.3094264, 0.7219949, -0.6188527],
            [0.921061, 0.0, -0.3894183, 0.0, 0.0, 1.0, 0.0, 0.0, 0.3894183, 0.0, 0.921061, 0.0, 3.0, -2.0, 1.0, 1.0],
            [0.0, 0.0],
            20.1,
            [
                -0.6966905,
                -0.08544908,
                -0.05843475,
                -0.08135639,
                0.3241714,
                0.5974301,
                0.2600345,
                0.5688154,
                -0.294556,
                0.8596032,
                0.4243902,
                0.8184316,
                -2.917543,
                -19.70523,
                9.270831,
                20.47586,
            ],
        ),
        (
            "fit_offsets",
            [0.3094264, 0.7219949, -0.6188527],
            [1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0],
            [2.0, -3.0],
            20.1,
            [
                -0.7246379, 0.2560414, 0.1123996, 0.243778, 0.3105591, 0.5974301, 0.2622658, 0.5688154, 0.0, 0.8250224,
                0.4117323, 0.785507, -1.576765, -20.10351, 9.042962, 20.09665,
            ],
        ),
        (
            "near_parameter",
            [0.3094264, 0.7219949, -0.6188527],
            [1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0],
            [0.0, 0.0],
            40.1,
            [
                -1.024985, 0.263746, 0.101053, 0.243778, 0.4392793, 0.6154074, 0.2357903, 0.5688154, 0.0, 0.8498483,
                0.4344255, 0.785507, 0.0, -40.10362, 18.04284, 40.09665,
            ],
        ),
        (
            "caster_bounds",
            [0.3094264, 0.7219949, -0.6188527],
            [1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0],
            [0.0, 0.0],
            20.1,
            [
                -0.2107865, 0.2524901, 0.1021069, 0.243778, 0.09033707, 0.5891436, 0.2382494, 0.5688154, 0.0,
                0.8135793, 0.4323178, 0.785507, -2.55955, 1.922749, 19.42856, 41.36286,
            ],
        ),
    ];
    for (label, direction, source_view, offset, near, expected) in cases {
        let mut points = points();
        if label == "caster_bounds" {
            for x in [-80.0, 60.0] {
                for y in [-10.0, 20.0] {
                    for z in [5.0, 60.0] {
                        points.push(Vec3::new(x, y, z));
                    }
                }
            }
        }
        let mut parameters = parameters();
        parameters.offset = offset;
        parameters.near = near;
        let view = Mat4::from_cols_array(&source_view);
        let fit = fit_view_points(view, Vec3::from_array(direction), &points, &parameters).expect(label);
        for (actual, expected) in fit.clip_from_world.to_cols_array().into_iter().zip(expected) {
            assert!((actual - expected).abs() < 0.00002, "{label}: {actual} != {expected}");
        }
        assert_eq!(fit.near, near);
        assert!(fit.far > fit.near);
        assert!(fit.extent.min_element() > 0.0);
        let inverse = view.inverse();
        let world_points: Vec<_> = points.iter().map(|&p| inverse.transform_point3(p)).collect();
        let world_light = inverse.transform_vector3(Vec3::from_array(direction));
        let from_world = fit_world_points(view, world_light, &world_points, &parameters).unwrap();
        for (a, b) in from_world.clip_from_world.to_cols_array().into_iter().zip(fit.clip_from_world.to_cols_array()) {
            assert!((a - b).abs() < 0.00002);
        }
    }
}

#[test]
fn projected_texture_bias_tracks_resolution_without_changing_clip_depth() {
    for resolution in [1024, 2048, 4096] {
        let bias = texture_bias(resolution).unwrap();
        let input = Vec4::new(0.4, -0.2, 0.3, 2.0);
        let output = bias * input;
        let center = 0.5 + 0.5 / resolution as f32;
        assert!((output.x - (0.2 + 2.0 * center)).abs() < 1e-7);
        assert!((output.y - (0.1 + 2.0 * center)).abs() < 1e-7);
        assert_eq!(output.z, input.z);
        assert_eq!(output.w, input.w);
    }
    assert!(texture_bias(0).is_none());
}

#[test]
fn invalid_or_degenerate_projection_inputs_fail_explicitly() {
    let mut p = parameters();
    assert!(fit_view_points(Mat4::IDENTITY, Vec3::Y, &[], &p).is_none());
    assert!(fit_view_points(Mat4::ZERO, Vec3::Y, &points(), &p).is_none());
    for light in [Vec3::ZERO, Vec3::splat(f32::NAN)] {
        assert!(fit_view_points(Mat4::IDENTITY, light, &points(), &p).is_none());
    }
    assert!(
        fit_view_points(Mat4::IDENTITY, Vec3::Z, &points(), &p).is_some(),
        "parallel sun uses a perpendicular fallback"
    );
    assert!(fit_view_points(Mat4::IDENTITY, Vec3::Y, &[Vec3::splat(f32::NAN)], &p).is_none());
    for near in [0.0, -1.0, f32::INFINITY] {
        p.near = near;
        assert!(fit_view_points(Mat4::IDENTITY, Vec3::Y, &points(), &p).is_none());
    }
    p = parameters();
    p.width_scale = 0.0;
    assert!(fit_view_points(Mat4::IDENTITY, Vec3::Y, &points(), &p).is_none());
    p = parameters();
    p.post_transform = [[0.0; 4]; 4];
    assert!(fit_view_points(Mat4::IDENTITY, Vec3::Y, &points(), &p).is_none());
}
