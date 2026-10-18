//! End-to-end HDR composition, resizing and SDR preview lifecycle.
use super::{post::display::Mapping, test_support};
use blackbox_gfx::{
    Antialiasing, FrameParams, PostSettings, Projection, Tonemap, UiLayer, UiMesh, UiTextureId, UiTexturePatch,
    UiVertex, Upscaler,
};
use glam::{Mat4, Vec3};
fn frame(colour: [f32; 3]) -> FrameParams {
    FrameParams {
        view: Mat4::IDENTITY,
        projection: Projection::Identity,
        camera_position: Vec3::ZERO,
        light_dir: Vec3::NEG_Z,
        clear_color: colour,
        fog: None,
        scattering: None,
        surface_light: None,
        camera_cut: false,

        shadow_tuning: None,
    }
}
fn hdr(r: &mut super::Renderer) {
    r.post.set_display(&r.device, Some(Mapping { peak: 1000.0 / 203.0, white: 203.0, linear: true }));
    r.ui.set_hdr_white(Some(203.0));
    r.refresh_targets();
}
fn layer(colour: [u8; 4]) -> UiLayer {
    UiLayer {
        pixels_per_point: 1.0,
        meshes: vec![UiMesh {
            vertices: [[0.0, 0.0], [16.0, 0.0], [16.0, 8.0], [0.0, 8.0]]
                .into_iter()
                .map(|position| UiVertex { position, uv: [0.5; 2], color_rgba: colour })
                .collect(),
            indices: vec![0, 1, 2, 0, 2, 3],
            texture: UiTextureId::from_raw(7),
            clip: [0.0, 0.0, 16.0, 8.0],
        }],
    }
}

#[test]
#[ignore = "requires a real GPU"]
fn hdr_highlights_survive_scaling_aa_and_sdr_preview_capture() {
    let _serial = test_support::serial();
    let mut r = test_support::headless((16, 8)).expect("HDR adapter");
    r.set_post_effects(PostSettings { tonemap: Tonemap::Aces, ..Default::default() });
    hdr(&mut r);
    assert!(!r.post_passes().contains(&"tonemap"), "Do not apply two tone mappers");
    let bright = frame([16.0; 3]);
    let baseline = r.capture_hdr_linear(16, 8, &bright, &[]).unwrap()[64][0];
    assert!((12.0..=12.5).contains(&baseline), "A 1000-nit peak must survive in scRGB: {baseline}");
    let blue = r.capture_hdr_linear(16, 8, &frame([0.0, 0.0, 1.0]), &[]).unwrap()[64];
    assert!(blue[0] < -0.01 && blue[2] > 2.0, "Preserve signed BT.709 transport of wide-gamut mapping: {blue:?}");
    for (scale, upscale, aa) in [
        (0.5, Upscaler::Bilinear, Antialiasing::Off),
        (0.5, Upscaler::Fsr1, Antialiasing::Off),
        (1.0, Upscaler::Bilinear, Antialiasing::Smaa),
    ] {
        r.set_render_scale(scale);
        r.set_upscaler(upscale);
        r.set_post_effects(PostSettings { tonemap: Tonemap::Aces, antialiasing: aa, ..Default::default() });
        let pixels = r.capture_hdr_linear(16, 8, &bright, &[]).unwrap();
        for value in pixels.iter().flat_map(|p| &p[..3]) {
            assert!((*value - baseline).abs() < 0.035, "scale={scale}, {upscale:?}/{aa:?}: {value} vs {baseline}");
        }
        let mapping = r.post.display;
        let ui = r.ui.hdr_white();
        let png = r.capture(13, 7, &bright, &[]).unwrap();
        assert_eq!(png.len(), 13 * 7 * 4);
        assert_eq!(r.post.display, mapping);
        assert_eq!(r.ui.hdr_white(), ui);
        assert!(r.capture_hdr_linear(16, 8, &bright, &[]).unwrap().iter().all(|p| p[0] > 1.0));
    }
    r.post.set_display(&r.device, None);
    assert!(r.post_passes().contains(&"tonemap"), "Restore the stored SDR selection");
}

