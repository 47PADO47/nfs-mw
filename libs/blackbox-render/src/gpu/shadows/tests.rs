//! Synthetic image regressions: movement, cutouts, spatial repeatability, and independent toggles.
use crate::{Renderer, gpu::test_support};
use blackbox_gfx::{
    Antialiasing, BlendMode, FrameParams, GraphicsSettings, Instance, PostSettings, Projection, RenderBackend,
    RenderExperiments, Shading,
};
use blackbox_gfx_testkit::mesh::{MeshBuilder, prelit};
use glam::{Mat4, Vec3};

#[test]
#[ignore = "needs a GPU"]
fn instance_caster_and_receiver_flags_suppress_native_shadows_independently() {
    use blackbox_gfx::ShadowFlags;
    let _gpu = test_support::serial();
    let mut r = test_support::headless((240, 160)).expect("GPU required");
    let (frame, ground, mut casters) = scene(&mut r);
    casters[0].key = blackbox_gfx::InstanceKey::new(1, 0);
    let options =
        RenderExperiments { sun_shadows: true, soft_shadows: false, cached_shadows: true, ..Default::default() };
    r.set_render_experiments(options);
    r.set_shadow_casters(Some(&casters));
    let shadowed = r.capture(240, 160, &frame, &ground).unwrap();
    for _ in 0..3 {
        assert_eq!(shadowed, r.capture(240, 160, &frame, &ground).unwrap());
    }
    r.set_render_experiments(RenderExperiments { sun_shadows: false, ..options });
    let baseline = r.capture(240, 160, &frame, &ground).unwrap();
    assert!(dark_pixels(&baseline, &shadowed) > 50, "default instances retain casting and receiving");

    r.set_render_experiments(options);
    casters[0].shadow.cast = false;
    r.set_shadow_casters(Some(&casters));
    assert_eq!(baseline, r.capture(240, 160, &frame, &ground).unwrap(), "non-casters clear earlier shadow coverage");

    casters[0].shadow.cast = true;
    r.set_shadow_casters(Some(&casters));
    let no_receive: Vec<_> =
        ground.iter().map(|instance| instance.with_shadow(ShadowFlags { cast: false, receive: false })).collect();
    assert_eq!(baseline, r.capture(240, 160, &frame, &no_receive).unwrap(), "non-receivers ignore active caster maps");
    let receive_only: Vec<_> =
        ground.iter().map(|instance| instance.with_shadow(ShadowFlags { cast: false, receive: true })).collect();
    assert_eq!(shadowed, r.capture(240, 160, &frame, &receive_only).unwrap(), "menu floors receive without casting");
}

pub(super) fn scene(renderer: &mut Renderer) -> (FrameParams, Vec<Instance>, Vec<Instance>) {
    let mut plane = MeshBuilder::new();
    plane.state(None, BlendMode::Opaque, Shading::Prelit);
    plane.flat_quad(
        [
            Vec3::new(-15.0, -15.0, 0.0),
            Vec3::new(15.0, -15.0, 0.0),
            Vec3::new(15.0, 15.0, 0.0),
            Vec3::new(-15.0, 15.0, 0.0),
        ],
        prelit([0.7; 3], 1.0),
    );
    let ground = plane.finish().upload(renderer, "shadow ground");
    let mut box_mesh = MeshBuilder::new();
    box_mesh.state(None, BlendMode::Opaque, Shading::Prelit);
    box_mesh.cuboid(Vec3::new(-0.7, -0.7, 0.0), Vec3::new(0.7, 0.7, 2.0), prelit([0.6; 3], 1.0));
    let caster = box_mesh.finish().upload(renderer, "moving caster");
    let eye = Vec3::new(3.0, -6.0, 4.0);
    let frame = FrameParams {
        view: glam::camera::rh::view::look_at_mat4(eye, Vec3::ZERO, Vec3::Z),
        projection: Projection::PerspectiveInfiniteReverse { fov_y: 1.0, aspect: 1.5, near: 0.1 },
        camera_position: eye,
        light_dir: Vec3::new(0.4, 0.3, -1.0).normalize(),
        clear_color: [0.2; 3],
        fog: None,
        scattering: None,
        surface_light: None,
        camera_cut: false,
        shadow_tuning: None,
    };
    (frame, vec![Instance::new(ground, Mat4::IDENTITY)], vec![Instance::new(caster, Mat4::IDENTITY)])
}

pub(super) fn dark_pixels(a: &[u8], b: &[u8]) -> usize {
    a.as_chunks::<4>().0.iter().zip(b.as_chunks::<4>().0).filter(|(a, b)| b[0].saturating_add(8) < a[0]).count()
}

