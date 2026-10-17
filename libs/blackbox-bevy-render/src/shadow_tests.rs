//! Procedural GPU coverage of dynamic sun visibility. No install or authored game assets.

use bevy_light::{CascadeShadowConfig, DirectionalLight, DirectionalLightShadowMap};
use blackbox_gfx::{
    BlendMode, FrameParams, GraphicsApi, GraphicsSettings, Instance, InstanceKey, MeshHandle, PixelFormat,
    RenderBackend, RgbaImage, Shading, ShadowQuality, TextureDesc,
};
use blackbox_gfx_testkit::{
    Mask,
    camera::Camera,
    compare,
    mesh::{MeshBuilder, NEUTRAL},
};
use blackbox_gpu_passes::test_support::serial;
use glam::{Mat4, Vec3};

use crate::HeadlessBevy;

const SIZE: [u32; 2] = [320, 240];

fn renderer(api: GraphicsApi) -> HeadlessBevy {
    let _ = tracing_subscriber::fmt().with_env_filter("warn").with_test_writer().try_init();
    HeadlessBevy::new(SIZE, api, false).expect("a GPU supporting the requested API")
}

fn frame() -> FrameParams {
    let mut frame = Camera::new(Vec3::new(0.0, -12.0, 13.0), Vec3::ZERO).frame(4.0 / 3.0, [0.1; 3], None);
    frame.light_dir = Vec3::new(0.8, 0.0, -1.0).normalize();
    frame
}

fn plate(backend: &mut HeadlessBevy, blend: BlendMode, transparent: bool) -> MeshHandle {
    let texture = transparent.then(|| {
        backend.create_texture(&TextureDesc {
            label: "invisible cutout",
            width: 1,
            height: 1,
            format: PixelFormat::Rgba8,
            mips: vec![&[255, 255, 255, 0]],
        })
    });
    let mut mesh = MeshBuilder::new();
    mesh.state(texture, blend, Shading::Prelit);
    mesh.flat_quad(
        [Vec3::new(-1.5, -1.5, 0.0), Vec3::new(1.5, -1.5, 0.0), Vec3::new(1.5, 1.5, 0.0), Vec3::new(-1.5, 1.5, 0.0)],
        NEUTRAL,
    );
    mesh.finish().upload(backend, "caster")
}

fn ground(backend: &mut HeadlessBevy, shading: Shading) -> Instance {
    let mut mesh = MeshBuilder::new();
    mesh.state(None, BlendMode::Opaque, shading);
    mesh.flat_quad(
        [
            Vec3::new(-12.0, -12.0, 0.0),
            Vec3::new(12.0, -12.0, 0.0),
            Vec3::new(12.0, 12.0, 0.0),
            Vec3::new(-12.0, 12.0, 0.0),
        ],
        NEUTRAL,
    );
    Instance::keyed(mesh.finish().upload(backend, "receiver"), Mat4::IDENTITY, InstanceKey::new(1, 0))
}

fn caster(mesh: MeshHandle, x: f32) -> Instance {
    Instance::keyed(mesh, Mat4::from_translation(Vec3::new(x, 0.0, 3.0)), InstanceKey::new(1, 1))
}

fn capture(backend: &mut HeadlessBevy, frame: &FrameParams, instances: &[Instance]) -> RgbaImage {
    capture_filtered(backend, frame, instances, None)
}

fn capture_filtered(
    backend: &mut HeadlessBevy,
    frame: &FrameParams,
    instances: &[Instance],
    filter: Option<bevy_light::ShadowFilteringMethod>,
) -> RgbaImage {
    let id = backend.request_capture(SIZE, frame, instances).unwrap();
    for _ in 0..120 {
        backend.render(frame, instances).unwrap();
        if let Some(filter) = filter {
            let world = backend.app_mut().world_mut();
            let mut cameras =
                world.query_filtered::<bevy_ecs::entity::Entity, bevy_ecs::query::With<bevy_camera::Camera>>();
            let cameras: Vec<_> = cameras.iter(world).collect();
            for camera in cameras {
                world.entity_mut(camera).insert(filter);
            }
        }
        if let Some(image) = backend.poll_capture(id) {
            return image.unwrap();
        }
    }
    panic!("capture timed out");
}

