//! Independent original-shader pixels with synthetic material ramps, lights and cube faces.
//! The behavior contract is documented in `docs/specs/car-material-shading.md`.
use blackbox_gfx::{
    BlendMode, DirectionalLight, DrawRange, Fog, FrameParams, GlossyMaterial, GlossyMaterialHandle, GlossyModel,
    Instance, LightingRig, MeshDesc, MeshHandle, PixelFormat, Projection, RenderBackend, RenderExperiments, RgbaImage,
    Shading, TextureDesc, TextureHandle, Vertex,
};
use glam::{Mat4, Quat, Vec3, Vec4};

use crate::{Renderer, gpu::test_support};

#[path = "original_test_data.rs"]
mod data;
#[path = "original_hdr_tests.rs"]
mod hdr;

const FACES: [[u8; 4]; 6] = [
    [200, 20, 30, 255],
    [40, 170, 60, 255],
    [20, 80, 220, 255],
    [180, 140, 20, 255],
    [110, 40, 160, 255],
    [30, 180, 190, 255],
];

struct Fixture {
    material: GlossyMaterial,
    rig: LightingRig,
    eye: Vec3,
    normal: [f32; 3],
    color_bgra: [u8; 4],
    diffuse: [u8; 4],
    faces: [[u8; 4]; 6],
    varying_normal: bool,
    model: Mat4,
}

impl Fixture {
    fn new(model: GlossyModel) -> Self {
        Self {
            material: GlossyMaterial {
                background_actor: false,
                model,
                surface: None,
                diffuse_min: [0.1, 0.12, 0.14, 0.7],
                diffuse_range: [0.4, 0.35, 0.3, 0.2],
                specular_min: [0.02, 0.03, 0.04],
                specular_range: [0.3, 0.2, 0.1],
                specular_power: 8.0,
                envmap_min: 0.15,
                envmap_range: 0.5,
                envmap_power: 2.0,
            },
            rig: LightingRig {
                lights: [
                    DirectionalLight::new(Vec3::Z, Vec3::new(0.8, 0.3, 0.15)),
                    DirectionalLight::new(Vec3::new(0.0, 0.8, 0.6), Vec3::new(0.1, 0.5, 0.2)),
                    DirectionalLight::new(Vec3::new(0.8, 0.0, 0.6), Vec3::new(0.15, 0.2, 0.7)),
                ],
                ambient: Vec3::ZERO,
                material_specular_scale: 1.0,
            },
            eye: Vec3::new(0.0, 0.0, 3.0),
            normal: [0.0, 0.0, 1.0],
            color_bgra: [64, 96, 128, 102],
            diffuse: [64, 128, 192, 128],
            faces: FACES,
            varying_normal: false,
            model: Mat4::IDENTITY,
        }
    }

