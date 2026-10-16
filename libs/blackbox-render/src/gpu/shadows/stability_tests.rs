//! Image regressions for sloped art, calibrated baked lighting and diagnostic views.
use crate::{Renderer, gpu::test_support};
use blackbox_gfx::{
    AoQuality, AoSettings, BlendMode, FrameParams, GraphicsSettings, Instance, LightingDebug, PostSettings, Projection,
    RenderBackend, RenderExperiments, Shading, SurfaceLight, Tonemap,
};
use blackbox_gfx_testkit::mesh::{MeshBuilder, prelit};
use glam::{Mat4, Vec3};

const SIZE: (u32, u32) = (240, 160);

fn frame(eye: Vec3, sun: Vec3) -> FrameParams {
    FrameParams {
        view: glam::camera::rh::view::look_at_mat4(eye, Vec3::ZERO, Vec3::Z),
        projection: Projection::PerspectiveInfiniteReverse { fov_y: 1.0, aspect: 1.5, near: 0.1 },
        camera_position: eye,
        light_dir: -sun,
        clear_color: [0.05; 3],
        fog: None,
        scattering: None,
        surface_light: None,
        camera_cut: false,
        shadow_tuning: None,
    }
}

fn capture(r: &mut Renderer, frame: &FrameParams, instances: &[Instance]) -> Vec<u8> {
    r.capture(SIZE.0, SIZE.1, frame, instances).unwrap()
}

#[test]
#[ignore = "needs a GPU"]
fn sloped_art_with_flat_authored_normals_does_not_shadow_itself_under_movement() {
    let _serial = test_support::serial();
    let Some(mut r) = test_support::headless(SIZE) else { return };
    let mut mesh = MeshBuilder::new();
    mesh.state(None, BlendMode::Opaque, Shading::Prelit);
    mesh.flat_quad(
        [
            Vec3::new(-15.0, -15.0, -9.0),
            Vec3::new(15.0, -15.0, -3.0),
            Vec3::new(15.0, 15.0, 9.0),
            Vec3::new(-15.0, 15.0, 3.0),
        ],
        prelit([0.8; 3], 1.0),
    );
    let mut mesh = mesh.finish();
    for v in &mut mesh.vertices {
        v.normal = Vec3::Z.to_array();
    }
    let mesh = mesh.upload(&mut r, "sloping road with authored normals");
    let instances = [Instance::new(mesh, Mat4::IDENTITY)];
    r.set_shadow_casters(Some(&instances));
    let horizontal = Vec3::new(-1.0, -0.4, 0.0).normalize();
    for resolution in [512, 2048] {
        for soft in [false, true] {
            for facing in [0.14, 0.149, 0.151, 0.16] {
                let sun = horizontal * (1.0_f32 - facing * facing).sqrt() + Vec3::Z * facing;
                for offset in [0.0, 0.03] {
                    let frame = frame(Vec3::new(3.0 + offset, -6.0 + offset, 5.0), sun);
                    r.set_render_experiments(Default::default());
                    let baseline = capture(&mut r, &frame, &instances);
                    r.set_render_experiments(RenderExperiments {
                        sun_shadows: true,
                        shadow_resolution: resolution,
                        soft_shadows: soft,
                        ..Default::default()
                    });
                    let image = capture(&mut r, &frame, &instances);
                    let pixels: Vec<_> = baseline
                        .as_chunks::<4>()
                        .0
                        .iter()
                        .zip(image.as_chunks::<4>().0)
                        .filter(|(a, _)| a[0] > 150)
                        .collect();
                    assert!(pixels.len() > 500);
                    let dark = pixels.iter().filter(|(a, b)| b[0].saturating_add(3) < a[0]).count();
                    assert!(
                        dark * 100 < pixels.len(),
                        "false shadow {dark}/{} at {resolution}, PCSS {soft}, angle {facing}",
                        pixels.len()
                    );
                }
            }
        }
    }
}

fn baked_scene(r: &mut Renderer) -> (FrameParams, Vec<Instance>, Vec<Instance>) {
    let mut mesh = MeshBuilder::new();
    mesh.state(None, BlendMode::Opaque, Shading::Prelit);
    for (left, right, brightness) in [(-15.0, 0.0, 0.25), (0.0, 15.0, 0.8)] {
        mesh.flat_quad(
            [
                Vec3::new(left, -15.0, 0.0),
                Vec3::new(right, -15.0, 0.0),
                Vec3::new(right, 15.0, 0.0),
                Vec3::new(left, 15.0, 0.0),
            ],
            prelit([brightness; 3], 1.0),
        );
    }
    let ground = mesh.finish().upload(r, "bright and baked shaded ground");
    let mut mesh = MeshBuilder::new();
    mesh.state(None, BlendMode::Opaque, Shading::Prelit);
    mesh.cuboid(Vec3::new(-2.0, -0.7, 0.0), Vec3::new(2.0, 0.7, 2.0), prelit([0.6; 3], 1.0));
    let caster = mesh.finish().upload(r, "occluder");
    (
        frame(Vec3::new(3.0, -6.0, 4.0), Vec3::new(-0.4, -0.3, 1.0).normalize()),
        vec![Instance::new(ground, Mat4::IDENTITY)],
        vec![Instance::new(caster, Mat4::IDENTITY)],
    )
}

