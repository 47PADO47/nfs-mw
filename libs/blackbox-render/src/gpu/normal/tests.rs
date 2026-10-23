//! Independent original PC normal-map shader pixels, with synthetic textures and geometry.
use blackbox_gfx::{
    BlendMode, DrawRange, FrameParams, Instance, MeshDesc, NormalMaterialDesc, PixelFormat, Projection, RenderBackend,
    RenderExperiments, RgbaImage, Scattering, Shading, SurfaceLight, TextureDesc, Vertex,
};
use glam::{Mat4, Vec3, Vec4};

use crate::{Renderer, gpu::test_support};

struct Fixture {
    normal_tex: [u8; 4],
    specular_tex: [u8; 8],
    normal: [f32; 3],
    tangent: [f32; 4],
    light: [f32; 3],
    eye: [f32; 3],
    ambient_alpha: f32,
    vertex_alpha: u8,
    specular: [f32; 3],
    power: f32,
    fog: bool,
    uv_offset: [f32; 2],
}

impl Default for Fixture {
    fn default() -> Self {
        Self {
            normal_tex: [128, 128, 255, 255],
            specular_tex: [128, 64, 32, 255, 0, 255, 0, 255],
            normal: [0.0, 0.0, 1.0],
            tangent: [1.0, 0.0, 0.0, 1.0],
            light: [0.0, 0.0, 1.0],
            eye: [0.0, 0.0, 3.0],
            ambient_alpha: 0.5,
            vertex_alpha: 102,
            specular: [0.5, 0.25, 0.125],
            power: 11.0,
            fog: false,
            uv_offset: [0.0; 2],
        }
    }
}

fn fixture(case: usize) -> Fixture {
    let mut f = Fixture::default();
    match case {
        1 => f.normal_tex = [218, 128, 218, 255],
        2 => f.normal_tex = [128, 218, 218, 255],
        3 | 4 => {
            f.normal_tex = [128, 218, 218, 255];
            f.light = [0.0, 0.6, 0.8];
            if case == 4 {
                f.tangent[3] = -1.0;
            }
        }
        5 => f.specular = [0.0; 3],
        6 => f.specular_tex = [0; 8],
        7 => f.specular_tex = [255; 8],
        8 | 9 => {
            f.light = [0.4, 0.0, 0.916_515_1];
            f.power = match case {
                8 => 1.0,
                _ => 32.0,
            };
        }
        10 | 11 => {
            f.ambient_alpha = match case {
                10 => 0.0,
                _ => 1.0,
            };
            f.specular = [0.0; 3];
        }
        12 => f.vertex_alpha = 0,
        13 | 14 => {
            f.fog = true;
            if case == 14 {
                f.eye[0] = 2.0;
            }
        }
        15 => f.uv_offset[0] = 0.5,
        16 => f.normal_tex[2] = 0,
        17 => {
            f.normal = [0.0, 0.0, 0.5];
            f.tangent[0] = 2.0;
            f.light = [0.3, 0.4, 0.866_025_4];
            f.normal_tex = [190, 160, 230, 255];
        }
        18 => f.specular = [0.0, 0.0, 1.0],
        19 => f.vertex_alpha = 255,
        20 => {
            f.normal_tex = [160, 145, 248, 255];
            f.light = [0.2, 0.1, 0.974_679_4];
            f.power = 4.0;
        }
        _ => {}
    }
    f
}

fn texture(renderer: &mut Renderer, width: u32, bytes: &[u8]) -> blackbox_gfx::TextureHandle {
    renderer.create_texture(&TextureDesc {
        label: "synthetic normal-map fixture",
        width,
        height: 1,
        format: PixelFormat::Rgba8,
        mips: vec![bytes],
    })
}