#[test]
#[ignore = "needs a GPU"]
fn offcamera_casters_move_and_off_roundtrips_without_history() {
    let _gpu = test_support::serial();
    let Some(mut r) = test_support::headless((240, 160)) else { return };
    let (frame, ground, mut casters) = scene(&mut r);
    let baseline = r.capture(240, 160, &frame, &ground).unwrap();
    r.set_render_experiments(RenderExperiments { sun_shadows: true, shadow_resolution: 1024, ..Default::default() });
    r.set_shadow_casters(Some(&casters));
    let shadow = r.capture(240, 160, &frame, &ground).unwrap();
    assert!(dark_pixels(&baseline, &shadow) > 100, "caster omitted from colour pass must shadow ground");
    assert_eq!(shadow, r.capture(240, 160, &frame, &ground).unwrap(), "spatial samples are repeatable");
    casters[0].transform = Mat4::from_translation(Vec3::new(2.0, 0.0, 0.0));
    r.set_shadow_casters(Some(&casters));
    assert_ne!(shadow, r.capture(240, 160, &frame, &ground).unwrap(), "moving caster updates immediately");
    r.set_shadow_casters(Some(&[]));
    assert_eq!(baseline, r.capture(240, 160, &frame, &ground).unwrap(), "removal leaves no stale shadow");
    r.set_render_experiments(Default::default());
    assert_eq!(baseline, r.capture(240, 160, &frame, &ground).unwrap());
}

#[test]
#[ignore = "needs a GPU"]
fn spatial_smaa_contact_and_far_modes_survive_switches_and_resize() {
    let _gpu = test_support::serial();
    let Some(mut r) = test_support::headless((240, 160)) else { return };
    let (frame, mut ground, casters) = scene(&mut r);
    ground.extend(casters);
    r.apply_graphics(&GraphicsSettings {
        post: PostSettings { antialiasing: Antialiasing::Smaa, ..Default::default() },
        ..Default::default()
    });
    assert_eq!(r.graphics().post.antialiasing, Antialiasing::Smaa);
    for resolution in [512, 1024, 2048, 4096] {
        r.set_render_experiments(RenderExperiments {
            sun_shadows: true,
            far_shadows: true,
            contact_shadows: true,
            clearcoat: true,
            shadow_resolution: resolution,
            ..Default::default()
        });
        let image = r.capture(240, 160, &frame, &ground).unwrap();
        assert_eq!(image, r.capture(240, 160, &frame, &ground).unwrap());
        assert_eq!(r.experiment_stats().cascades, 4);
        assert_eq!(r.experiment_stats().map_mib, resolution * resolution * 16 / (1024 * 1024));
        assert_eq!(r.capture(111, 73, &frame, &ground).unwrap().len(), 111 * 73 * 4);
    }
    r.set_render_experiments(Default::default());
    r.apply_graphics(&GraphicsSettings::default());
    r.capture(240, 160, &frame, &ground).unwrap();
    assert!(r.smaa.is_none());
    assert_eq!(r.experiment_stats().map_mib, 0);
    r.set_post_effects(PostSettings { antialiasing: Antialiasing::Smaa, ..Default::default() });
    r.capture(240, 160, &frame, &ground).unwrap();
    assert!(r.smaa.is_some(), "the inherent post API also runs spatial SMAA");
}

#[test]
#[ignore = "needs a GPU"]
fn alpha_cutouts_and_vertex_opacity_control_shadow_coverage() {
    let _gpu = test_support::serial();
    let Some(mut r) = test_support::headless((240, 160)) else { return };
    let (frame, ground, _) = scene(&mut r);
    let baseline = r.capture(240, 160, &frame, &ground).unwrap();
    r.set_render_experiments(RenderExperiments { sun_shadows: true, ..Default::default() });
    let mut coverage = Vec::new();
    for (texture_alpha, vertex_alpha) in [(0u8, 1.0), (255, 0.0), (255, 1.0)] {
        let rgba = [255, 255, 255, texture_alpha];
        let texture = r.create_texture(&blackbox_gfx::TextureDesc {
            label: "cutout alpha",
            width: 1,
            height: 1,
            format: blackbox_gfx::PixelFormat::Rgba8,
            mips: vec![&rgba],
        });
        let mut mesh = MeshBuilder::new();
        mesh.state(Some(texture), BlendMode::AlphaTest, Shading::Prelit);
        mesh.cuboid(Vec3::new(-0.7, -0.7, 0.0), Vec3::new(0.7, 0.7, 2.0), prelit([0.6; 3], vertex_alpha));
        let caster = mesh.finish().upload(&mut r, "cutout caster");
        r.set_shadow_casters(Some(&[Instance::new(caster, Mat4::IDENTITY)]));
        coverage.push(dark_pixels(&baseline, &r.capture(240, 160, &frame, &ground).unwrap()));
    }
    assert_eq!(coverage[..2], [0, 0], "transparent texels and vertices must not cast");
    assert!(coverage[2] > 100, "solid alpha-tested geometry must cast");
}