#[test]
#[ignore = "requires a real GPU"]
fn hdr_ui_unpremultiplies_before_decoding_and_has_its_own_white() {
    let _serial = test_support::serial();
    let mut r = test_support::headless((16, 8)).expect("HDR UI adapter");
    hdr(&mut r);
    r.update_ui_texture(&UiTexturePatch { id: UiTextureId::from_raw(7), offset: None, size: [1, 1], rgba: &[255; 4] });
    r.set_ui_layer(layer([128; 4]));
    let black = frame([0.0; 3]);
    for nits in [100.0, 203.0, 300.0] {
        r.ui.set_hdr_white(Some(nits));
        let pixel = r.capture_hdr_linear(16, 8, &black, &[]).unwrap()[64];
        let expected = (128.0 / 255.0) * nits / 80.0;
        assert!((pixel[0] - expected).abs() < 0.003, "{nits} nits: {pixel:?} expected {expected}");
    }
    r.set_ui_layer(layer([0; 4]));
    for pixel in r.capture_hdr_linear(16, 8, &black, &[]).unwrap() {
        assert!(pixel.iter().all(|v| v.is_finite()));
        assert!(pixel[..3].iter().all(|v| v.abs() < 0.001));
    }
}

#[test]
#[ignore = "requires a real GPU"]
fn hdr_ui_preserves_additive_colour_and_coverage_without_occluding_the_scene() {
    let _serial = test_support::serial();
    let mut r = test_support::headless((16, 8)).expect("additive HDR UI adapter");
    hdr(&mut r);
    let background = frame([0.25; 3]);
    let baseline = r.capture_hdr_linear(16, 8, &background, &[]).unwrap()[64][0];
    // Authored minimap tiles encode additive blending by clearing alpha
    // after premultiplying their texture coverage. Egui also uses this encoding.
    for (texture, vertex, gamma) in [
        ([255, 128, 64, 0], [255; 4], 1.0_f32),
        ([128, 64, 32, 0], [255; 4], 128.0 / 255.0),
        ([255; 4], [128, 64, 32, 0], 128.0 / 255.0),
        ([0; 4], [255; 4], 0.0),
    ] {
        r.update_ui_texture(&UiTexturePatch {
            id: UiTextureId::from_raw(7),
            offset: None,
            size: [1, 1],
            rgba: &texture,
        });
        r.set_ui_layer(layer(vertex));
        let pixel = r.capture_hdr_linear(16, 8, &background, &[]).unwrap()[64];
        let linear = match gamma <= 0.04045 {
            true => gamma / 12.92,
            false => ((gamma + 0.055) / 1.055).powf(2.4),
        };
        let expected = baseline + linear * 203.0 / 80.0;
        assert!((pixel[0] - expected).abs() < 0.005, "{texture:?}/{vertex:?}: {pixel:?}, expected {expected}");
        assert!(pixel.iter().all(|value| value.is_finite()));
    }
}

#[test]
#[ignore = "requires a real GPU"]
fn temporal_upscaling_keeps_hdr_highlights_and_display_mapping_after_capture() {
    use blackbox_gfx::{GraphicsSettings, RenderBackend, UpscaleQuality};
    let _serial = test_support::serial();
    let mut r = test_support::headless((64, 32)).expect("HDR temporal adapter");
    assert!(r.capabilities().upscalers.contains(Upscaler::Fsr3));
    r.output = super::output::Output::texture_format(&r.device, (64, 32), wgpu::TextureFormat::Rgba16Float);
    hdr(&mut r);
    r.apply_graphics(&GraphicsSettings {
        upscaler: Upscaler::Fsr3,
        upscale_quality: UpscaleQuality::Quality,
        ..Default::default()
    });
    let mut bright = frame([16.0; 3]);
    bright.projection = Projection::PerspectiveInfiniteReverse { fov_y: 1.0, aspect: 2.0, near: 0.1 };
    for _ in 0..8 {
        r.render(&bright, &[]).unwrap();
    }
    // The presentation copy is deliberately omitted by post_passes().
    assert_eq!(r.post_passes(), ["fsr3", "hdr-display"]);
    let super::output::Output::Texture { texture, .. } = &r.output else { unreachable!() };
    let pixels = r.read_raw_texture(texture, (64, 32), 8).unwrap();
    for pixel in pixels.as_chunks::<8>().0 {
        let red = half::f16::from_le_bytes([pixel[0], pixel[1]]).to_f32();
        assert!((12.0..=12.5).contains(&red), "temporal reconstruction clipped the HDR peak: {red}");
    }
    let mapping = r.post.display;
    r.capture(32, 16, &bright, &[]).unwrap();
    assert_eq!(r.post.display, mapping);
    assert_eq!(r.post_passes(), ["fsr3", "hdr-display"]);
    r.render(&bright, &[]).unwrap();
}

