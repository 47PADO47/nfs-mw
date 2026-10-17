//! Screen-space capture fixtures use synthetic geometry, not installed game assets.
use blackbox_gfx::*;
use glam::{Mat4, Vec3};

use super::super::{Renderer, test_support};

fn frame() -> FrameParams {
    FrameParams {
        view: Mat4::IDENTITY,
        projection: Projection::Identity,
        camera_position: Vec3::new(0.0, 0.0, 3.0),
        light_dir: Vec3::NEG_Z,
        clear_color: [0.0; 3],
        fog: None,
        scattering: None,
        camera_cut: false,

        shadow_tuning: None,
        surface_light: Some(SurfaceLight {
            ambient: [0.0; 3],
            ambient_alpha: 1.0,
            diffuse: [0.0; 3],
            specular: [0.0; 3],
            specular_power: 11.0,
            reflection_scale: 0.25,
            reflective_specular_scale: 1.0,
            shadow_strength: 1.0,
        }),
    }
}
fn upload(r: &mut Renderer, rgba: [u8; 4]) -> TextureHandle {
    r.create_texture(&TextureDesc {
        label: "projected synthetic texel",
        width: 1,
        height: 1,
        format: PixelFormat::Rgba8,
        mips: vec![&rgba],
    })
}
fn quad(r: &mut Renderer, bounds: [f32; 4], rgba: [u8; 4], texture: TextureHandle, shading: Shading) -> MeshHandle {
    let [x0, y0, x1, y1] = bounds;
    let [red, green, blue, alpha] = rgba;
    let vertices = [[x0, y0, 0.5], [x0, y1, 0.5], [x1, y1, 0.5], [x1, y0, 0.5]].map(|position| Vertex {
        position,
        normal: [0.0, 0.0, 1.0],
        color_bgra: [blue, green, red, alpha],
        uv: [0.5; 2],
    });
    r.create_mesh(&MeshDesc {
        label: "projected synthetic quad",
        vertices: &vertices,
        tangents: Some(&[[1.0, 0.0, 0.0, 1.0]; 4]),
        secondary_uvs: None,
        indices: &[0, 1, 2, 0, 2, 3],
        draws: vec![DrawRange {
            first_index: 0,
            index_count: 6,
            base_vertex: 0,
            texture: Some(texture),
            blend: BlendMode::Opaque,
            shading,
        }],
    })
}
fn material(r: &mut Renderer, textures: [TextureHandle; 3], reflection: Option<TextureHandle>) -> NormalMaterialHandle {
    r.create_normal_material(&NormalMaterialDesc {
        model: NormalModel::Reflective,
        diffuse: textures[0],
        normal: textures[1],
        specular: textures[2],
        reflection,
        uv_offset: [0.0; 2],
    })
    .unwrap()
}
fn image(r: &mut Renderer, receiver: MeshHandle) -> RgbaImage {
    r.render(&frame(), &[Instance::new(receiver, Mat4::IDENTITY)]).unwrap();
    r.read_output().expect("headless capture").unwrap()
}
fn rgb(image: &RgbaImage, xy: (u32, u32), expected: [u8; 3]) {
    let actual = image.pixel(xy.0, xy.1).unwrap();
    for c in 0..3 {
        assert!(actual[c].abs_diff(expected[c]) <= 2, "pixel {xy:?}: {actual:?}, expected {expected:?}");
    }
}
fn capture(r: &mut Renderer, frame: FrameParams, size: [u32; 2], instances: &[Instance], blur: bool) {
    r.set_projected_reflection_scene(Some(ProjectedReflectionScene { frame, size, instances, blur }));
}