#[test]
#[ignore = "needs a GPU"]
fn glossy_receivers_follow_sun_shadow_and_survive_map_rebinding() {
    let _gpu = test_support::serial();
    let Some(mut r) = test_support::headless((240, 160)) else { return };
    let (mut frame, _, casters) = scene(&mut r);
    frame.light_dir = Vec3::NEG_Z;
    let material = r.create_glossy_material(&blackbox_gfx::GlossyMaterial {
        specular_min: [0.0; 3],
        envmap_min: 0.0,
        ..Default::default()
    });
    let mut rig = blackbox_gfx::LightingRig::default();
    rig.lights[0] = blackbox_gfx::DirectionalLight::new(Vec3::Z, Vec3::splat(0.7));
    rig.lights[1].color = Vec3::splat(0.1);
    rig.lights[2].color = Vec3::ZERO;
    r.set_lighting_rig(&rig);
    let mut plane = MeshBuilder::new();
    plane.state(None, BlendMode::Opaque, Shading::Glossy(material));
    plane.flat_quad(
        [
            Vec3::new(-15.0, -15.0, 0.0),
            Vec3::new(15.0, -15.0, 0.0),
            Vec3::new(15.0, 15.0, 0.0),
            Vec3::new(-15.0, 15.0, 0.0),
        ],
        [255; 4],
    );
    let mesh = plane.finish().upload(&mut r, "glossy receiver");
    let receiver = [Instance::new(mesh, Mat4::IDENTITY)];
    let baseline = r.capture(240, 160, &frame, &receiver).unwrap();
    r.set_shadow_casters(Some(&casters));
    for resolution in [512, 2048, 1024] {
        r.set_render_experiments(RenderExperiments {
            sun_shadows: true,
            shadow_resolution: resolution,
            ..Default::default()
        });
        let shadow = r.capture(240, 160, &frame, &receiver).unwrap();
        assert!(dark_pixels(&baseline, &shadow) > 50, "sun shadows reach glossy diffuse lighting");
        assert_eq!(shadow, r.capture(240, 160, &frame, &receiver).unwrap());
    }
    r.set_render_experiments(RenderExperiments { clearcoat: true, ..Default::default() });
    assert_ne!(baseline, r.capture(240, 160, &frame, &receiver).unwrap(), "coat is independent of shadows");
    r.set_render_experiments(Default::default());
    assert_eq!(baseline, r.capture(240, 160, &frame, &receiver).unwrap());
}

#[test]
#[ignore = "needs a GPU"]
fn contact_detail_changes_only_bounded_spatial_visibility() {
    let _gpu = test_support::serial();
    let Some(mut r) = test_support::headless((240, 160)) else { return };
    let (frame, mut ground, casters) = scene(&mut r);
    ground.extend(casters);
    let off = r.capture(240, 160, &frame, &ground).unwrap();
    r.set_render_experiments(RenderExperiments { contact_shadows: true, ..Default::default() });
    let on = r.capture(240, 160, &frame, &ground).unwrap();
    assert!(dark_pixels(&off, &on) > 3, "contact rays must change visible pixels");
    assert_eq!(on, r.capture(240, 160, &frame, &ground).unwrap());
    for (a, b) in off.as_chunks::<4>().0.iter().zip(on.as_chunks::<4>().0) {
        assert!(b[0] <= a[0] + 1);
    }
}

