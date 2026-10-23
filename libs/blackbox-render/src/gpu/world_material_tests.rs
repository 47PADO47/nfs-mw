//! Pixel fixtures measured independently with the original PC vertex/pixel shaders.
//! Synthetic inputs only; `docs/specs/world-material-routing.md` describes the contract.

use blackbox_gfx::{
    BlendMode, DrawRange, FrameParams, Instance, MeshDesc, PixelFormat, Projection, RenderBackend, RenderExperiments,
    RgbaImage, Scattering, Shading, SurfaceLight, TextureDesc, Vertex,
};
use glam::{Mat4, Vec3, Vec4};

use super::{Renderer, test_support};

const COLOR: [u8; 4] = [191, 128, 64, 102];

fn renderer() -> Renderer {
    let mut renderer = Renderer::headless((64, 64), test_support::options())
        .expect("the selected GPU API must be available for an original-material fixture");
    renderer.set_render_experiments(RenderExperiments { original_world_materials: true, ..Default::default() });
    renderer
}

fn frame() -> FrameParams {
    FrameParams {
        // Keep world-space z=1 for fog while projecting every quad vertex to clip z=0.5.
        view: Mat4::from_cols(Vec4::X, Vec4::Y, Vec4::new(0.0, 0.0, 0.5, 0.0), Vec4::W),
        projection: Projection::Identity,
        camera_position: Vec3::ZERO,
        light_dir: Vec3::NEG_Z,
        clear_color: [0.0; 3],
        fog: None,
        scattering: None,
        surface_light: Some(SurfaceLight {
            shadow_strength: 1.0,
            specular: [0.0; 3],
            specular_power: 1.0,
            reflection_scale: 0.0,
            reflective_specular_scale: 1.0,
            ambient: [0.2, 0.4, 0.6],
            ambient_alpha: 0.5,
            diffuse: [0.3, 0.2, 0.1],
        }),
        camera_cut: false,

        shadow_tuning: None,
    }
}

fn capture(
    renderer: &mut Renderer,
    frame: &FrameParams,
    shading: Shading,
    blend: BlendMode,
    color: [u8; 4],
    normal: [f32; 3],
) -> RgbaImage {
    let vertices = [[-1.0, -1.0, 1.0], [-1.0, 1.0, 1.0], [1.0, 1.0, 1.0], [1.0, -1.0, 1.0]].map(|position| Vertex {
        position,
        normal,
        color_bgra: color,
        uv: [0.5; 2],
    });
    let texture = renderer.create_texture(&TextureDesc {
        label: "synthetic original-material diffuse",
        width: 1,
        height: 1,
        format: PixelFormat::Rgba8,
        mips: vec![&[64, 128, 192, 128]],
    });
    let mesh = renderer.create_mesh(&MeshDesc {
        secondary_uvs: None,
        tangents: None,
        label: "synthetic original-material quad",
        vertices: &vertices,
        indices: &[0, 1, 2, 0, 2, 3],
        draws: vec![DrawRange {
            first_index: 0,
            index_count: 6,
            base_vertex: 0,
            texture: Some(texture),
            blend,
            shading,
            two_sided: true,
        }],
    });
    renderer.render(frame, &[Instance::new(mesh, Mat4::IDENTITY)]).unwrap();
    let image = renderer.read_output().expect("headless output").unwrap();
    renderer.destroy_mesh(mesh);
    renderer.destroy_texture(texture);
    image
}

fn assert_rgb(image: &RgbaImage, at: (u32, u32), expected: [u8; 3], label: &str) {
    let actual = image.pixel(at.0, at.1).unwrap();
    for (channel, expected) in expected.into_iter().enumerate() {
        // Legacy PS 1.1 rounding and pixel-center conventions differ slightly between APIs.
        assert!(
            actual[channel].abs_diff(expected) <= 2,
            "{label} at {at:?}: {actual:?}, expected channel {channel}={expected}"
        );
    }
}

