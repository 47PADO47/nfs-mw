//! Camera movement must not remove an upstream caster from a visible receiver's map.
use super::tests::{dark_pixels, scene};
use crate::gpu::test_support;
use blackbox_gfx::{BlendMode, Instance, RenderBackend, RenderExperiments, Shading};
use blackbox_gfx_testkit::mesh::{MeshBuilder, prelit};
use glam::{Mat4, Vec3};

#[test]
#[ignore = "needs a GPU"]
fn upstream_casters_survive_camera_orbit_and_small_translation_with_and_without_pcss() {
    let _gpu = test_support::serial();
    let Some(mut r) = test_support::headless((240, 160)) else { return };
    let (mut frame, ground, mut casters) = scene(&mut r);
    let sun = Vec3::new(0.0, -0.98, 0.2).normalize();
    frame.light_dir = -sun;
    // Its shadow reaches the origin, even though the caster is behind the camera
    // and beyond the former fixed 180-unit upstream allowance.
    casters[0].transform = Mat4::from_translation(sun * 240.0);
    r.set_shadow_casters(Some(&casters));
    for soft_shadows in [false, true] {
        for angle in [0.0_f32, 0.7, 1.8, 3.1, 4.8] {
            for offset in [-0.05, 0.05] {
                let eye = Vec3::new(angle.sin() * 8.0 + offset, angle.cos() * -8.0, 4.0);
                frame.camera_position = eye;
                frame.view = glam::camera::rh::view::look_at_mat4(eye, Vec3::ZERO, Vec3::Z);
                r.set_render_experiments(Default::default());
                let baseline = r.capture(240, 160, &frame, &ground).unwrap();
                r.set_render_experiments(RenderExperiments {
                    sun_shadows: true,
                    shadow_resolution: 1024,
                    soft_shadows,
                    ..Default::default()
                });
                let image = r.capture(240, 160, &frame, &ground).unwrap();
                let dark = dark_pixels(&baseline, &image);
                assert!(
                    dark > 150,
                    "upstream shadow lost: {dark} pixels, orbit {angle}, shift {offset}, PCSS {soft_shadows}"
                );
                assert_eq!(image, r.capture(240, 160, &frame, &ground).unwrap(), "no frame-rate-dependent history");
            }
        }
    }
}

#[test]
#[ignore = "needs a GPU"]
fn noncasting_sky_and_transparent_bounds_cannot_change_shadow_depth_or_filtering() {
    let _gpu = test_support::serial();
    let Some(mut r) = test_support::headless((240, 160)) else { return };
    let (frame, ground, casters) = scene(&mut r);
    r.set_render_experiments(RenderExperiments { sun_shadows: true, ..Default::default() });
    r.set_shadow_casters(Some(&casters));
    let baseline = r.capture(240, 160, &frame, &ground).unwrap();
    for (blend, shading) in [(BlendMode::Opaque, Shading::Sky), (BlendMode::AlphaBlend, Shading::Prelit)] {
        let mut mesh = MeshBuilder::new();
        mesh.state(None, blend, shading);
        mesh.cuboid(Vec3::splat(-200000.0), Vec3::splat(200000.0), prelit([0.5; 3], 1.0));
        let mesh = mesh.finish().upload(&mut r, "large noncasting geometry");
        let mut candidates = casters.clone();
        candidates.push(Instance::new(mesh, Mat4::IDENTITY));
        r.set_shadow_casters(Some(&candidates));
        assert!(
            baseline == r.capture(240, 160, &frame, &ground).unwrap(),
            "noncasting {shading:?}/{blend:?} must not inflate the physical depth span"
        );
    }
}

#[test]
#[ignore = "needs a GPU"]
fn original_projected_casters_survive_orbit_at_city_coordinates() {
    let _gpu = test_support::serial();
    let Some(mut r) = test_support::headless((240, 160)) else { return };
    let (mut frame, mut ground, mut casters) = scene(&mut r);
    let offset = Vec3::new(2500.0, 3000.0, 200.0);
    let sun = Vec3::new(0.0, -0.98, 0.2).normalize();
    frame.light_dir = -sun;
    for ground in &mut ground {
        ground.transform = Mat4::from_translation(offset) * ground.transform;
    }
    casters[0].transform = Mat4::from_translation(offset + sun * 240.0);
    r.set_shadow_casters(Some(&casters));
    for angle in [0.0_f32, 0.7, 1.8, 3.1, 4.8] {
        let eye = offset + Vec3::new(angle.sin() * 8.0, angle.cos() * -8.0, 4.0);
        frame.camera_position = eye;
        frame.view = glam::camera::rh::view::look_at_mat4(eye, offset, Vec3::Z);
        r.set_render_experiments(Default::default());
        let baseline = r.capture(240, 160, &frame, &ground).unwrap();
        r.set_render_experiments(RenderExperiments {
            sun_shadows: true,
            shadow_projection: Some(super::projected_fit::tests::parameters()),
            shadow_resolution: 2048,
            ..Default::default()
        });
        let image = r.capture(240, 160, &frame, &ground).unwrap();
        let dark = dark_pixels(&baseline, &image);
        assert!(dark > 150, "original projected caster clipped: {dark} dark pixels at orbit {angle}");
        assert_eq!(image, r.capture(240, 160, &frame, &ground).unwrap(), "no previous-frame dependency");
    }
}
