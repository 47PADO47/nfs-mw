//! Four-ray reconstruction quality and strictly non-recursive history regressions.
use crate::{Renderer, gpu::test_support};
use blackbox_gfx::*;
use blackbox_gfx_testkit::mesh::{MeshBuilder, prelit};
use glam::{Mat4, Vec3};
fn mask(r: &Renderer) -> Vec<u8> {
    r.read_raw_texture(r.ray_shadows.as_ref().unwrap().mask.texture(), (320, 240), 2).unwrap()
}
fn spatial(r: &Renderer) -> Vec<u8> {
    let pass = r.ray_shadows.as_ref().unwrap();
    let raw = r.read_raw_texture(pass.spatial.as_ref().unwrap().texture(), (320, 240), 8).unwrap();
    raw.as_chunks::<8>()
        .0
        .iter()
        .map(|p| (half::f16::from_le_bytes([p[0], p[1]]).to_f32() * 255.0).round() as u8)
        .collect()
}
fn error(a: &[u8], reference: &[u8]) -> f64 {
    a.as_chunks::<2>()
        .0
        .iter()
        .zip(reference.as_chunks::<2>().0)
        .map(|(a, b)| {
            let e = f64::from(a[0]) - f64::from(b[0]);
            e * e
        })
        .sum::<f64>()
        / ((a.len() / 2) as f64)
}
#[test]
#[ignore = "requires Vulkan RT and BLACKBOX_RT_REFLECTIONS=1"]
fn four_ray_plane_filter_improves_reference_error_and_history_never_recurses() {
    if std::env::var("BLACKBOX_RT_REFLECTIONS").as_deref() != Ok("1")
        || test_support::test_api() != crate::Backend::Vulkan
    {
        return;
    }
    let _gpu = test_support::serial();
    let mut r = Renderer::headless((320, 240), test_support::options()).unwrap();
    let mut m = MeshBuilder::new();
    m.state(None, BlendMode::Opaque, Shading::Prelit);
    m.flat_quad(
        [
            Vec3::new(-30.0, -30.0, 0.0),
            Vec3::new(30.0, -30.0, 0.0),
            Vec3::new(30.0, 30.0, 0.0),
            Vec3::new(-30.0, 30.0, 0.0),
        ],
        prelit([0.6; 3], 1.0),
    );
    let ground = Instance::new(m.finish().upload(&mut r, "four ray receiver"), Mat4::IDENTITY);
    let mut m = MeshBuilder::new();
    m.state(None, BlendMode::Opaque, Shading::Prelit);
    m.flat_quad(
        [
            Vec3::new(-2.0, -2.0, 20.0),
            Vec3::new(2.0, -2.0, 20.0),
            Vec3::new(2.0, 2.0, 20.0),
            Vec3::new(-2.0, 2.0, 20.0),
        ],
        prelit([0.6; 3], 1.0),
    );
    let roof = Instance::new(m.finish().upload(&mut r, "four ray roof"), Mat4::IDENTITY);
    r.set_shadow_casters(Some(&[roof]));
    let eye = Vec3::new(0.0, -9.0, 2.0);
    let frame = FrameParams {
        view: glam::camera::rh::view::look_at_mat4(eye, Vec3::ZERO, Vec3::Z),
        projection: Projection::PerspectiveInfiniteReverse { fov_y: 0.9, aspect: 320.0 / 240.0, near: 0.1 },
        camera_position: eye,
        light_dir: Vec3::NEG_Z,
        clear_color: [0.0; 3],
        fog: None,
        scattering: None,
        surface_light: None,
        camera_cut: false,
        shadow_tuning: None,
    };
    let settings = RenderExperiments {
        sun_shadows: true,
        ray_sun_shadows: true,
        ray_soft_shadows: true,
        ray_shadow_samples: 4,
        ray_shadow_resolution: 100,
        ray_shadow_filter: 0.0,
        sun_angle: 1.0,
        ..Default::default()
    };
    r.set_render_experiments(settings);
    // Test-only high-ray reference; the public quality control remains bounded at 16.
    r.experiments.ray_shadow_samples = 128;
    r.capture(320, 240, &frame, &[ground]).unwrap();
    let reference = mask(&r);
    r.set_render_experiments(settings);
    r.capture(320, 240, &frame, &[ground]).unwrap();
    let unfiltered = mask(&r);
    r.set_render_experiments(RenderExperiments { ray_shadow_filter: 0.75, ..settings });
    r.capture(320, 240, &frame, &[ground]).unwrap();
    let filtered = mask(&r);
    let raw_error = error(&unfiltered, &reference);
    let filtered_error = error(&filtered, &reference);
    eprintln!("four ray MSE raw={raw_error:.3} filtered={filtered_error:.3}");
    assert!(
        filtered_error < raw_error * 0.8,
        "reconstruction must improve actual penumbra error, not just add gray pixels"
    );
    r.set_render_experiments(RenderExperiments { ray_shadow_resolution: 50, ray_shadow_filter: 0.75, ..settings });
    r.capture(320, 240, &frame, &[ground]).unwrap();
    let reduced_error = error(&mask(&r), &reference);
    eprintln!("half-resolution MSE={reduced_error:.3}");
    assert!(reduced_error < raw_error * 0.8, "lower-resolution reconstruction must still improve over raw four rays");
    assert_eq!(r.ray_shadows.as_ref().unwrap().raw.as_ref().unwrap().texture().width(), 160);
    r.set_render_experiments(RenderExperiments { ray_shadow_temporal: true, ray_shadow_filter: 0.75, ..settings });
    for _ in 0..4 {
        r.capture(320, 240, &frame, &[ground]).unwrap();
    }
    let eye = eye + Vec3::X * 0.04;
    let frame = FrameParams {
        view: glam::camera::rh::view::look_at_mat4(eye, Vec3::ZERO, Vec3::Z),
        camera_position: eye,
        ..frame
    };
    r.capture(320, 240, &frame, &[ground]).unwrap();
    let pass = r.ray_shadows.as_ref().unwrap();
    let stored = r.read_raw_texture(pass.history.as_ref().unwrap().stored().texture(), (320, 240), 8).unwrap();
    let current = r.read_raw_texture(pass.spatial.as_ref().unwrap().texture(), (320, 240), 8).unwrap();
    for (old, now) in stored.as_chunks::<8>().0.iter().zip(current.as_chunks::<8>().0) {
        if half::f16::from_le_bytes([old[2], old[3]]).to_f32() > 0.0 {
            assert_eq!(&old[..2], &now[..2], "history stores the unblended estimate");
        }
    }
    let resolved = mask(&r);
    let current_spatial = spatial(&r);
    assert!(
        resolved.as_chunks::<2>().0.iter().zip(current_spatial).filter(|(a, b)| a[0].abs_diff(*b) > 1).count() > 30,
        "valid reprojected history must contribute"
    );
    r.capture(320, 240, &FrameParams { camera_cut: true, ..frame }, &[ground]).unwrap();
    assert!(
        mask(&r).as_chunks::<2>().0.iter().zip(spatial(&r)).all(|(a, b)| a[0].abs_diff(b) <= 1),
        "camera cuts reject history"
    );
    r.set_shadow_casters(Some(&[]));
    r.capture(320, 240, &frame, &[ground]).unwrap();
    assert!(mask(&r).as_chunks::<2>().0.iter().all(|p| p[0] == 255), "removed blockers leave no temporal trail");
}