fn capture(renderer: &mut Renderer, f: &Fixture, blend: BlendMode) -> RgbaImage {
    let diffuse = texture(renderer, 1, &[64, 128, 192, 128]);
    let normal = texture(renderer, 1, &f.normal_tex);
    let specular = texture(renderer, 2, &f.specular_tex);
    let material = renderer
        .create_normal_material(&NormalMaterialDesc {
            model: blackbox_gfx::NormalModel::Bump,
            reflection: None,
            diffuse,
            normal,
            specular,
            uv_offset: f.uv_offset,
        })
        .expect("native normal-mapped materials");
    let vertices = [[-1.0, -1.0, 1.0], [-1.0, 1.0, 1.0], [1.0, 1.0, 1.0], [1.0, -1.0, 1.0]].map(|position| Vertex {
        position,
        normal: f.normal,
        color_bgra: [191, 128, 64, f.vertex_alpha],
        uv: [0.25, 0.5],
    });
    let tangents = [f.tangent; 4];
    let mesh = renderer.create_mesh(&MeshDesc {
        label: "synthetic original normal-map quad",
        vertices: &vertices,
        secondary_uvs: None,
        tangents: Some(&tangents),
        indices: &[0, 1, 2, 0, 2, 3],
        draws: vec![DrawRange {
            first_index: 0,
            index_count: 6,
            base_vertex: 0,
            texture: Some(diffuse),
            blend,
            shading: Shading::WorldNormal(material),
            two_sided: true,
        }],
    });
    let scattering = f.fog.then_some(Scattering {
        rayleigh: [0.2, 0.3, 0.4],
        mie: [0.1, 0.2, 0.3],
        g: 0.1,
        to_sun: Vec3::Z,
        distance_scale: 0.75,
        in_scatter_scale: 1.0,
    });
    let frame = FrameParams {
        // D3D9 samples integer pixel centers. Shift this 64px fixture by half a pixel
        // so (32,32)/(48,32) measure exactly the original local x=0/0.5, y=0.
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
        scattering,
        surface_light: Some(SurfaceLight {
            shadow_strength: 1.0,
            ambient: [0.2, 0.4, 0.6],
            ambient_alpha: f.ambient_alpha,
            diffuse: [0.3, 0.2, 0.1],
            specular: f.specular,
            specular_power: f.power,
            reflection_scale: 0.0,
            reflective_specular_scale: 1.0,
        }),
        camera_cut: false,

        shadow_tuning: None,
    };
    renderer.render(&frame, &[Instance::new(mesh, Mat4::IDENTITY)]).unwrap();
    let image = renderer.read_output().expect("headless output").unwrap();
    renderer.destroy_mesh(mesh);
    renderer.destroy_normal_material(material);
    for texture in [diffuse, normal, specular] {
        renderer.destroy_texture(texture);
    }
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
fn authored_normal_mapping_matches_independent_pc_shader_pixels() {
    let _gpu = test_support::serial();
    let mut renderer = Renderer::headless((64, 64), test_support::options()).expect("selected GPU API");
    renderer.set_render_experiments(RenderExperiments { original_world_materials: true, ..Default::default() });
    // These numbers came from executing the original shader pair, not the WGSL implementation.
    let expected = [
        ("flat", [170, 109, 133], [131, 99, 130]),
        ("tilt x", [30, 62, 113], [30, 62, 113]),
        ("tilt y", [30, 62, 113], [30, 62, 113]),
        ("positive handedness", [42, 76, 124], [42, 76, 124]),
        ("negative handedness", [9, 33, 92], [9, 33, 92]),
        ("zero specular colour", [42, 77, 125], [42, 77, 125]),
        ("black specular texture", [42, 77, 125], [42, 77, 125]),
        ("white specular texture", [255, 204, 188], [220, 166, 169]),
        ("power one", [156, 102, 129], [165, 104, 129]),
        ("power 32", [47, 75, 122], [124, 94, 127]),
        ("ambient alpha zero", [38, 51, 38], [38, 51, 38]),
        ("ambient alpha one", [45, 103, 211], [45, 103, 211]),
        ("zero vertex alpha", [170, 109, 133], [131, 99, 130]),
        ("fog", [108, 58, 54], [86, 54, 53]),
        ("fog and eye offset", [34, 40, 43], [42, 44, 47]),
        ("UV offset selects specular texture", [42, 204, 125], [42, 166, 125]),
        ("reverse normal retains unclamped reflection dot", [131, 58, 94], [97, 49, 92]),
        ("nonunit basis", [141, 98, 127], [38, 72, 121]),
        ("blue specular colour", [42, 77, 189], [42, 77, 170]),
        ("full vertex alpha", [170, 109, 133], [131, 99, 130]),
        ("tilted highlight", [138, 101, 131], [102, 92, 128]),
    ];
    for (index, (label, center, right)) in expected.into_iter().enumerate() {
        let image = capture(&mut renderer, &fixture(index), BlendMode::Opaque);
        assert_rgb(&image, 32, center, label);
        assert_rgb(&image, 48, right, label);
    }
    let image = capture(&mut renderer, &fixture(12), BlendMode::AlphaTest);
    assert_rgb(&image, 32, [0, 0, 0], "vertex alpha controls cutout");
    let image = capture(&mut renderer, &fixture(19), BlendMode::AlphaTest);
    assert_rgb(&image, 32, [170, 109, 133], "opaque vertex alpha preserves textured cutout");
    renderer.set_render_experiments(RenderExperiments::default());
    let image = capture(&mut renderer, &fixture(0), BlendMode::Opaque);
    assert_rgb(&image, 32, [32, 128, 255], "legacy switch restores prelit fallback");
}