#[test]
#[ignore = "needs a GPU"]
fn original_world_light_uses_authored_ambient_alpha_and_source_normal_length() {
    let _gpu = test_support::serial();
    let mut renderer = renderer();
    let mut frame = frame();
    for shading in [Shading::World, Shading::Tree] {
        let image = capture(&mut renderer, &frame, shading, BlendMode::Opaque, COLOR, [0.0, 0.0, 1.0]);
        assert_rgb(&image, (32, 32), [42, 77, 125], "independent baseline");
        frame.surface_light.as_mut().unwrap().ambient_alpha = 0.0;
        let image = capture(&mut renderer, &frame, shading, BlendMode::Opaque, COLOR, [0.0, 0.0, 1.0]);
        assert_rgb(&image, (32, 32), [38, 51, 38], "ambient intensity zero");
        frame.surface_light.as_mut().unwrap().ambient_alpha = 1.0;
        let image = capture(&mut renderer, &frame, shading, BlendMode::Opaque, COLOR, [0.0, 0.0, 1.0]);
        assert_rgb(&image, (32, 32), [45, 103, 211], "ambient intensity one");
        frame.surface_light.as_mut().unwrap().ambient_alpha = 0.5;
        let image = capture(&mut renderer, &frame, shading, BlendMode::Opaque, COLOR, [0.0, 0.0, 0.5]);
        assert_rgb(&image, (32, 32), [22, 51, 105], "source normals are not renormalized");
    }
    frame.surface_light.as_mut().unwrap().diffuse = [4.0, 3.0, 2.0];
    let image = capture(&mut renderer, &frame, Shading::World, BlendMode::Opaque, COLOR, [0.0, 0.0, 1.0]);
    assert_rgb(&image, (32, 32), [131, 255, 255], "vertex COLOR saturation precedes texture multiplication");
}

#[test]
#[ignore = "needs a GPU"]
fn original_world_and_tree_fog_differ_and_are_interpolated_from_vertices() {
    let _gpu = test_support::serial();
    let mut renderer = renderer();
    let mut frame = frame();
    frame.scattering = Some(Scattering {
        rayleigh: [0.2, 0.3, 0.4],
        mie: [0.1, 0.2, 0.3],
        g: 0.1,
        to_sun: Vec3::Z,
        distance_scale: 0.75,
        in_scatter_scale: 1.0,
    });
    for (shading, expected, center, right) in [
        (Shading::World, [46, 73, 84], [47, 71, 75], [47, 72, 79]),
        (Shading::Tree, [35, 61, 112], [32, 56, 109], [34, 59, 111]),
    ] {
        frame.camera_position = Vec3::ZERO;
        let image = capture(&mut renderer, &frame, shading, BlendMode::Opaque, COLOR, [0.0, 0.0, 1.0]);
        assert_rgb(&image, (32, 32), expected, "family-specific extinction");
        frame.camera_position = Vec3::new(2.0, 0.0, 0.0);
        let image = capture(&mut renderer, &frame, shading, BlendMode::Opaque, COLOR, [0.0, 0.0, 1.0]);
        assert_rgb(&image, (32, 32), center, "vertex fog center");
        assert_rgb(&image, (48, 32), right, "vertex interpolation differs from per-pixel fog");
    }
    for shading in [Shading::WorldNoFog, Shading::WorldPrelit] {
        let image = capture(&mut renderer, &frame, shading, BlendMode::Opaque, COLOR, [0.0, 0.0, 1.0]);
        assert_rgb(&image, (32, 32), [32, 128, 255], "unlit no-fog families ignore atmosphere");
    }
}

#[test]
#[ignore = "needs a GPU"]
fn original_alpha_distinguishes_tree_world_and_doubled_prelit() {
    let _gpu = test_support::serial();
    let mut renderer = renderer();
    let frame = frame();
    for (shading, expected) in [
        (Shading::World, [8, 15, 25]),
        (Shading::Tree, [21, 39, 63]),
        (Shading::WorldNoFog, [6, 26, 51]),
        (Shading::WorldPrelit, [13, 51, 102]),
    ] {
        let image = capture(&mut renderer, &frame, shading, BlendMode::AlphaBlend, COLOR, [0.0, 0.0, 1.0]);
        assert_rgb(&image, (32, 32), expected, "original alpha observable through blending");
    }
    let invisible_vertex = [COLOR[0], COLOR[1], COLOR[2], 0];
    for (shading, expected) in [(Shading::World, [0, 0, 0]), (Shading::Tree, [42, 77, 125])] {
        let image = capture(&mut renderer, &frame, shading, BlendMode::AlphaTest, invisible_vertex, [0.0, 0.0, 1.0]);
        assert_rgb(&image, (32, 32), expected, "tree cutout ignores vertex alpha");
    }
    let partial_vertex = [COLOR[0], COLOR[1], COLOR[2], 191];
    for (shading, expected) in [(Shading::WorldNoFog, [0, 0, 0]), (Shading::WorldPrelit, [32, 128, 255])] {
        let image = capture(&mut renderer, &frame, shading, BlendMode::AlphaTest, partial_vertex, [0.0, 0.0, 1.0]);
        assert_rgb(&image, (32, 32), expected, "doubled prelit alpha changes cutout coverage");
    }
}