#[test]
#[ignore = "needs a GPU"]
fn calibrated_ambient_preserves_unoccluded_art_and_avoids_double_darkening_baked_shade() {
    let _serial = test_support::serial();
    let Some(mut r) = test_support::headless(SIZE) else { return };
    let (mut frame, ground, casters) = baked_scene(&mut r);
    let baseline = capture(&mut r, &frame, &ground);
    let mut settings = RenderExperiments { world_ambient: true, ..Default::default() };
    r.set_render_experiments(settings);
    assert_eq!(baseline, capture(&mut r, &frame, &ground));
    r.set_shadow_casters(Some(&casters));
    settings.sun_shadows = true;
    settings.world_ambient = false;
    r.set_render_experiments(settings);
    let previous = capture(&mut r, &frame, &ground);
    settings.world_ambient = true;
    r.set_render_experiments(settings);
    let calibrated = capture(&mut r, &frame, &ground);
    let groups = [(40, 100), (150, 230)];
    let mut retention = Vec::new();
    for (lo, hi) in groups {
        let pixels: Vec<_> = baseline
            .as_chunks::<4>()
            .0
            .iter()
            .zip(previous.as_chunks::<4>().0)
            .zip(calibrated.as_chunks::<4>().0)
            .filter(|((base, old), _)| base[0] > lo && base[0] < hi && old[0].saturating_add(8) < base[0])
            .collect();
        assert!(pixels.len() > 50);
        assert!(pixels.iter().all(|((_, old), new)| new[0] > old[0]));
        retention.push(
            pixels.iter().map(|((base, _), new)| f32::from(new[0]) / f32::from(base[0])).sum::<f32>()
                / pixels.len() as f32,
        );
    }
    assert!(retention[0] > retention[1] + 0.03, "already shaded art retains more illumination: {retention:?}");
    frame.surface_light = Some(SurfaceLight { ambient: [0.15, 0.2, 0.3], diffuse: [0.7; 3] });
    let tinted = capture(&mut r, &frame, &ground);
    assert!(tinted.as_chunks::<4>().0.iter().filter(|p| p[2] > p[0].saturating_add(2)).count() > 50);
    r.set_shadow_casters(Some(&[]));
    assert_eq!(baseline, capture(&mut r, &frame, &ground), "ambient tint only changes additional shaded regions");
}

#[test]
#[ignore = "needs a GPU"]
fn surface_debug_views_are_spatial_and_restore_the_lit_scene() {
    let _serial = test_support::serial();
    let Some(mut r) = test_support::headless(SIZE) else { return };
    let (frame, mut ground, casters) = baked_scene(&mut r);
    ground.extend(casters.clone());
    r.set_shadow_casters(Some(&ground));
    r.apply_graphics(&GraphicsSettings {
        ao: AoSettings { quality: AoQuality::Medium, ..Default::default() },
        post: PostSettings { tonemap: Tonemap::Aces, bloom_intensity: 0.7, ..Default::default() },
        ..Default::default()
    });
    let mut settings = RenderExperiments { sun_shadows: true, world_ambient: true, ..Default::default() };
    r.set_render_experiments(settings);
    let original = capture(&mut r, &frame, &ground);
    for view in [LightingDebug::SunVisibility, LightingDebug::AmbientVisibility, LightingDebug::Cascades] {
        settings.lighting_debug = view;
        r.set_render_experiments(settings);
        let image = capture(&mut r, &frame, &ground);
        assert_ne!(image, original);
        assert_eq!(image, capture(&mut r, &frame, &ground));
        if view == LightingDebug::Cascades {
            assert!(image.as_chunks::<4>().0.iter().filter(|p| p[0].abs_diff(p[1]) > 20).count() > 100);
            continue;
        }
        assert!(image.as_chunks::<4>().0.iter().all(|p| p[0] == p[1] && p[1] == p[2]));
        assert!(
            image.as_chunks::<4>().0.iter().filter(|p| p[0] > 253).count() > 200,
            "debug white bypasses colour effects"
        );
        assert_eq!(r.graphics().post.tonemap, Tonemap::Aces, "debug does not change the selected effects");
    }
    settings.lighting_debug = LightingDebug::Off;
    r.set_render_experiments(settings);
    assert_eq!(original, capture(&mut r, &frame, &ground));
}
