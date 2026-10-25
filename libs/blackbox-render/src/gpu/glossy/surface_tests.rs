//! Custom surface regression coverage.
#[test]
fn the_environment_filter_shader_validates() {
    let source = super::prefilter::SHADER;
    let module = naga::front::wgsl::parse_str(source).unwrap_or_else(|e| panic!("{}", e.emit_to_string(source)));
    naga::valid::Validator::new(naga::valid::ValidationFlags::all(), naga::valid::Capabilities::empty())
        .validate(&module)
        .unwrap();
}

use crate::{Renderer, gpu::test_support};
use blackbox_gfx::{
    BlendMode, DirectionalLight, FrameParams, GlossyMaterial, GlossyModel, GlossySurface, Instance, LightingRig,
    Projection, ReflectionQuality, ReflectionScene, RenderBackend, RenderExperiments, Shading,
};
use blackbox_gfx_testkit::mesh::MeshBuilder;
use glam::{Mat4, Vec3};
const SIZE: (u32, u32) = (96, 96);

fn frame() -> FrameParams {
    let eye = Vec3::new(0.0, 0.0, 4.0);
    FrameParams {
        view: glam::camera::rh::view::look_at_mat4(eye, Vec3::ZERO, Vec3::Y),
        projection: Projection::PerspectiveInfiniteReverse { fov_y: 1.0, aspect: 1.0, near: 0.1 },
        camera_position: eye,
        light_dir: Vec3::NEG_Z,
        clear_color: [0.0; 3],
        fog: None,
        scattering: None,
        surface_light: None,
        camera_cut: false,
        shadow_tuning: None,
    }
}
fn material(roughness: f32) -> GlossyMaterial {
    GlossyMaterial {
        surface: Some(GlossySurface { roughness, coat: 0.0, ..Default::default() }),
        diffuse_min: [0.0, 0.0, 0.0, 1.0],
        diffuse_range: [0.0; 4],
        specular_min: [0.5; 3],
        specular_range: [0.0; 3],
        envmap_min: 2.0,
        envmap_range: 0.0,
        ..Default::default()
    }
}
fn mesh(r: &mut Renderer, desc: GlossyMaterial) -> Vec<Instance> {
    mesh_draw(r, desc, 128, BlendMode::Opaque)
}
fn mesh_draw(r: &mut Renderer, desc: GlossyMaterial, baked: u8, blend: BlendMode) -> Vec<Instance> {
    let material = r.create_glossy_material(&desc);
    let mut mesh = MeshBuilder::new();
    mesh.state(None, blend, Shading::Glossy(material));
    mesh.flat_quad(
        [Vec3::new(-5.0, -5.0, 0.0), Vec3::new(5.0, -5.0, 0.0), Vec3::new(5.0, 5.0, 0.0), Vec3::new(-5.0, 5.0, 0.0)],
        [baked, baked, baked, 255],
    );
    vec![Instance::new(mesh.finish().upload(r, "surface receiver"), Mat4::IDENTITY)]
}

#[path = "chrome_tests.rs"]
mod chrome;
#[path = "layer_tests.rs"]
mod layers;
fn rig(r: &mut Renderer, intensity: f32) {
    r.set_lighting_rig(&LightingRig {
        lights: [
            DirectionalLight::new(Vec3::Z, Vec3::splat(intensity)),
            DirectionalLight::new(Vec3::Z, Vec3::ZERO),
            DirectionalLight::new(Vec3::Z, Vec3::ZERO),
        ],
        ambient: Vec3::ZERO,
        ..Default::default()
    });
}
fn center(image: &[u8]) -> u8 {
    image[(48 * 96 + 48) * 4]
}
fn linear(byte: u8) -> f32 {
    let value = f32::from(byte) / 255.0;
    if value <= 0.04045 {
        return value / 12.92;
    }
    ((value + 0.055) / 1.055).powf(2.4)
}
fn stats(image: &[u8]) -> (f32, f32) {
    let pixels: Vec<_> = (16..80).flat_map(|y| (16..80).map(move |x| f32::from(image[(y * 96 + x) * 4]))).collect();
    let mean = pixels.iter().sum::<f32>() / pixels.len() as f32;
    let variance = pixels.iter().map(|p| (p - mean).powi(2)).sum::<f32>() / pixels.len() as f32;
    (mean, variance.sqrt())
}

#[test]
#[ignore = "needs a GPU"]
fn rough_surfaces_preserve_constant_energy_and_refresh_after_probe_changes() {
    let _serial = test_support::serial();
    let Some(mut r) = test_support::headless(SIZE) else { return };
    rig(&mut r, 0.0);
    for roughness in [0.12, 0.35, 0.78, 1.0] {
        let instances = mesh(&mut r, material(roughness));
        for color in [40u8, 100, 180] {
            let face = [color, color, color, 255].repeat(64);
            r.set_environment_faces(8, [&face; 6]);
            let image = r.capture(96, 96, &frame(), &instances).unwrap();
            assert!(
                center(&image).abs_diff(color) <= 2,
                "roughness {roughness}, wanted {color}, got {}",
                center(&image)
            );
            assert_eq!(image, r.capture(96, 96, &frame(), &instances).unwrap(), "fixed inputs must not shimmer");
        }
    }
}