#[test]
#[ignore = "requires a real GPU"]
fn authored_surface_and_treatment_keep_highlights_only_in_hdr() {
    use blackbox_gfx::{
        BlendMode, ColourTreatment, DrawRange, Instance, MeshDesc, RenderBackend, RenderExperiments, Shading,
        SurfaceLight, Vertex,
    };
    let _serial = test_support::serial();
    let mut r = test_support::headless((16, 8)).expect("HDR surface adapter");
    r.set_render_experiments(RenderExperiments { original_world_materials: true, ..Default::default() });
    let vertices = [[-1.0, -1.0, 0.5], [-1.0, 1.0, 0.5], [1.0, 1.0, 0.5], [1.0, -1.0, 0.5]].map(|position| Vertex {
        position,
        normal: [0.0, 0.0, 1.0],
        color_bgra: [255; 4],
        uv: [0.5; 2],
    });
    let mesh = r.create_mesh(&MeshDesc {
        label: "HDR authored surface",
        vertices: &vertices,
        indices: &[0, 1, 2, 0, 2, 3],
        secondary_uvs: None,
        tangents: None,
        draws: vec![DrawRange {
            first_index: 0,
            index_count: 6,
            base_vertex: 0,
            texture: None,
            blend: BlendMode::Opaque,
            shading: Shading::World,
        }],
    });
    let instances = [Instance::new(mesh, Mat4::IDENTITY)];
    let mut lit = frame([0.0; 3]);
    lit.surface_light = Some(SurfaceLight {
        ambient: [0.5; 3],
        ambient_alpha: 1.0,
        diffuse: [0.5; 3],
        specular: [0.0; 3],
        specular_power: 1.0,
        reflection_scale: 0.0,
        reflective_specular_scale: 1.0,
        shadow_strength: 1.0,
    });
    let sdr = r.capture(16, 8, &lit, &instances).unwrap();
    assert_eq!(&sdr[64 * 4..64 * 4 + 3], &[255; 3]);
    hdr(&mut r);
    let before = r.capture_hdr_linear(16, 8, &lit, &instances).unwrap()[64][0];
    assert!(before > 3.0, "Final material clamp must be lifted in HDR: {before}");
    r.set_colour_treatment(
        Some(&ColourTreatment {
            tint: [1.0; 3],
            luminance: [1.0 / 3.0; 3],
            desaturation: 1.0,
            black_intensity: 0.0,
            colour_intensity: 0.0,
            black_curve: [1.0; 4],
            colour_curve: [1.0; 4],
        }),
        None,
    );
    r.set_post_effects(PostSettings { colour_treatment: 1.0, ..Default::default() });
    let after = r.capture_hdr_linear(16, 8, &lit, &instances).unwrap()[64][0];
    assert!((after - before).abs() < 0.01, "Treatment must preserve extended highlights: {after} vs {before}");
    r.post.set_display(&r.device, None);
    let restored = r.capture(16, 8, &lit, &instances).unwrap();
    assert_eq!(restored, sdr, "SDR retains original arithmetic and clamp");
    r.destroy_mesh(mesh);
}
