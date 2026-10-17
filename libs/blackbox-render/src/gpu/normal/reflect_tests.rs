//! Independent synthetic pixels measured from the original reflective surface programs.
use blackbox_gfx::{
    BlendMode, DrawRange, FrameParams, Instance, MeshDesc, NormalMaterialDesc, NormalModel, PixelFormat, Projection,
    RenderBackend, RenderExperiments, RgbaImage, Scattering, Shading, SurfaceLight, TextureDesc, TextureHandle, Vertex,
};
use glam::{Mat4, Vec3, Vec4};

use crate::{Renderer, gpu::test_support};

#[path = "reflect_test_data.rs"]
mod data;

struct Fixture {
    textures: [Vec<[u8; 4]>; 4],
    normal: [f32; 3],
    tangent: [f32; 4],
    light: [f32; 3],
    eye: [f32; 3],
    color: [u8; 4],
    ambient: [f32; 3],
    ambient_alpha: f32,
    diffuse: [f32; 3],
    specular: [f32; 3],
    power: f32,
    reflection: f32,
    uv_offset: [f32; 2],
    fog: bool,
    clip_scale: f32,
}

impl Default for Fixture {
    fn default() -> Self {
        Self {
            textures: [
                vec![[64, 128, 192, 128]; 16],
                vec![[128, 128, 255, 255]; 16],
                vec![[128, 64, 32, 255]; 16],
                (0..16).map(|i| [40 + 35 * (i % 4), 20 + 40 * (i / 4), 100, 255]).collect(),
            ],
            normal: [0.0, 0.0, 1.0],
            tangent: [1.0, 0.0, 0.0, 1.0],
            light: [0.0, 0.0, 1.0],
            eye: [0.0, 0.0, 3.0],
            color: [191, 128, 64, 102],
            ambient: [0.2, 0.4, 0.6],
            ambient_alpha: 0.5,
            diffuse: [0.3, 0.2, 0.1],
            specular: [0.3, 0.2, 0.1],
            power: 11.0,
            reflection: 0.25,
            uv_offset: [0.0; 2],
            fog: false,
            clip_scale: 1.0,
        }
    }
}

fn fixture(case: usize) -> Fixture {
    let mut f = Fixture::default();
    match case {
        1 => f.reflection = 0.0,
        2 => {
            f.ambient = [0.0; 3];
            f.ambient_alpha = 0.0;
            f.diffuse = [0.0; 3];
            f.specular = [0.0; 3];
            f.reflection = 0.5;
        }
        3 => f.reflection = 1.0,
        4 => f.color[3] = 0,
        5 => f.color[3] = 255,
        6 | 7 => {
            for pixel in &mut f.textures[0] {
                pixel[3] = match case {
                    6 => 0,
                    _ => 255,
                };
            }
        }
        8 | 9 => {
            for pixel in &mut f.textures[2] {
                pixel[0] = match case {
                    8 => 0,
                    _ => 255,
                };
            }
        }
        10 => f.textures[2] = vec![[0, 255, 0, 255]; 16],
        11 => f.textures[2] = vec![[0, 0, 255, 255]; 16],
        12 => {
            for pixel in &mut f.textures[3] {
                pixel[3] = 0;
            }
        }
        13 => f.ambient_alpha = 0.0,
        14 => f.ambient_alpha = 1.0,
        15 => f.textures[1] = vec![[218, 128, 218, 255]; 16],
        16..=18 => {
            f.textures[1] = vec![[128, 218, 218, 255]; 16];
            if case >= 17 {
                f.light = [0.0, 0.6, 0.8];
            }
            if case == 18 {
                f.tangent[3] = -1.0;
            }
        }
        19 => {
            f.normal[2] = 0.5;
            f.tangent[0] = 2.0;
            f.light = [0.3, 0.4, 0.866_025_4];
            f.textures[1] = vec![[190, 160, 230, 255]; 16];
        }
        20 => f.eye[0] = 2.0,
        21 => f.fog = true,
        22 | 23 => {
            f.power = match case {
                22 => 1.0,
                _ => 32.0,
            };
            f.light = [0.4, 0.0, 0.916_515_1];
        }
        24 => {
            f.diffuse = [3.0, 2.0, 1.0];
            f.specular = [2.0, 3.0, 4.0];
            f.reflection = 2.0;
        }
        25 => {
            f.uv_offset[0] = 0.5;
            for (slot, pixel) in [[32, 192, 64, 255], [218, 128, 218, 255], [255, 0, 128, 255]].into_iter().enumerate()
            {
                for y in 0..4 {
                    f.textures[slot][y * 4 + 2] = pixel;
                }
            }
        }
        26 => f.clip_scale = 2.0,
        28 => f.color[3] = 128,
        _ => {}
    }
    f
}