    fn case(index: usize, model: GlossyModel) -> Self {
        let mut f = Self::new(model);
        match index {
            1 => {
                f.material.diffuse_min = [0.4, 0.35, 0.3, 0.8];
                f.material.diffuse_range = [0.0; 4];
                f.material.specular_min = [0.1, 0.08, 0.06];
                f.material.specular_range = [0.0; 3];
                f.material.envmap_min = 0.0;
                f.material.envmap_range = 0.0;
            }
            2 => {
                f.material.diffuse_min = [0.05, 0.05, 0.06, 0.25];
                f.material.diffuse_range = [0.02, 0.02, 0.02, 0.1];
                f.material.specular_min = [0.04, 0.05, 0.06];
                f.material.specular_range = [0.2; 3];
                f.material.envmap_min = 0.5;
                f.material.envmap_range = 0.5;
                f.material.envmap_power = 1.0;
            }
            3 => {
                f.material.specular_min = [0.0; 3];
                f.material.specular_range = [0.0; 3];
                f.material.envmap_min = 0.0;
                f.material.envmap_range = 0.0;
            }
            4 => {
                f.material.diffuse_min = [0.0, 0.0, 0.0, 1.0];
                f.material.diffuse_range = [0.0; 4];
                f.material.specular_min = [0.2, 0.3, 0.4];
                f.material.specular_range = [0.1; 3];
                f.material.envmap_min = 0.0;
                f.material.envmap_range = 0.0;
            }
            5 | 27 => {
                let alpha = match index {
                    27 => 0.3,
                    _ => 1.0,
                };
                f.material.diffuse_min = [0.0, 0.0, 0.0, alpha];
                f.material.diffuse_range = [0.0; 4];
                f.material.specular_min = [0.0; 3];
                f.material.specular_range = [0.0; 3];
                f.material.envmap_min = 1.0;
                f.material.envmap_range = 0.0;
            }
            6 => f.color_bgra[2] = 0,
            7 => f.color_bgra[2] = 64,
            8 => f.color_bgra[3] = 0,
            9 => f.normal = [0.6, 0.0, 0.8],
            10 => f.normal = [0.0, 0.0, -1.0],
            11 => f.normal = [0.0, 0.0, 0.5],
            12 => f.eye.x = 2.0,
            13 => {
                f.material.diffuse_min = [3.0, 3.0, 3.0, 2.0];
                f.material.specular_min = [3.0; 3];
                f.material.envmap_min = 3.0;
            }
            14 => f.material.specular_power = 1.0,
            15 => f.material.specular_power = 16.0,
            16 => f.material.envmap_power = 1.0,
            17 => f.material.envmap_power = 8.0,
            18 => {
                f.rig.lights = [
                    DirectionalLight::new(Vec3::new(0.6, 0.0, 0.8), Vec3::new(0.1, 0.8, 0.2)),
                    DirectionalLight::new(Vec3::new(0.0, -0.8, 0.6), Vec3::new(0.7, 0.2, 0.1)),
                    DirectionalLight::new(Vec3::new(-0.8, 0.0, 0.6), Vec3::new(0.2, 0.3, 0.9)),
                ]
            }
            // Convert the original view-aligned face colors into this renderer's world-aligned cube.
            19 => f.faces = [FACES[5], FACES[4], FACES[2], FACES[3], FACES[0], FACES[1]],
            20 => f.diffuse[3] = 0,
            21 => f.diffuse = [255; 4],
            22 => f.varying_normal = true,
            29 => {
                f.color_bgra[0] = 0;
                f.color_bgra[1] = 0;
            }
            30 => {
                f.model = Mat4::from_scale_rotation_translation(
                    Vec3::new(2.0, 0.5, 1.25),
                    Quat::from_rotation_y(std::f32::consts::PI / 6.0),
                    Vec3::new(0.25, -0.5, 0.75),
                )
            }
            31 => {
                f.model = Mat4::from_scale_rotation_translation(
                    Vec3::new(-1.5, 0.75, 1.0),
                    Quat::from_rotation_z(-std::f32::consts::FRAC_PI_4),
                    Vec3::new(-0.25, 0.25, 0.1),
                )
            }
            32 => {
                f.model =
                    Mat4::from_scale_rotation_translation(Vec3::splat(1.75), Quat::IDENTITY, Vec3::new(0.4, 0.2, -0.1))
            }
            _ => {}
        }
        f
    }
}

struct Scene {
    mesh: MeshHandle,
    material: GlossyMaterialHandle,
    texture: TextureHandle,
    frame: FrameParams,
    model: Mat4,
}

