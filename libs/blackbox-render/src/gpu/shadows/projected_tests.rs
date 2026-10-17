//! Off-camera caster coverage belongs to the map fit, not the main colour frustum.
use super::*;
use blackbox_gfx::Projection;

fn frame(eye: Vec3, sun: Vec3) -> FrameParams {
    FrameParams {
        view: glam::camera::rh::view::look_at_mat4(eye, Vec3::ZERO, Vec3::Z),
        projection: Projection::PerspectiveInfiniteReverse { fov_y: 1.0, aspect: 1.5, near: 0.1 },
        camera_position: eye,
        light_dir: -sun,
        clear_color: [0.0; 3],
        fog: None,
        scattering: None,
        surface_light: None,
        camera_cut: false,
    }
}

fn settings() -> RenderExperiments {
    RenderExperiments {
        sun_shadows: true,
        shadow_projection: Some(projected_fit::tests::parameters()),
        ..Default::default()
    }
}

#[test]
fn original_fit_contains_off_camera_upstream_casters_through_orbit() {
    let sun = Vec3::new(0.0, -0.98, 0.2).normalize();
    let caster = sun * 240.0;
    let mut points = Vec::new();
    let mut excluded_before = false;
    for angle in [0.0_f32, 0.7, 1.8, 3.1, 4.8] {
        let eye = Vec3::new(angle.sin() * 8.0, angle.cos() * -8.0, 4.0);
        let frame = frame(eye, sun);
        let before = fit(&frame, settings(), &[], &mut points).unwrap();
        let old = Mat4::from_cols_array(&before.params.matrices[0]) * caster.extend(1.0);
        excluded_before |= old.w <= 0.0 || old.x.abs() > old.w || old.y.abs() > old.w || old.z < 0.0 || old.z > old.w;
        let after = fit(&frame, settings(), &[(0, caster, 3.0)], &mut points).unwrap();
        let matrix = Mat4::from_cols_array(&after.params.matrices[0]);
        for x in [-3.0, 3.0] {
            for y in [-3.0, 3.0] {
                for z in [-3.0, 3.0] {
                    let p = matrix * (caster + Vec3::new(x, y, z)).extend(1.0);
                    assert!(p.w > 0.0 && p.x.abs() <= p.w && p.y.abs() <= p.w && p.z >= 0.0 && p.z <= p.w);
                }
            }
        }
        assert_eq!(points.len(), 16, "one relevant bound adds exactly eight corners");
    }
    assert!(excluded_before, "regression must exercise a caster excluded by frustum-only fitting");
}

#[test]
fn unrelated_casters_do_not_dilute_original_map_density() {
    let sun = Vec3::new(0.0, -0.98, 0.2).normalize();
    let frame = frame(Vec3::new(0.0, -8.0, 4.0), sun);
    let mut points = Vec::new();
    let a = fit(&frame, settings(), &[], &mut points).unwrap();
    let b = fit(&frame, settings(), &[(0, Vec3::splat(100000.0), 1.0)], &mut points).unwrap();
    assert_eq!(a.params.matrices, b.params.matrices);
    assert_eq!(points.len(), 8);
}

#[test]
fn looking_parallel_to_the_sun_does_not_disable_original_map() {
    let mut frame = frame(Vec3::new(0.0, -8.0, 4.0), Vec3::Z);
    frame.view = Mat4::IDENTITY;
    frame.camera_position = Vec3::ZERO;
    frame.light_dir = Vec3::NEG_Z;
    let map = fit(&frame, settings(), &[], &mut Vec::new()).expect("parallel camera/sun needs a finite basis");
    assert!(Mat4::from_cols_array(&map.params.matrices[0]).is_finite());
}
