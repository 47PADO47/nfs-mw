//! Synthetic pixels independently measured with the original PC window shader.
use crate::{Renderer, gpu::test_support};
use blackbox_gfx::{
    BlendMode, DrawRange, FrameParams, Instance, MeshDesc, PixelFormat, Projection, RenderBackend, RenderExperiments,
    RgbaImage, Scattering, Shading, SurfaceLight, TextureDesc, Vertex, WindowMaterialDesc,
};
use glam::{Mat4, Vec3, Vec4};

struct Fixture {
    normal: [f32; 3],
    light: [f32; 3],
    eye: [f32; 3],
    ambient_alpha: f32,
    vertex_alpha: u8,
    texture_alpha: u8,
    black_reflection: bool,
    reflection_alpha: u8,
    fog: bool,
}
impl Default for Fixture {
    fn default() -> Self {
        Self {
            normal: [0.0, 0.0, 1.0],
            light: [0.0, 0.0, 1.0],
            eye: [0.0, 0.0, 3.0],
            ambient_alpha: 0.5,
            vertex_alpha: 102,
            texture_alpha: 128,
            black_reflection: false,
            reflection_alpha: 255,
            fog: false,
        }
    }
}
fn fixture(case: usize) -> Fixture {
    let mut f = Fixture::default();
    match case {
        1 => f.vertex_alpha = 0,
        2 => f.vertex_alpha = 255,
        3 => f.texture_alpha = 0,
        4 => f.texture_alpha = 255,
        5 => f.black_reflection = true,
        6 => f.reflection_alpha = 0,
        10 => f.normal = [0.0, 0.0, 0.5],
        11 => f.normal = [0.6, 0.0, 0.8],
        12 => f.light = [0.4, 0.0, 0.916_515_1],
        13 => f.eye = [2.0, 0.0, 3.0],
        14 => f.ambient_alpha = 0.0,
        15 => f.ambient_alpha = 1.0,
        16 | 17 => {
            f.fog = true;
            if case == 17 {
                f.ambient_alpha = 0.0;
            }
        }
        _ => {}
    }
    f
}
fn capture(renderer: &mut Renderer, f: &Fixture, blend: BlendMode) -> RgbaImage {
    let diffuse = renderer.create_texture(&TextureDesc {
        label: "synthetic window base",
        width: 1,
        height: 1,
        format: PixelFormat::Rgba8,
        mips: vec![&[64, 128, 192, f.texture_alpha]],
    });
    let reflection_pixels: Vec<u8> = (0..4)
        .flat_map(|y| {
            (0..4).flat_map(move |x| match f.black_reflection {
                true => [0, 0, 0, f.reflection_alpha],
                false => [40 + 35 * x, 20 + 40 * y, 100, f.reflection_alpha],
            })
        })
        .collect();
    let reflection = renderer.create_texture(&TextureDesc {
        label: "synthetic authored window reflection",
        width: 4,
        height: 4,
        format: PixelFormat::Rgba8,
        mips: vec![&reflection_pixels],
    });
    let material = renderer
        .create_window_material(&WindowMaterialDesc { diffuse, reflection })
        .expect("native window material support");
    let vertices = [[-1.0, -1.0, 1.0], [-1.0, 1.0, 1.0], [1.0, 1.0, 1.0], [1.0, -1.0, 1.0]].map(|position| Vertex {
        position,
        normal: f.normal,
        color_bgra: [191, 128, 64, f.vertex_alpha],
        uv: [0.25, 0.25],
    });
    let mesh = renderer.create_mesh(&MeshDesc {
        label: "synthetic original window quad",
        vertices: &vertices,
        secondary_uvs: None,
        tangents: None,
        indices: &[0, 1, 2, 0, 2, 3],
        draws: vec![DrawRange {
            first_index: 0,
            index_count: 6,
            base_vertex: 0,
            texture: Some(diffuse),
            blend,
            shading: Shading::WorldWindow(material),
        }],
    });
    let frame = FrameParams {
        // Match D3D9 integer centers at local x=0/.5 using a half-pixel clip shift.
        view: Mat4::from_cols(
            Vec4::X,
            Vec4::Y,
            Vec4::new(0.0, 0.0, 0.5, 0.0),
            Vec4::new(1.0 / 64.0, -1.0 / 64.0, 0.0, 1.0),
        ),
        projection: Projection::Identity,
        camera_position: Vec3::from_array(f.eye),
        light_dir: -Vec3::from_array(f.light),
        clear_color: [0.0; 3],
        fog: None,
        scattering: f.fog.then_some(Scattering {
            rayleigh: [0.2, 0.3, 0.4],
            mie: [0.1, 0.2, 0.3],
            g: 0.1,
            to_sun: Vec3::Z,
            distance_scale: 0.75,
            in_scatter_scale: 1.0,
        }),
        surface_light: Some(SurfaceLight {
            shadow_strength: 1.0,
            ambient: [0.2, 0.4, 0.6],
            ambient_alpha: f.ambient_alpha,
            diffuse: [0.3, 0.2, 0.1],
            specular: [0.0; 3],
            specular_power: 1.0,
            reflection_scale: 0.0,
            reflective_specular_scale: 1.0,
        }),
        camera_cut: false,

        shadow_tuning: None,
    };
    renderer.render(&frame, &[Instance::new(mesh, Mat4::IDENTITY)]).unwrap();
    let image = renderer.read_output().expect("headless output").unwrap();
    renderer.destroy_mesh(mesh);
    renderer.destroy_window_material(material);
    renderer.destroy_texture(diffuse);
    renderer.destroy_texture(reflection);
    image
}
fn assert_rgb(image: &RgbaImage, x: u32, expected: [u8; 3], label: &str) {
    let pixel = image.pixel(x, 32).unwrap();
    for (channel, expected) in expected.into_iter().enumerate() {
        assert!(
            pixel[channel].abs_diff(expected) <= 2,
            "{label} at x={x}: {pixel:?}; expected channel {channel}={expected}"
        );
    }
}
#[test]
#[ignore = "needs a GPU"]
fn authored_window_mapping_matches_independent_pc_shader_pixels() {
    let _gpu = test_support::serial();
    let mut renderer = Renderer::headless((64, 64), test_support::options()).expect("selected GPU API");
    renderer.set_render_experiments(RenderExperiments { original_world_materials: true, ..Default::default() });
    // Original programmable unshadowed and fully lit shadowed pairs agree independently.
    // Exact reflection-UV coordinates, LINEAR/Repeat sampler and vertex COLOR bytes
    // match those inputs. Contrived independent WorldView rows and FLOAT4 color>1
    // cannot be expressed by this ordinary mesh/frame path and are not claimed here.
    let expected = [
        (0, "base", [129, 154, 242], [129, 154, 242]),
        (1, "vertex_alpha_zero", [129, 154, 242], [129, 154, 242]),
        (2, "vertex_alpha_one", [129, 154, 242], [129, 154, 242]),
        (3, "texture_alpha_zero", [42, 77, 125], [42, 77, 125]),
        (4, "texture_alpha_one", [216, 231, 255], [216, 231, 255]),
        (5, "reflection_black", [109, 144, 192], [109, 144, 192]),
        (6, "reflection_alpha_zero", [129, 154, 242], [129, 154, 242]),
        (7, "worldview_row_x", [129, 154, 242], [129, 154, 242]),
        (10, "normal_half", [42, 61, 156], [42, 61, 156]),
        (11, "normal_tilt", [77, 79, 169], [76, 78, 168]),
        (12, "light_tilt", [161, 185, 255], [211, 235, 255]),
        (13, "eye_offset", [96, 121, 209], [112, 138, 226]),
        (14, "ambient_zero", [126, 129, 156], [126, 129, 156]),
        (15, "ambient_one", [132, 180, 255], [132, 180, 255]),
        (16, "fog", [138, 153, 197], [138, 153, 197]),
        (17, "fog_no_ambient", [136, 143, 173], [136, 143, 173]),
    ];
    for (index, label, center, right) in expected {
        let image = capture(&mut renderer, &fixture(index), BlendMode::Opaque);
        assert_rgb(&image, 32, center, label);
        assert_rgb(&image, 48, right, label);
    }
    assert_rgb(&capture(&mut renderer, &fixture(3), BlendMode::AlphaTest), 32, [0, 0, 0], "base alpha clips");
    assert_rgb(
        &capture(&mut renderer, &fixture(1), BlendMode::AlphaTest),
        32,
        [129, 154, 242],
        "vertex alpha does not clip glazing",
    );
    renderer.set_render_experiments(RenderExperiments::default());
    assert_rgb(&capture(&mut renderer, &fixture(0), BlendMode::Opaque), 32, [32, 128, 255], "legacy profile fallback");
}
