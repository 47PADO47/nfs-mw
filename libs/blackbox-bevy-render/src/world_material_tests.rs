//! Original-shader numeric fixtures exercising the authored WESL material variants.
use blackbox_gfx::{
    BlendMode, DrawRange, FrameParams, GraphicsApi, Instance, MeshDesc, PixelFormat, Projection, RenderBackend,
    RenderExperiments, RgbaImage, Scattering, Shading, SurfaceLight, TextureDesc, Vertex,
};
use blackbox_gpu_passes::test_support::serial;
use glam::{Mat4, Vec3};

use crate::HeadlessBevy;

const COLOR: [u8; 4] = [191, 128, 64, 102];

fn frame() -> FrameParams {
    FrameParams {
        // A 90-degree perspective at z=1 projects the synthetic square to the viewport edges.
        view: Mat4::from_rotation_y(std::f32::consts::PI),
        projection: Projection::PerspectiveInfiniteReverse {
            fov_y: std::f32::consts::FRAC_PI_2,
            aspect: 1.0,
            near: 0.1,
        },
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
    }
}

fn capture(
    renderer: &mut HeadlessBevy,
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
        label: "synthetic original-material quad",
        vertices: &vertices,
        secondary_uvs: None,
        tangents: None,
        indices: &[0, 1, 2, 0, 2, 3],
        draws: vec![DrawRange {
            first_index: 0,
            index_count: 6,
            base_vertex: 0,
            texture: Some(texture),
            blend,
            shading,
        }],
    });
    let instances = [Instance::new(mesh, Mat4::IDENTITY)];
    let id = renderer.request_capture([64, 64], frame, &instances).unwrap();
    for _ in 0..120 {
        renderer.render(frame, &instances).unwrap();
        if let Some(image) = renderer.poll_capture(id) {
            renderer.destroy_mesh(mesh);
            renderer.destroy_texture(texture);
            return image.unwrap();
        }
    }
    panic!("original-material capture timed out");
}

fn assert_rgb(image: &RgbaImage, expected: [u8; 3], label: &str) {
    let actual = image.pixel(32, 32).unwrap();
    for (channel, expected) in expected.into_iter().enumerate() {
        assert!(actual[channel].abs_diff(expected) <= 2, "{label}: {actual:?}, expected channel {channel}={expected}");
    }
}

fn original_world_material_fixtures(api: GraphicsApi) {
    let _gpu = serial();
    let _ = tracing_subscriber::fmt().with_env_filter("warn").with_test_writer().try_init();
    let mut renderer = HeadlessBevy::new([64, 64], api, false).expect("the selected GPU API must be available");
    renderer.set_render_experiments(RenderExperiments { original_world_materials: true, ..Default::default() });
    let mut frame = frame();
    for shading in [Shading::World, Shading::Tree] {
        let image = capture(&mut renderer, &frame, shading, BlendMode::Opaque, COLOR, [0.0, 0.0, 1.0]);
        assert_rgb(&image, [42, 77, 125], "original baseline");
        frame.surface_light.as_mut().unwrap().ambient_alpha = 0.0;
        let image = capture(&mut renderer, &frame, shading, BlendMode::Opaque, COLOR, [0.0, 0.0, 1.0]);
        assert_rgb(&image, [38, 51, 38], "authored ambient intensity");
        frame.surface_light.as_mut().unwrap().ambient_alpha = 0.5;
        let image = capture(&mut renderer, &frame, shading, BlendMode::Opaque, COLOR, [0.0, 0.0, 0.5]);
        assert_rgb(&image, [22, 51, 105], "source normal magnitude");
    }
    frame.scattering = Some(Scattering {
        rayleigh: [0.2, 0.3, 0.4],
        mie: [0.1, 0.2, 0.3],
        g: 0.1,
        to_sun: Vec3::Z,
        distance_scale: 0.75,
        in_scatter_scale: 1.0,
    });
    for (shading, expected) in [
        (Shading::World, [46, 73, 84]),
        (Shading::Tree, [35, 61, 112]),
        (Shading::WorldNoFog, [32, 128, 255]),
        (Shading::WorldPrelit, [32, 128, 255]),
    ] {
        let image = capture(&mut renderer, &frame, shading, BlendMode::Opaque, COLOR, [0.0, 0.0, 1.0]);
        assert_rgb(&image, expected, "original family atmospheric behavior");
    }
    frame.scattering = None;
    for (shading, color, expected) in [
        (Shading::World, [191, 128, 64, 0], [0, 0, 0]),
        (Shading::Tree, [191, 128, 64, 0], [42, 77, 125]),
        (Shading::WorldNoFog, [191, 128, 64, 191], [0, 0, 0]),
        (Shading::WorldPrelit, [191, 128, 64, 191], [32, 128, 255]),
    ] {
        let image = capture(&mut renderer, &frame, shading, BlendMode::AlphaTest, color, [0.0, 0.0, 1.0]);
        assert_rgb(&image, expected, "original alpha-test channel selection");
    }
}

#[test]
#[ignore = "needs a GPU"]
fn original_world_material_wesl_matches_pc_shader_fixtures_vulkan() {
    original_world_material_fixtures(GraphicsApi::Vulkan);
}

#[test]
#[ignore = "needs a GPU"]
fn original_world_material_wesl_matches_pc_shader_fixtures_dx12() {
    original_world_material_fixtures(GraphicsApi::Dx12);
}