fn quality(backend: &mut HeadlessBevy, shadows: ShadowQuality) {
    let requested = GraphicsSettings { shadows, ..*backend.graphics() };
    assert!(backend.apply_graphics(&requested).is_exact());
}

fn sun(backend: &mut HeadlessBevy) -> bevy_ecs::entity::Entity {
    backend
        .app_mut()
        .world_mut()
        .query_filtered::<bevy_ecs::entity::Entity, bevy_ecs::query::With<DirectionalLight>>()
        .single(backend.app().world())
        .unwrap()
}

fn assert_spatial_camera(backend: &HeadlessBevy) {
    let world = backend.app().world();
    let camera = world.resource::<crate::apply::WorldState>().capture.as_ref().unwrap().camera;
    assert_eq!(
        world.get::<bevy_light::ShadowFilteringMethod>(camera),
        Some(&bevy_light::ShadowFilteringMethod::Gaussian)
    );
    assert!(world.get::<bevy_render::camera::TemporalJitter>(camera).is_none());
    assert!(world.get::<bevy_anti_alias::taa::TemporalAntiAliasing>(camera).is_none());
    assert!(world.get::<bevy_core_pipeline::prepass::DepthPrepass>(camera).is_none());
    assert!(world.get::<bevy_core_pipeline::prepass::MotionVectorPrepass>(camera).is_none());
}