#[test]
#[ignore = "needs a GPU"]
fn contact_rejects_unoccluded_walls_and_preserves_fog_and_ambient() {
    let _gpu = test_support::serial();
    let Some(mut r) = test_support::headless((240, 160)) else { return };
    let (mut frame, ground, casters) = scene(&mut r);
    for rotation in [Mat4::IDENTITY, Mat4::from_rotation_x(std::f32::consts::FRAC_PI_2)] {
        let wall = [Instance::new(ground[0].mesh, rotation)];
        r.set_render_experiments(Default::default());
        let off = r.capture(240, 160, &frame, &wall).unwrap();
        r.set_render_experiments(RenderExperiments { contact_shadows: true, ..Default::default() });
        assert_eq!(off, r.capture(240, 160, &frame, &wall).unwrap(), "an isolated plane must not shadow itself");
    }
    let mut visible = ground;
    visible.extend(casters);
    frame.fog = Some(blackbox_gfx::Fog { start: 0.0, end: 0.01 });
    r.set_render_experiments(Default::default());
    let fog = r.capture(240, 160, &frame, &visible).unwrap();
    r.set_render_experiments(RenderExperiments { contact_shadows: true, ..Default::default() });
    assert_eq!(fog, r.capture(240, 160, &frame, &visible).unwrap(), "contact visibility cannot darken fog");
    frame.fog = None;
    frame.surface_light = Some(blackbox_gfx::SurfaceLight {
        shadow_strength: 1.0,
        specular: [0.0; 3],
        specular_power: 1.0,
        reflection_scale: 0.0,
        reflective_specular_scale: 1.0,
        ambient_alpha: 1.0,
        ambient: [0.6; 3],
        diffuse: [0.0; 3],
    });
    let mut ambient = MeshBuilder::new();
    ambient.state(None, BlendMode::Opaque, Shading::Lit);
    ambient.flat_quad(
        [
            Vec3::new(-15.0, -15.0, 0.0),
            Vec3::new(15.0, -15.0, 0.0),
            Vec3::new(15.0, 15.0, 0.0),
            Vec3::new(-15.0, 15.0, 0.0),
        ],
        [200; 4],
    );
    ambient.cuboid(Vec3::new(-1.0, -1.0, 0.0), Vec3::new(1.0, 1.0, 2.0), [200; 4]);
    let mesh = ambient.finish().upload(&mut r, "ambient-only receiver");
    let visible = [Instance::new(mesh, Mat4::IDENTITY)];
    frame.surface_light = Some(blackbox_gfx::SurfaceLight {
        shadow_strength: 1.0,
        specular: [0.0; 3],
        specular_power: 1.0,
        reflection_scale: 0.0,
        reflective_specular_scale: 1.0,
        ambient_alpha: 1.0,
        ambient: [0.2; 3],
        diffuse: [0.8; 3],
    });
    r.set_render_experiments(Default::default());
    let direct = r.capture(240, 160, &frame, &visible).unwrap();
    r.set_render_experiments(RenderExperiments { contact_shadows: true, ..Default::default() });
    assert!(
        dark_pixels(&direct, &r.capture(240, 160, &frame, &visible).unwrap()) > 3,
        "fixture actually receives contact visibility"
    );
    frame.surface_light = Some(blackbox_gfx::SurfaceLight {
        shadow_strength: 1.0,
        specular: [0.0; 3],
        specular_power: 1.0,
        reflection_scale: 0.0,
        reflective_specular_scale: 1.0,
        ambient_alpha: 1.0,
        ambient: [0.6; 3],
        diffuse: [0.0; 3],
    });
    r.set_render_experiments(Default::default());
    let off = r.capture(240, 160, &frame, &visible).unwrap();
    r.set_render_experiments(RenderExperiments { contact_shadows: true, ..Default::default() });
    assert_eq!(off, r.capture(240, 160, &frame, &visible).unwrap(), "ambient remains unattenuated");
}

#[test]
#[ignore = "needs a GPU"]
fn pcss_softens_separated_casters_and_gpu_timings_are_read_asynchronously() {
    let _gpu = test_support::serial();
    let Some(mut r) = test_support::headless((240, 160)) else { return };
    let (mut frame, ground, mut casters) = scene(&mut r);
    frame.light_dir = Vec3::NEG_Z;
    casters[0].transform = Mat4::from_translation(Vec3::Z * 5.0);
    r.set_shadow_casters(Some(&casters));
    let mut options = RenderExperiments { sun_shadows: true, soft_shadows: false, ..Default::default() };
    r.set_render_experiments(options);
    let hard = r.capture(240, 160, &frame, &ground).unwrap();
    options.soft_shadows = true;
    options.sun_angle = 2.0;
    r.set_render_experiments(options);
    let soft = r.capture(240, 160, &frame, &ground).unwrap();
    assert_ne!(hard, soft, "blocker separation must change the penumbra");
    assert_eq!(soft, r.capture(240, 160, &frame, &ground).unwrap(), "PCSS has no temporal seed");
    if r.timing.is_some() {
        // Screenshot passes deliberately do not publish live-frame timings.
        // Exercise ordinary rendering and retire asynchronous readbacks explicitly.
        for _ in 0..3 {
            r.render(&frame, &ground).unwrap();
            r.device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
        }
        let stats = r.experiment_stats();
        assert!(stats.gpu_frame_ms.is_some_and(|ms| ms.is_finite() && ms > 0.0));
        assert!(stats.gpu_shadow_ms.is_some_and(|ms| ms.is_finite() && ms > 0.0));
    }
}