#[test]
#[ignore = "needs a GPU"]
fn projected_scene_updates_receivers_without_feedback_or_resource_churn() {
    let _gpu = test_support::serial();
    let mut r = Renderer::headless((64, 64), test_support::options()).expect("selected GPU required");
    assert!(r.capabilities().projected_reflections, "selected API needs projected capture support");
    r.set_render_experiments(RenderExperiments { original_world_materials: true, ..Default::default() });
    let initial = r.resource_counts();
    let white = upload(&mut r, [255; 4]);
    let black = upload(&mut r, [0, 0, 0, 255]);
    let flat = upload(&mut r, [128, 128, 255, 255]);
    let red_mask = upload(&mut r, [255, 0, 0, 255]);
    let fixed = upload(&mut r, [37, 83, 149, 255]);
    let normal = material(&mut r, [black, flat, red_mask], None);
    let explicit = material(&mut r, [black, flat, red_mask], Some(fixed));
    let receiver = quad(&mut r, [-1.0, -1.0, 1.0, 1.0], [255; 4], black, Shading::WorldNormal(normal));
    let fixed_receiver = quad(&mut r, [-1.0, -1.0, 1.0, 1.0], [255; 4], black, Shading::WorldNormal(explicit));
    // Clockwise screen quadrants: source World must use unlit NoFog in capture.
    let sources = [
        ([-0.8, 0.0, 0.0, 0.8], [64, 0, 0, 255]),
        ([0.0, 0.0, 0.8, 0.8], [0, 64, 0, 255]),
        ([-0.8, -0.8, 0.0, 0.0], [0, 0, 64, 255]),
        ([0.0, -0.8, 0.8, 0.0], [64, 64, 0, 255]),
    ]
    .map(|(bounds, color)| quad(&mut r, bounds, color, white, Shading::World));
    let instances = sources.map(|mesh| Instance::new(mesh, Mat4::IDENTITY));
    let counts = r.resource_counts();
    let mut source_frame = frame();
    source_frame.clear_color = [0.4, 0.2, 0.1]; // capture must still clear opaque black.
    capture(&mut r, source_frame, [64, 64], &instances, false);
    let first = image(&mut r, receiver);
    for (xy, color) in [
        ((16, 16), [128, 0, 0]),
        ((48, 16), [0, 128, 0]),
        ((16, 48), [0, 0, 128]),
        ((48, 48), [128, 128, 0]),
        ((2, 2), [0; 3]),
    ] {
        rgb(&first, xy, color);
    }
    rgb(&image(&mut r, fixed_receiver), (32, 32), [37, 83, 149]);
    capture(&mut r, source_frame, [64, 64], &instances, false);
    assert_eq!(image(&mut r, receiver).rgba, first.rgba, "capture has no temporal accumulation");
    // Reusing the same target must use the current view, not last frame's camera.
    source_frame.view = Mat4::from_translation(Vec3::new(0.5, 0.0, 0.0));
    capture(&mut r, source_frame, [64, 64], &instances, false);
    rgb(&image(&mut r, receiver), (16, 16), [0; 3]);
    source_frame.view = Mat4::IDENTITY;
    // Reallocation and blur changes preserve live material handles and explicit bindings.
    for (size, blur) in [([32, 16], false), ([64, 64], true), ([64, 64], false)] {
        capture(&mut r, source_frame, size, &instances, blur);
        rgb(&image(&mut r, receiver), (16, 16), [128, 0, 0]);
        rgb(&image(&mut r, fixed_receiver), (32, 32), [37, 83, 149]);
        assert_eq!(r.resource_counts(), counts, "capture must not create public mesh/texture handles");
    }
    // A late receiver inherits the retained image without requiring recapture.
    let late = material(&mut r, [black, flat, red_mask], None);
    let late_mesh = quad(&mut r, [-1.0, -1.0, 1.0, 1.0], [255; 4], black, Shading::WorldNormal(late));
    rgb(&image(&mut r, late_mesh), (16, 16), [128, 0, 0]);
    r.set_projected_reflection_scene(None);
    rgb(&image(&mut r, receiver), (16, 16), [0; 3]);
    rgb(&image(&mut r, late_mesh), (16, 16), [0; 3]);
    rgb(&image(&mut r, fixed_receiver), (16, 16), [37, 83, 149]);
    assert!(r.projected.is_none());
    for mesh in sources.into_iter().chain([receiver, fixed_receiver, late_mesh]) {
        r.destroy_mesh(mesh);
    }
    for handle in [normal, explicit, late] {
        r.destroy_normal_material(handle);
    }
    for texture in [white, black, flat, red_mask, fixed] {
        r.destroy_texture(texture);
    }
    assert_eq!(r.resource_counts(), initial);
}

