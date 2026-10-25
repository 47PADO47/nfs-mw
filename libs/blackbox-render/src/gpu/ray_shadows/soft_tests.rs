use crate::{Renderer, gpu::test_support};
use blackbox_gfx::*;
use blackbox_gfx_testkit::mesh::{MeshBuilder, prelit};
use glam::{Mat4, Vec3};
fn pixels(r: &Renderer) -> Vec<u8> {
    r.read_raw_texture(r.ray_shadows.as_ref().unwrap().mask.texture(), (320, 240), 2).unwrap()
}
fn partial(p: &[u8]) -> usize {
    p.as_chunks::<2>().0.iter().filter(|p| p[1] == 255 && p[0] > 2 && p[0] < 253).count()
}
#[test]
#[ignore = "requires Vulkan RT and BLACKBOX_RT_REFLECTIONS=1"]
fn finite_sun_broadens_elevated_edges_and_spatial_filter_round_trips_without_history() {
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
    let ground = Instance::new(m.finish().upload(&mut r, "soft sun receiver"), Mat4::IDENTITY);
    let mut m = MeshBuilder::new();
    m.state(None, BlendMode::Opaque, Shading::Prelit);
    m.flat_quad(
        [Vec3::new(-2.0, -2.0, 0.0), Vec3::new(2.0, -2.0, 0.0), Vec3::new(2.0, 2.0, 0.0), Vec3::new(-2.0, 2.0, 0.0)],
        prelit([0.6; 3], 1.0),
    );
    let roof = m.finish().upload(&mut r, "soft sun roof");
    let eye = Vec3::new(0.0, -9.0, 8.0);
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
    let hard =
        RenderExperiments { sun_shadows: true, ray_sun_shadows: true, ray_soft_shadows: false, ..Default::default() };
    r.set_render_experiments(hard);
    r.set_shadow_casters(Some(&[Instance::new(roof, Mat4::from_translation(Vec3::Z * 5.0))]));
    r.capture(320, 240, &frame, &[ground]).unwrap();
    let hard_pixels = pixels(&r);
    assert_eq!(partial(&hard_pixels), 0);
    let soft = RenderExperiments {
        ray_soft_shadows: true,
        ray_shadow_samples: 8,
        ray_shadow_filter: 0.0,
        sun_angle: 2.0,
        ..hard
    };
    r.set_render_experiments(soft);
    r.capture(320, 240, &frame, &[ground]).unwrap();
    let high = pixels(&r);
    assert!(partial(&high) > 200, "elevated blocker must produce a penumbra: {}", partial(&high));
    r.capture(320, 240, &frame, &[ground]).unwrap();
    assert_eq!(high, pixels(&r), "sample pattern is frame independent");
    r.set_shadow_casters(Some(&[Instance::new(roof, Mat4::from_translation(Vec3::Z * 0.08))]));
    r.capture(320, 240, &frame, &[ground]).unwrap();
    let low = pixels(&r);
    assert!(partial(&high) > partial(&low) * 3, "near-contact edges must stay narrower");
    r.set_shadow_casters(Some(&[Instance::new(roof, Mat4::from_translation(Vec3::Z * 5.0))]));
    r.set_render_experiments(RenderExperiments { ray_shadow_filter: 0.75, ..soft });
    r.capture(320, 240, &frame, &[ground]).unwrap();
    let filtered = pixels(&r);
    assert!(partial(&filtered) > 200);
    assert_ne!(high, filtered, "filter reconstructs quantized sample steps");
    r.capture(320, 240, &frame, &[ground]).unwrap();
    assert_eq!(filtered, pixels(&r));
    r.set_render_experiments(hard);
    r.capture(320, 240, &frame, &[ground]).unwrap();
    assert_eq!(hard_pixels, pixels(&r), "hard comparison restored exactly");
    r.set_render_experiments(RenderExperiments { sun_angle: 0.0, ..soft });
    r.capture(320, 240, &frame, &[ground]).unwrap();
    assert_eq!(hard_pixels, pixels(&r), "zero sun radius restores deterministic hard shadows");
    // A soft sun ray can reach beyond the central sunward cylinder.
    let eye = Vec3::new(0.0, -4.0, 3.0);
    let near = FrameParams {
        view: glam::camera::rh::view::look_at_mat4(eye, Vec3::ZERO, Vec3::Z),
        camera_position: eye,
        ..frame
    };
    r.set_shadow_casters(Some(&[Instance::new(roof, Mat4::from_translation(Vec3::new(15.0, -9.3, 600.0)))]));
    r.set_render_experiments(RenderExperiments { ray_shadow_range: 10.0, ray_shadow_length: 800.0, ..soft });
    r.capture(320, 240, &near, &[ground]).unwrap();
    assert!(partial(&pixels(&r)) > 20, "sun-disc extent must participate in caster selection");
}