fn texture(renderer: &mut Renderer, pixels: &[[u8; 4]]) -> TextureHandle {
    let bytes: Vec<u8> = pixels.iter().flatten().copied().collect();
    renderer.create_texture(&TextureDesc {
        label: "synthetic reflective surface texture",
        width: 4,
        height: 4,
        format: PixelFormat::Rgba8,
        mips: vec![&bytes],
    })
}

fn capture(renderer: &mut Renderer, f: &Fixture, blend: BlendMode, supplied_reflection: bool) -> RgbaImage {
    let handles = f.textures.each_ref().map(|pixels| texture(renderer, pixels));
    let material = renderer
        .create_normal_material(&NormalMaterialDesc {
            model: NormalModel::Reflective,
            diffuse: handles[0],
            normal: handles[1],
            specular: handles[2],
            reflection: supplied_reflection.then_some(handles[3]),
            uv_offset: f.uv_offset,
        })
        .expect("reflective surface material");
    let vertices = [[-1.0, -1.0, 1.0], [-1.0, 1.0, 1.0], [1.0, 1.0, 1.0], [1.0, -1.0, 1.0]].map(|position| Vertex {
        position,
        normal: f.normal,
        color_bgra: f.color,
        uv: [0.125, 0.375],
    });
    let tangents = [f.tangent; 4];
    let mesh = renderer.create_mesh(&MeshDesc {
        label: "synthetic projected reflection quad",
        vertices: &vertices,
        secondary_uvs: None,
        tangents: Some(&tangents),
        indices: &[0, 1, 2, 0, 2, 3],
        draws: vec![DrawRange {
            first_index: 0,
            index_count: 6,
            base_vertex: 0,
            texture: Some(handles[0]),
            blend,
            shading: Shading::WorldNormal(material),
        }],
    });
    let frame = FrameParams {
        // Match original integer sample positions while retaining the clip w fixture.
        view: Mat4::from_cols(
            Vec4::X,
            Vec4::Y,
            Vec4::new(0.0, 0.0, 0.0, 0.0),
            Vec4::new(1.0 / 64.0, -1.0 / 64.0, 0.5, 1.0),
        ) * f.clip_scale,
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
            ambient: f.ambient,
            ambient_alpha: f.ambient_alpha,
            diffuse: f.diffuse,
            specular: f.specular,
            specular_power: f.power,
            reflection_scale: f.reflection,
            reflective_specular_scale: 1.0,
        }),
        camera_cut: false,
    };
    renderer.render(&frame, &[Instance::new(mesh, Mat4::IDENTITY)]).unwrap();
    let image = renderer.read_output().expect("headless surface output").unwrap();
    renderer.destroy_mesh(mesh);
    renderer.destroy_normal_material(material);
    for handle in handles {
        renderer.destroy_texture(handle);
    }
    image
}

fn assert_pixels(image: &RgbaImage, expected: [[u8; 3]; 5], label: &str) {
    for ((x, y), expected) in [(32, 32), (48, 32), (16, 32), (32, 16), (32, 48)].into_iter().zip(expected) {
        let actual = image.pixel(x, y).unwrap();
        for channel in 0..3 {
            assert!(
                actual[channel].abs_diff(expected[channel]) <= 2,
                "{label} ({x},{y}): {actual:?}, expected {expected:?}"
            );
        }
    }
}

#[test]
#[ignore = "needs a GPU"]
fn projected_reflective_surfaces_match_independent_original_shader_pixels() {
    let _gpu = test_support::serial();
    let mut renderer = Renderer::headless((64, 64), test_support::options()).expect("selected GPU API");
    renderer.set_render_experiments(RenderExperiments { original_world_materials: true, ..Default::default() });
    // Original programs, synthetic 4x4 textures, LINEAR + Repeat, no sRGB or mip levels.
    for &(index, label, expected) in data::EXPECTED {
        let image = capture(&mut renderer, &fixture(index), BlendMode::Opaque, true);
        assert_pixels(&image, expected, label);
    }
    let zero_scale = data::EXPECTED[1].2;
    assert_pixels(
        &capture(&mut renderer, &fixture(0), BlendMode::Opaque, false),
        zero_scale,
        "missing reflection binds black",
    );
    for index in [4, 6] {
        let expected = data::EXPECTED[index].2;
        for blend in [BlendMode::AlphaBlend, BlendMode::AlphaTest] {
            assert_pixels(
                &capture(&mut renderer, &fixture(index), blend, true),
                expected,
                "modern shader alpha remains one",
            );
        }
    }
}