#[test]
#[ignore = "needs a GPU"]
fn projected_capture_normal_family_uses_verified_fixed_function_colour() {
    let _gpu = test_support::serial();
    let mut r = Renderer::headless((64, 64), test_support::options()).expect("selected GPU required");
    r.set_render_experiments(RenderExperiments { original_world_materials: true, ..Default::default() });
    let diffuse = upload(&mut r, [200, 20, 90, 128]);
    let black = upload(&mut r, [0, 0, 0, 255]);
    let flat = upload(&mut r, [128, 128, 255, 255]);
    let mask = upload(&mut r, [255, 0, 0, 255]);
    let receive_material = material(&mut r, [black, flat, mask], None);
    let receiver = quad(&mut r, [-1.0, -1.0, 1.0, 1.0], [255; 4], black, Shading::WorldNormal(receive_material));
    for model in [NormalModel::Bump, NormalModel::Reflective] {
        let source = r
            .create_normal_material(&NormalMaterialDesc {
                model,
                diffuse,
                normal: flat,
                specular: mask,
                reflection: None,
                uv_offset: [0.0; 2],
            })
            .unwrap();
        for alpha in [0, 102, 255] {
            // Draw.texture deliberately differs: capture uses the material's diffuse binding.
            let mesh = quad(&mut r, [-1.0, -1.0, 1.0, 1.0], [64, 128, 192, alpha], black, Shading::WorldNormal(source));
            capture(&mut r, frame(), [64, 64], &[Instance::new(mesh, Mat4::IDENTITY)], false);
            rgb(&image(&mut r, receiver), (32, 32), [100, 20, 135]);
            r.destroy_mesh(mesh);
        }
        r.destroy_normal_material(source);
    }
}

#[test]
#[ignore = "needs a GPU"]
fn projected_car_capture_selects_vertex_lit_family_without_changing_main_material() {
    let _gpu = test_support::serial();
    let mut r = Renderer::headless((64, 64), test_support::options()).expect("selected GPU required");
    r.set_render_experiments(RenderExperiments { original_world_materials: true, ..Default::default() });
    r.set_lighting_rig(&LightingRig::default());
    let white = upload(&mut r, [128, 96, 64, 255]);
    let black = upload(&mut r, [0, 0, 0, 255]);
    let flat = upload(&mut r, [128, 128, 255, 255]);
    let mask = upload(&mut r, [255, 0, 0, 255]);
    let normal = material(&mut r, [black, flat, mask], None);
    let receiver = quad(&mut r, [-1.0, -1.0, 1.0, 1.0], [255; 4], black, Shading::WorldNormal(normal));
    let desc =
        GlossyMaterial { model: GlossyModel::Shadowed, specular_min: [0.0; 3], envmap_min: 0.5, ..Default::default() };
    let modern = r.create_glossy_material(&desc);
    let legacy = r.create_glossy_material(&GlossyMaterial { model: GlossyModel::VertexLit, ..desc });
    let modern_mesh = quad(&mut r, [-1.0, -1.0, 1.0, 1.0], [128; 4], white, Shading::Glossy(modern));
    let legacy_mesh = quad(&mut r, [-1.0, -1.0, 1.0, 1.0], [128; 4], white, Shading::Glossy(legacy));
    let expected = image(&mut r, legacy_mesh);
    let main_before = image(&mut r, modern_mesh);
    capture(&mut r, frame(), [64, 64], &[Instance::new(modern_mesh, Mat4::IDENTITY)], false);
    let actual = image(&mut r, receiver);
    for xy in [(16, 16), (32, 32), (48, 48)] {
        let color = expected.pixel(xy.0, xy.1).unwrap();
        rgb(&actual, xy, [color[0], color[1], color[2]]);
    }
    assert_ne!(actual.rgba, main_before.rgba, "fixture distinguishes the two original families");
    assert_eq!(image(&mut r, modern_mesh).rgba, main_before.rgba, "capture must not mutate registered materials");
}