impl Scene {
    fn new(renderer: &mut Renderer, fixture: &Fixture, blend: BlendMode) -> Self {
        let texture = renderer.create_texture(&TextureDesc {
            label: "synthetic original car diffuse",
            width: 1,
            height: 1,
            format: PixelFormat::Rgba8,
            mips: vec![&fixture.diffuse],
        });
        let material = renderer.create_glossy_material(&fixture.material);
        let vertices = [[-1.0, -1.0, 1.0], [-1.0, 1.0, 1.0], [1.0, 1.0, 1.0], [1.0, -1.0, 1.0]].map(|position| {
            let normal = match fixture.varying_normal {
                true => [0.2 * position[0], 0.0, 0.979_795_9],
                false => fixture.normal,
            };
            Vertex { position, normal, color_bgra: fixture.color_bgra, uv: [0.5; 2] }
        });
        let mesh = renderer.create_mesh(&MeshDesc {
            label: "synthetic original car quad",
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
                shading: Shading::Glossy(material),
                two_sided: true,
            }],
        });
        let frame = FrameParams {
            // Align WGPU half-pixel centers with the independent D3D9 fixture's integer centers.
            // The inverse below cancels only projected geometry coverage. Camera/lights stay
            // in world space; shading must apply the original model-transpose input transform.
            view: Mat4::from_cols(
                Vec4::X,
                Vec4::Y,
                Vec4::new(0.0, 0.0, 0.5, 0.0),
                Vec4::new(1.0 / 64.0, -1.0 / 64.0, 0.0, 1.0),
            ) * fixture.model.inverse(),
            projection: Projection::Identity,
            camera_position: fixture.eye,
            light_dir: Vec3::NEG_Z,
            clear_color: [0.0; 3],
            fog: None,
            scattering: None,
            surface_light: None,
            camera_cut: false,

            shadow_tuning: None,
        };
        Self { mesh, material, texture, frame, model: fixture.model }
    }

    fn capture(&self, renderer: &mut Renderer, fixture: &Fixture) -> RgbaImage {
        renderer.set_environment_faces(1, fixture.faces.each_ref().map(<[u8; 4]>::as_slice));
        renderer.set_lighting_rig(&fixture.rig);
        renderer.render(&self.frame, &[Instance::new(self.mesh, self.model)]).unwrap();
        renderer.read_output().expect("headless output").unwrap()
    }

    fn release(self, renderer: &mut Renderer) {
        renderer.destroy_mesh(self.mesh);
        renderer.destroy_glossy_material(self.material);
        renderer.destroy_texture(self.texture);
    }
}

fn renderer() -> Renderer {
    Renderer::headless((64, 64), test_support::options()).expect("the selected GPU API must be available")
}

fn assert_rgb(image: &RgbaImage, x: u32, expected: [u8; 3], label: &str) {
    let actual = image.pixel(x, 32).unwrap();
    for (channel, expected) in expected.into_iter().enumerate() {
        assert!(
            actual[channel].abs_diff(expected) <= 3,
            "{label} x={x}: {actual:?}; expected channel {channel}={expected}"
        );
    }
}

fn assert_alpha(image: &RgbaImage, x: u32, expected: u8, label: &str) {
    let actual = image.pixel(x, 32).unwrap()[3];
    assert!(actual.abs_diff(expected) <= 1, "{label} x={x}: alpha={actual}, expected {expected}");
}

#[test]
#[ignore = "needs a GPU"]
fn original_car_families_match_independent_material_and_interpolation_pixels() {
    let _gpu = test_support::serial();
    let mut renderer = renderer();
    for (model, expected) in [(GlossyModel::Shadowed, data::MODERN), (GlossyModel::VertexLit, data::VERTEX_LIT)] {
        for (index, name, center, right) in expected {
            let fixture = Fixture::case(index, model);
            let scene = Scene::new(&mut renderer, &fixture, BlendMode::Opaque);
            let image = scene.capture(&mut renderer, &fixture);
            assert_rgb(&image, 32, center, name);
            assert_rgb(&image, 48, right, name);
            // Presented SDR targets/captures intentionally remain opaque. Source shader
            // alpha is verified separately in the HDR scene intermediate.
            assert_alpha(&image, 32, 255, name);
            assert_alpha(&image, 48, 255, name);
            scene.release(&mut renderer);
        }
    }
}