#[test]
#[ignore = "needs a GPU"]
fn rough_reflections_remove_sharp_details_without_just_darkening_them() {
    let _serial = test_support::serial();
    let Some(mut r) = test_support::headless(SIZE) else { return };
    rig(&mut r, 0.0);
    let face: Vec<u8> = (0..128 * 128)
        .flat_map(|i| {
            let color = match ((i % 128) / 4 + (i / 128) / 4) & 1 {
                0 => 20,
                _ => 230,
            };
            [color, color, color, 255]
        })
        .collect();
    r.set_environment_faces(128, [&face; 6]);
    let sharp = mesh(&mut r, material(0.12));
    let rough = mesh(&mut r, material(0.78));
    let (sharp_mean, sharp_contrast) = stats(&r.capture(96, 96, &frame(), &sharp).unwrap());
    let (rough_mean, rough_contrast) = stats(&r.capture(96, 96, &frame(), &rough).unwrap());
    eprintln!("reflection mean/contrast: sharp {sharp_mean}/{sharp_contrast}, rough {rough_mean}/{rough_contrast}");
    assert!(sharp_contrast > 30.0);
    assert!(rough_contrast < sharp_contrast * 0.55);
    assert!((rough_mean - sharp_mean).abs() < 25.0, "filtering should retain the environment's average energy");
}

#[test]
#[ignore = "needs a GPU"]
fn rough_highlights_have_lower_peaks_and_broader_tails() {
    let _serial = test_support::serial();
    let Some(mut r) = test_support::headless(SIZE) else { return };
    rig(&mut r, 10.0);
    let black = [0u8, 0, 0, 255].repeat(64);
    r.set_environment_faces(8, [&black; 6]);
    let mut values = Vec::new();
    for roughness in [0.24, 0.78] {
        let mut desc = material(roughness);
        desc.envmap_min = 0.0;
        let mut instances = mesh(&mut r, desc);
        let peak = center(&r.capture(96, 96, &frame(), &instances).unwrap());
        instances[0].transform = Mat4::from_rotation_y(30f32.to_radians());
        let tail = center(&r.capture(96, 96, &frame(), &instances).unwrap());
        values.push((peak, tail));
    }
    eprintln!("gloss peak/tail {:?}, matte {:?}", values[0], values[1]);
    assert!(values[1].0 < values[0].0 / 2);
    assert!(values[1].1 > values[0].1.saturating_mul(3));
}

#[test]
#[ignore = "needs a GPU"]
fn response_toggle_original_models_and_probe_restoration_keep_their_contracts() {
    let _serial = test_support::serial();
    let Some(mut r) = test_support::headless(SIZE) else { return };
    rig(&mut r, 0.3);
    let face = [90u8, 90, 90, 255].repeat(64);
    r.set_environment_faces(8, [&face; 6]);
    for model in [GlossyModel::Custom, GlossyModel::VertexLit, GlossyModel::Shadowed] {
        let desc = GlossyMaterial { model, ..material(0.78) };
        let opted = mesh(&mut r, desc);
        let old = mesh(&mut r, GlossyMaterial { surface: None, ..desc });
        r.set_render_experiments(RenderExperiments { paint_response: false, clearcoat: true, ..Default::default() });
        let legacy = r.capture(96, 96, &frame(), &old).unwrap();
        assert_eq!(legacy, r.capture(96, 96, &frame(), &opted).unwrap());
        if model != GlossyModel::Custom {
            r.set_render_experiments(RenderExperiments { clearcoat: true, ..Default::default() });
            assert_eq!(legacy, r.capture(96, 96, &frame(), &opted).unwrap());
        }
    }
    r.set_render_experiments(RenderExperiments::default());
    rig(&mut r, 0.0);
    let instances = mesh(&mut r, material(0.78));
    let before = r.capture(96, 96, &frame(), &instances).unwrap();
    r.set_reflection_scene(Some(ReflectionScene {
        sun_shadows: false,
        fade_distance: None,
        origin: Vec3::ZERO,
        frame: frame(),
        faces: [&[]; 6],
        quality: ReflectionQuality::Low,
        refresh: blackbox_gfx::reflection::ALL_FACES,
    }));
    let captured = r.capture(96, 96, &frame(), &instances).unwrap();
    assert_ne!(center(&before), center(&captured));
    r.set_reflection_scene(None);
    assert_eq!(before, r.capture(96, 96, &frame(), &instances).unwrap());
}