#[test]
#[ignore = "needs a GPU"]
fn projected_capture_excludes_post_and_effects_and_blurs_texel_footprint() {
    let _gpu = test_support::serial();
    let mut r = Renderer::headless((64, 64), test_support::options()).expect("selected GPU required");
    r.set_render_experiments(RenderExperiments { original_world_materials: true, ..Default::default() });
    let white = upload(&mut r, [255; 4]);
    let black = upload(&mut r, [0, 0, 0, 255]);
    let flat = upload(&mut r, [128, 128, 255, 255]);
    let mask = upload(&mut r, [255, 0, 0, 255]);
    let normal = material(&mut r, [black, flat, mask], None);
    let receiver = quad(&mut r, [-1.0, -1.0, 1.0, 1.0], [255; 4], black, Shading::WorldNormal(normal));
    let source = quad(&mut r, [-1.0, -1.0, 0.0, 1.0], [64, 64, 64, 255], white, Shading::WorldNoFog);
    let source_instances = [Instance::new(source, Mat4::IDENTITY)];
    let mut effects = EffectLayer::default();
    EffectLayer::quad(
        &mut effects.glows,
        [Vec3::new(-1.0, -1.0, 0.9), Vec3::new(-1.0, 1.0, 0.9), Vec3::new(1.0, 1.0, 0.9), Vec3::new(1.0, -1.0, 0.9)],
        [255, 0, 0, 255],
    );
    r.set_effects(&effects);
    r.set_post_effects(PostSettings { tonemap: Tonemap::Aces, ..Default::default() });
    capture(&mut r, frame(), [64, 64], &source_instances, false);
    r.set_effects(&EffectLayer::default());
    r.set_post_effects(PostSettings::default());
    let raw = image(&mut r, receiver);
    rgb(&raw, (16, 32), [128; 3]);
    rgb(&raw, (34, 32), [0; 3]);
    capture(&mut r, frame(), [64, 64], &source_instances, true);
    let blurred = image(&mut r, receiver);
    // Four-tap source kernel straddles the half-plane edge; unchanged solid interiors.
    rgb(&blurred, (16, 32), [128; 3]);
    let edge = blurred.pixel(33, 32).unwrap();
    assert!(edge[0] > 10 && edge[0] < 100, "blur must spread across the edge, got {edge:?}");
    assert!(edge[0].abs_diff(edge[1]) <= 1 && edge[1].abs_diff(edge[2]) <= 1);
    // Both axes use texel sizes from the new target, not the main output size.
    capture(&mut r, frame(), [32, 16], &source_instances, true);
    let resized = image(&mut r, receiver);
    assert!(resized.pixel(35, 32).unwrap()[0] > 0, "lower-resolution blur must widen in screen space");
    capture(&mut r, frame(), [0, 64], &source_instances, false);
    rgb(&image(&mut r, receiver), (16, 32), [0; 3]);
    assert!(r.projected.is_none(), "zero-size capture disables and releases its target");
}