#[test]
#[ignore = "needs a GPU"]
fn original_car_alpha_blending_and_cutout_follow_material_and_texture_channels() {
    let _gpu = test_support::serial();
    let mut renderer = renderer();
    for (model, paint, glass) in
        [(GlossyModel::Shadowed, [22, 22, 40], [3, 2, 6]), (GlossyModel::VertexLit, [41, 34, 61], [8, 5, 12])]
    {
        for (index, expected) in [(0, paint), (2, glass), (20, [0, 0, 0])] {
            let fixture = Fixture::case(index, model);
            let scene = Scene::new(&mut renderer, &fixture, BlendMode::AlphaBlend);
            assert_rgb(&scene.capture(&mut renderer, &fixture), 32, expected, "gamma-space source-alpha composition");
            scene.release(&mut renderer);
        }
        let white = match model {
            GlossyModel::Shadowed => data::MODERN[21].2,
            _ => data::VERTEX_LIT[21].2,
        };
        for (index, expected) in [(0, [0, 0, 0]), (20, [0, 0, 0]), (21, white)] {
            let fixture = Fixture::case(index, model);
            let scene = Scene::new(&mut renderer, &fixture, BlendMode::AlphaTest);
            let image = scene.capture(&mut renderer, &fixture);
            assert_rgb(&image, 32, expected, "cutout tests the original shader's composed alpha");
            assert_alpha(&image, 32, 255, "published SDR cutout output remains opaque");
            scene.release(&mut renderer);
        }
    }
}

#[test]
#[ignore = "needs a GPU"]
fn original_car_rig_scale_updates_retained_materials_and_custom_keeps_its_behavior() {
    let _gpu = test_support::serial();
    let mut renderer = renderer();
    for (model, full) in [(GlossyModel::Shadowed, [4, 6, 8]), (GlossyModel::VertexLit, [76, 111, 146])] {
        let mut fixture = Fixture::case(4, model);
        let mut scene = Scene::new(&mut renderer, &fixture, BlendMode::Opaque);
        for scale in [0.0, 0.5, 1.0, 0.0] {
            fixture.rig.material_specular_scale = scale;
            let expected = full.map(|v| (f32::from(v) * scale).round() as u8);
            assert_rgb(
                &scene.capture(&mut renderer, &fixture),
                32,
                expected,
                "time-of-day specular scale updates existing material",
            );
        }
        fixture.rig.material_specular_scale = 1.0;
        fixture.rig.ambient = Vec3::splat(100.0);
        scene.frame.fog = Some(Fog { start: 0.0, end: 0.1 });
        renderer.set_render_experiments(RenderExperiments {
            clearcoat: true,
            car_skylight: true,
            ..Default::default()
        });
        assert_rgb(
            &scene.capture(&mut renderer, &fixture),
            32,
            full,
            "original family ignores custom ambient, coat, sky and atmospheric fog",
        );
        scene.release(&mut renderer);
        renderer.set_render_experiments(RenderExperiments::default());
    }
    let mut fixture = Fixture::case(4, GlossyModel::Custom);
    let mut scene = Scene::new(&mut renderer, &fixture, BlendMode::Opaque);
    fixture.rig.material_specular_scale = 0.0;
    let first = scene.capture(&mut renderer, &fixture);
    fixture.rig.material_specular_scale = 1.0;
    let second = scene.capture(&mut renderer, &fixture);
    assert_eq!(first.rgba, second.rgba, "new original-material scale must not change Custom");
    assert_alpha(&second, 32, 255, "Custom keeps opaque target alpha");
    scene.frame.fog = Some(Fog { start: 0.0, end: 0.1 });
    let fogged = scene.capture(&mut renderer, &fixture);
    assert_ne!(first.rgba, fogged.rgba, "Custom keeps its existing atmospheric fog");
    assert_rgb(&fogged, 32, [0, 0, 0], "Custom fog reaches the clear colour");
    scene.release(&mut renderer);
    let fixture = Fixture::case(0, GlossyModel::Custom);
    let scene = Scene::new(&mut renderer, &fixture, BlendMode::AlphaTest);
    let image = scene.capture(&mut renderer, &fixture);
    assert_alpha(&image, 32, 255, "Custom keeps cutout target alpha");
    assert!(
        image.pixel(32, 32).unwrap()[..3].iter().any(|v| *v > 0),
        "Custom retains texture-only cutout; Original rejects this composed opacity"
    );
    scene.release(&mut renderer);
}