fn dump(name: &str, image: &RgbaImage) {
    let Ok(dir) = std::env::var("BLACKBOX_DUMP_DIR") else { return };
    let file = std::fs::File::create(std::path::Path::new(&dir).join(format!("{name}.png"))).unwrap();
    let mut encoder = png::Encoder::new(std::io::BufWriter::new(file), image.width, image.height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder.write_header().unwrap().write_image_data(&image.rgba).unwrap();
}

fn dynamic(api: GraphicsApi) {
    let _gpu = serial();
    let mut backend = renderer(api);
    let frame = frame();
    let ground = ground(&mut backend, Shading::Prelit);
    let mesh = plate(&mut backend, BlendMode::Opaque, false);
    let scene = [ground, caster(mesh, 0.0)];
    let off = capture(&mut backend, &frame, &scene);
    quality(&mut backend, ShadowQuality::Medium);
    let on = capture(&mut backend, &frame, &scene);
    dump("shadow-off", &off);
    dump("shadow-soft", &on);
    let diff = compare(&off, &on, &Mask::all()).unwrap();
    eprintln!("{api} shadows off/on: {diff}");
    assert!(diff.mean > 0.5 && diff.max > 50, "a caster darkens the receiver: {diff}");

    let moved = capture(&mut backend, &frame, &[ground, caster(mesh, -4.0)]);
    assert!(compare(&on, &moved, &Mask::all()).unwrap().mean > 1.0, "moving a keyed caster moves its shadow");
    let mut changed = frame;
    changed.light_dir.x = -changed.light_dir.x;
    let other_sun = capture(&mut backend, &changed, &scene);
    assert!(compare(&on, &other_sun, &Mask::all()).unwrap().mean > 0.5, "the shadow follows the frame sun");

    // Removing a caster must stop its shadow immediately, even during the entity pool's grace period.
    let empty = capture(&mut backend, &frame, &[ground]);
    quality(&mut backend, ShadowQuality::Off);
    let empty_off = capture(&mut backend, &frame, &[ground]);
    assert!(compare(&empty, &empty_off, &Mask::all()).unwrap().mean < 0.2);
    assert!(backend.app().world().resource::<crate::apply::WorldState>().sun.is_none());
    let restored = capture(&mut backend, &frame, &scene);
    assert!(compare(&off, &restored, &Mask::all()).unwrap().mean < 0.2, "off restores the original shading");

    for q in [ShadowQuality::Low, ShadowQuality::High, ShadowQuality::Medium] {
        quality(&mut backend, q);
        capture(&mut backend, &frame, &scene);
        let sun = sun(&mut backend);
        let config = backend.app().world().get::<CascadeShadowConfig>(sun).unwrap();
        let (size, cascades, distance) = q.sampling();
        assert_eq!(config.bounds.len(), cascades);
        assert!((config.bounds.last().copied().unwrap() - distance).abs() < 0.001);
        assert_eq!(backend.app().world().resource::<DirectionalLightShadowMap>().size, size);
    }
}

#[test]
#[ignore = "needs a Vulkan GPU"]
fn dynamic_soft_shadows_vulkan() {
    dynamic(GraphicsApi::Vulkan);
}

#[cfg(target_os = "windows")]
#[test]
#[ignore = "needs a DX12 GPU"]
fn dynamic_soft_shadows_dx12() {
    dynamic(GraphicsApi::Dx12);
}

#[test]
#[ignore = "needs a GPU"]
fn cutout_alpha_is_respected_in_shadow_depth() {
    let _gpu = serial();
    let mut backend = renderer(GraphicsApi::Vulkan);
    let frame = frame();
    let ground = ground(&mut backend, Shading::Prelit);
    quality(&mut backend, ShadowQuality::Medium);
    let empty = capture(&mut backend, &frame, &[ground]);
    let invisible = plate(&mut backend, BlendMode::AlphaTest, true);
    let cutout = capture(&mut backend, &frame, &[ground, caster(invisible, 0.0)]);
    assert!(compare(&empty, &cutout, &Mask::all()).unwrap().mean < 0.2, "transparent cutout pixels cast no shadow");
    let opaque_cutout = plate(&mut backend, BlendMode::AlphaTest, false);
    let solid = capture(&mut backend, &frame, &[ground, caster(opaque_cutout, 0.0)]);
    assert!(compare(&empty, &solid, &Mask::all()).unwrap().mean > 1.0, "opaque cutout pixels still cast");
}

#[test]
#[ignore = "needs a GPU"]
fn spatial_filter_is_stable_without_taa_and_works_with_smaa_and_upscaling() {
    let _gpu = serial();
    let mut backend = renderer(GraphicsApi::Vulkan);
    let frame = frame();
    let ground = ground(&mut backend, Shading::Lit);
    let mesh = plate(&mut backend, BlendMode::Opaque, false);
    let scene = [ground, caster(mesh, 0.0)];
    quality(&mut backend, ShadowQuality::Medium);
    let soft = capture(&mut backend, &frame, &scene);
    let repeated = capture(&mut backend, &frame, &scene);
    assert_spatial_camera(&backend);
    assert_eq!(soft.rgba, repeated.rgba, "static spatial shadows have no temporal noise");
    let hard = capture_filtered(&mut backend, &frame, &scene, Some(bevy_light::ShadowFilteringMethod::Hardware2x2));
    let diff = compare(&soft, &hard, &Mask::all()).unwrap();
    eprintln!("Gaussian/hardware filter: {diff}");
    assert!(diff.mean > 0.005 && diff.max > 5, "spatial PCF softens actual edges: {diff}");
    dump("shadow-hard", &hard);
    dump("shadow-spatial", &soft);
    let asked = GraphicsSettings {
        post: blackbox_gfx::PostSettings { antialiasing: blackbox_gfx::Antialiasing::Smaa, ..Default::default() },
        upscaler: blackbox_gfx::Upscaler::Fsr1,
        render_scale: 0.67,
        ..*backend.graphics()
    };
    assert!(backend.apply_graphics(&asked).is_exact());
    let smaa = capture(&mut backend, &frame, &scene);
    assert_spatial_camera(&backend);
    dump("shadow-smaa-upscaled", &smaa);
    assert!(compare(&smaa, &soft, &Mask::all()).unwrap().mean < 5.0, "SMAA and FSR 1 preserve the shadow");
}

#[test]
#[ignore = "needs a GPU"]
fn a_caster_outside_the_camera_frustum_still_casts() {
    let _gpu = serial();
    let mut backend = renderer(GraphicsApi::Vulkan);
    let mut frame = frame();
    frame.light_dir = Vec3::new(0.0, 1.0, -0.25).normalize();
    let ground = ground(&mut backend, Shading::Prelit);
    let mesh = plate(&mut backend, BlendMode::Opaque, false);
    let caster = Instance::keyed(mesh, Mat4::from_translation(Vec3::new(0.0, -25.0, 7.0)), InstanceKey::new(1, 1));
    let scene = [ground, caster];
    let off = capture(&mut backend, &frame, &scene);
    quality(&mut backend, ShadowQuality::High);
    let on = capture(&mut backend, &frame, &scene);
    let diff = compare(&off, &on, &Mask::all()).unwrap();
    eprintln!("off-camera caster: {diff}");
    assert!(diff.mean > 0.3 && diff.max > 50, "an offscreen object can shadow an onscreen receiver: {diff}");
}

#[test]
#[ignore = "needs a GPU"]
fn glossy_sun_is_shadowed_while_fill_and_reflections_remain() {
    let _gpu = serial();
    let mut backend = renderer(GraphicsApi::Vulkan);
    let frame = frame();
    let material = backend.create_glossy_material(&blackbox_gfx::GlossyMaterial::default());
    let receiver = ground(&mut backend, Shading::Glossy(material));
    let mesh = plate(&mut backend, BlendMode::Opaque, false);
    let scene = [receiver, caster(mesh, 0.0)];
    let mut rig = blackbox_gfx::LightingRig {
        lights: [blackbox_gfx::DirectionalLight::new(-frame.light_dir, Vec3::splat(0.25)); 3],
        ambient: Vec3::splat(0.05),
        ..Default::default()
    };
    backend.set_lighting_rig(&rig);
    let off = capture(&mut backend, &frame, &scene);
    quality(&mut backend, ShadowQuality::Medium);
    let on = capture(&mut backend, &frame, &scene);
    assert!(compare(&off, &on, &Mask::all()).unwrap().mean > 0.3, "the glossy key light responds to occlusion");

    rig.lights[0].color = Vec3::ZERO;
    // Disable the sun glint to isolate fill, back, ambient and the environment reflection.
    backend.destroy_glossy_material(material);
    let material =
        backend.create_glossy_material(&blackbox_gfx::GlossyMaterial { specular_min: [0.0; 3], ..Default::default() });
    let scene = [ground(&mut backend, Shading::Glossy(material)), caster(mesh, 0.0)];
    backend.set_lighting_rig(&rig);
    let fill_on = capture(&mut backend, &frame, &scene);
    quality(&mut backend, ShadowQuality::Off);
    let fill_off = capture(&mut backend, &frame, &scene);
    assert!(compare(&fill_on, &fill_off, &Mask::all()).unwrap().mean < 0.2, "indirect light and reflections survive");
}

#[test]
#[ignore = "needs a GPU"]
fn last_cascade_fades_instead_of_cutting_off() {
    let _gpu = serial();
    let mut backend = renderer(GraphicsApi::Vulkan);
    let frame = frame();
    let ground = ground(&mut backend, Shading::Prelit);
    let mesh = plate(&mut backend, BlendMode::Opaque, false);
    let scene = [ground, caster(mesh, 0.0)];
    let off = capture(&mut backend, &frame, &scene);
    quality(&mut backend, ShadowQuality::Medium);
    let full = capture(&mut backend, &frame, &scene);
    let light = sun(&mut backend);
    let mut strengths = Vec::new();
    for distance in [24.0, 22.0, 20.0, 18.0, 16.0] {
        let config = bevy_light::CascadeShadowConfigBuilder {
            num_cascades: 1,
            maximum_distance: distance,
            overlap_proportion: 0.0,
            ..Default::default()
        }
        .build();
        backend.app_mut().world_mut().entity_mut(light).insert(config);
        let image = capture(&mut backend, &frame, &scene);
        dump(&format!("shadow-fade-{distance}"), &image);
        strengths.push(compare(&off, &image, &Mask::all()).unwrap().mean);
    }
    let full = compare(&off, &full, &Mask::all()).unwrap().mean;
    eprintln!("last-cascade fade: full {full}, steps {strengths:?}");
    assert!(strengths[2] > 0.1 && strengths[2] < full * 0.9, "partially faded shadow before the boundary");
    assert!(strengths.windows(2).all(|s| s[0] > s[1]), "shadow weakens continuously approaching the boundary");
    assert!(strengths[4] < 0.2, "beyond the range there is no abrupt dark edge");
}

#[test]
#[ignore = "needs a GPU"]
fn aces_preserves_authored_midtones_clear_and_fog() {
    let _gpu = serial();
    let mut backend = renderer(GraphicsApi::Vulkan);
    let mut frame = frame();
    frame.clear_color = [0.4; 3];
    frame.fog = Some(blackbox_gfx::Fog { start: 0.0, end: 40.0 });
    let mut mesh = MeshBuilder::new();
    mesh.state(None, BlendMode::Opaque, Shading::Prelit);
    // Half of reference vertex brightness: a mid-grey patch against mid-grey fog/clear.
    mesh.flat_quad(
        [
            Vec3::new(-12.0, -12.0, 0.0),
            Vec3::new(12.0, -12.0, 0.0),
            Vec3::new(12.0, 12.0, 0.0),
            Vec3::new(-12.0, 12.0, 0.0),
        ],
        [64, 64, 64, 255],
    );
    let scene =
        [Instance::keyed(mesh.finish().upload(&mut backend, "mid-grey"), Mat4::IDENTITY, InstanceKey::new(1, 0))];
    let off = capture(&mut backend, &frame, &scene);
    let asked = GraphicsSettings {
        post: blackbox_gfx::PostSettings { tonemap: blackbox_gfx::Tonemap::Aces, ..Default::default() },
        ..*backend.graphics()
    };
    assert!(backend.apply_graphics(&asked).is_exact());
    let aces = capture(&mut backend, &frame, &scene);
    dump("aces-off", &off);
    dump("aces-calibrated", &aces);
    let diff = compare(&off, &aces, &Mask::all()).unwrap();
    eprintln!("ACES authored midtones: {diff}");
    assert!(diff.max < 12 && diff.mean < 8.0, "ACES must not crush SDR-authored midtones, fog or clear: {diff}");
    let darker = GraphicsSettings { post: blackbox_gfx::PostSettings { exposure: 0.5, ..asked.post }, ..asked };
    backend.apply_graphics(&darker);
    let exposed = capture(&mut backend, &frame, &scene);
    assert!(
        compare(&aces, &exposed, &Mask::all()).unwrap().mean > 15.0,
        "ACES exposure controls must affect the output"
    );
    backend.apply_graphics(&asked);
    quality(&mut backend, ShadowQuality::Medium);
    let mesh = plate(&mut backend, BlendMode::Opaque, false);
    let shaded = capture(&mut backend, &frame, &[scene[0], caster(mesh, 0.0)]);
    assert!(compare(&aces, &shaded, &Mask::all()).unwrap().mean > 0.3, "ACES still retains dynamic shadow contrast");
}

#[test]
#[ignore = "needs a GPU"]
fn aces_retains_above_white_prelit_detail() {
    let _gpu = serial();
    let mut backend = renderer(GraphicsApi::Vulkan);
    let mut mesh = MeshBuilder::new();
    mesh.state(None, BlendMode::Opaque, Shading::Prelit);
    for (x, brightness) in [(-12.0, 150), (0.0, 200)] {
        mesh.flat_quad(
            [
                Vec3::new(x, -12.0, 0.0),
                Vec3::new(x + 12.0, -12.0, 0.0),
                Vec3::new(x + 12.0, 12.0, 0.0),
                Vec3::new(x, 12.0, 0.0),
            ],
            [brightness, brightness, brightness, 255],
        );
    }
    let frame = frame();
    let scene =
        [Instance::keyed(mesh.finish().upload(&mut backend, "above-white"), Mat4::IDENTITY, InstanceKey::new(1, 0))];
    let off = capture(&mut backend, &frame, &scene);
    let settings = GraphicsSettings {
        post: blackbox_gfx::PostSettings { tonemap: blackbox_gfx::Tonemap::Aces, ..Default::default() },
        ..*backend.graphics()
    };
    backend.apply_graphics(&settings);
    let aces = capture(&mut backend, &frame, &scene);
    let pixel = |image: &RgbaImage, x: usize| image.rgba[(120 * SIZE[0] as usize + x) * 4];
    assert_eq!(pixel(&off, 100), 255);
    assert_eq!(pixel(&off, 220), 255, "the legacy pipeline clamps both patches");
    assert!(pixel(&aces, 220) > pixel(&aces, 100) + 3, "ACES preserves their distinct above-white brightness");
}