/// Deliberately retain main-view screen state without recomputing its passes. The tiny
/// disabled masks are initialized black by WebGPU; enabling them is an asymmetric
/// fixture for stale AO/contact coverage, independent of the AO generation algorithm.
fn main_screen_state(r: &Renderer, ao: bool, contact: bool, debug: bool) {
    let data = &r.shared.shadows.buffer;
    // Shader ShadowData: four matrices followed by splits/options/camera/... vectors.
    for (offset, value) in [
        (384, [1.0, 1.0, u8::from(contact) as f32, 0.0]),
        (400, [1.0, 1.0, u8::from(ao) as f32, 0.0]),
        (448, [if debug { 2.0_f32 } else { 0.0 }, 0.0, 0.0, 0.0]),
    ] {
        r.queue.write_buffer(data, offset, bytemuck::cast_slice(&value));
    }
}

/// Draw just the material stage so injected receiver state is not replaced by a new
/// camera prepass. This exercises the same main/capture fragment programs.
fn material_stage_image(r: &mut Renderer, mesh: MeshHandle) -> RgbaImage {
    let super::super::output::Output::Texture { texture, .. } = &r.output else { panic!("headless output") };
    let output = texture.create_view(&Default::default());
    let instances = [Instance::new(mesh, Mat4::IDENTITY)];
    r.instances.upload_instances(&r.device, &r.queue, &instances);
    let mut encoder = r.device.create_command_encoder(&Default::default());
    r.encode_scene_mode(&mut encoder, None, &output, &frame(), &instances, false, None);
    r.queue.submit([encoder.finish()]);
    r.read_output().expect("headless output").unwrap()
}

#[test]
#[ignore = "needs a GPU"]
fn projected_custom_cars_ignore_main_ao_contact_and_debug_without_disabling_main_effects() {
    let _gpu = test_support::serial();
    let mut r = Renderer::headless((64, 64), test_support::options()).expect("selected GPU required");
    r.set_render_experiments(RenderExperiments { original_world_materials: true, ..Default::default() });
    r.set_lighting_rig(&LightingRig::default());
    let texture = upload(&mut r, [128, 96, 64, 255]);
    let black = upload(&mut r, [0, 0, 0, 255]);
    let flat = upload(&mut r, [128, 128, 255, 255]);
    let mask = upload(&mut r, [255, 0, 0, 255]);
    let custom = r.create_glossy_material(&GlossyMaterial {
        model: GlossyModel::Custom,
        specular_min: [0.0; 3],
        envmap_min: 0.0,
        ..Default::default()
    });
    let source = quad(&mut r, [-1.0, -1.0, 1.0, 1.0], [128; 4], texture, Shading::Glossy(custom));
    let normal = material(&mut r, [black, flat, mask], None);
    let receiver = quad(&mut r, [-1.0, -1.0, 1.0, 1.0], [255; 4], black, Shading::WorldNormal(normal));
    let instances = [Instance::new(source, Mat4::IDENTITY)];
    main_screen_state(&r, false, false, false);
    let main_neutral = material_stage_image(&mut r, source);
    capture(&mut r, frame(), [32, 16], &instances, false);
    let reflection_neutral = image(&mut r, receiver);
    assert!(main_neutral.pixel(32, 32).unwrap()[0] > 20, "fixture must contain lit car colour");
    for (ao, contact, debug) in [(true, false, false), (false, true, false), (false, false, true)] {
        main_screen_state(&r, ao, contact, debug);
        let main_changed = material_stage_image(&mut r, source);
        assert_ne!(
            main_changed.pixel(32, 32),
            main_neutral.pixel(32, 32),
            "main view must still respond to AO={ao}, contact={contact}, debug={debug}"
        );
        capture(&mut r, frame(), [32, 16], &instances, false);
        let reflection = image(&mut r, receiver);
        for xy in [(16, 16), (32, 32), (48, 48)] {
            let expected = reflection_neutral.pixel(xy.0, xy.1).unwrap();
            rgb(&reflection, xy, [expected[0], expected[1], expected[2]]);
        }
    }
    main_screen_state(&r, false, false, false);
    assert_eq!(
        material_stage_image(&mut r, source).rgba,
        main_neutral.rgba,
        "neutral main-view shading must round trip"
    );
}