#[test]
#[ignore = "needs a GPU"]
fn matte_reflection_uses_linear_energy_and_live_brightness_without_an_ambient_floor() {
    let _serial = test_support::serial();
    let Some(mut r) = test_support::headless(SIZE) else { return };
    rig(&mut r, 0.0);
    let face = [200u8, 200, 200, 255].repeat(64);
    r.set_environment_faces(8, [&face; 6]);
    // Isolate environment light: the legacy sun glint ignores the rig color.
    let desc = GlossyMaterial { envmap_min: 0.0, specular_min: [0.0; 3], ..material(0.78) };
    let instances = mesh(&mut r, desc);
    let lit = center(&r.capture(96, 96, &frame(), &instances).unwrap());
    assert!((linear(lit) / linear(200) - 0.04).abs() < 0.003, "4 percent linear reflection, got {lit}");
    assert!(lit > 35, "4 percent of a display-encoded byte would erase the reflection: {lit}");
    let changed = blackbox_gfx::GraphicsSettings {
        vehicle_reflection_brightness: blackbox_gfx::reflection::brightness::VehicleBrightness(2.0),
        ..Default::default()
    };
    assert!(r.apply_graphics(&changed).is_exact());
    rig(&mut r, 0.0); // Updates the existing material buffer, without recreating its mesh.
    let brighter = center(&r.capture(96, 96, &frame(), &instances).unwrap());
    assert!((linear(brighter) / linear(lit) - 2.0).abs() < 0.12, "linear energy: {lit} -> {brighter}");
    let black = [0u8, 0, 0, 255].repeat(64);
    r.set_environment_faces(8, [&black; 6]);
    assert_eq!(center(&r.capture(96, 96, &frame(), &instances).unwrap()), 0, "no invented fill light");
    r.set_render_experiments(RenderExperiments { paint_response: false, ..Default::default() });
    assert_eq!(center(&r.capture(96, 96, &frame(), &instances).unwrap()), 0);
}

#[test]
#[ignore = "needs a GPU"]
fn rough_coat_keeps_reflections_soft_and_does_not_change_legacy_coat() {
    let _serial = test_support::serial();
    let Some(mut r) = test_support::headless(SIZE) else { return };
    rig(&mut r, 0.0);
    let face: Vec<_> = (0..128 * 128)
        .flat_map(|i| match ((i % 128) / 4 + (i / 128) / 4) & 1 {
            0 => [20u8, 20, 20, 255],
            _ => [230, 230, 230, 255],
        })
        .collect();
    r.set_environment_faces(128, [&face; 6]);
    let desc = GlossyMaterial {
        envmap_min: 0.0,
        specular_min: [0.0; 3],
        surface: Some(GlossySurface { roughness: 0.52, coat: 0.25, ..Default::default() }),
        ..material(0.52)
    };
    let instances = mesh(&mut r, desc);
    let legacy = mesh(&mut r, GlossyMaterial { surface: None, ..desc });
    r.set_render_experiments(RenderExperiments { clearcoat: true, ..Default::default() });
    let (rough_mean, rough_contrast) = stats(&r.capture(96, 96, &frame(), &instances).unwrap());
    let legacy_coat = r.capture(96, 96, &frame(), &legacy).unwrap();
    r.set_render_experiments(RenderExperiments { clearcoat: false, ..Default::default() });
    let (bare_mean, _) = stats(&r.capture(96, 96, &frame(), &instances).unwrap());
    assert!(rough_mean > bare_mean + 1.0, "coat should remain visible");
    assert!(rough_contrast < 8.0, "rough coat must not restore sharp checker detail: {rough_contrast}");
    r.set_render_experiments(RenderExperiments { paint_response: false, clearcoat: true, ..Default::default() });
    assert_eq!(legacy_coat, r.capture(96, 96, &frame(), &instances).unwrap());
}

#[test]
#[ignore = "needs a GPU"]
fn finite_sun_broadens_a_smooth_highlight_without_increasing_its_peak() {
    let _serial = test_support::serial();
    let mut r = test_support::headless(SIZE).expect("GPU adapter");
    let black = [0, 0, 0, 255];
    r.set_environment_faces(1, [&black; 6]);
    let instances = mesh(&mut r, material(0.08));
    let mut options = r.render_experiments();
    options.sun_angle = 0.0;
    r.set_render_experiments(options);
    rig(&mut r, 0.005);
    let point = r.capture(96, 96, &frame(), &instances).unwrap();
    options.sun_angle = 0.27;
    r.set_render_experiments(options);
    let small_disc = r.capture(96, 96, &frame(), &instances).unwrap();
    options.sun_angle = 2.0;
    r.set_render_experiments(options);
    let disc = r.capture(96, 96, &frame(), &instances).unwrap();
    let peak = |image: &[u8]| image.as_chunks::<4>().0.iter().map(|p| p[0]).max().unwrap();
    assert!(
        peak(&small_disc) < peak(&point),
        "physical radius must reduce a smooth peak: {} vs {}",
        peak(&small_disc),
        peak(&point)
    );
    let extent = |image: &[u8]| image.as_chunks::<4>().0.iter().filter(|p| p[0] > 1).count();
    assert!(peak(&disc) < peak(&point), "{} vs {}", peak(&disc), peak(&point));
    assert!(extent(&disc) > extent(&point), "{} vs {}", extent(&disc), extent(&point));
}
