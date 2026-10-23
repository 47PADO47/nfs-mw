//! Synthetic pixels checked against the original PC D3D9 shader behavior.

use blackbox_gfx::{
    DrawRange, FrameParams, Instance, MeshDesc, PixelFormat, Projection, Shading, SkyLayerKind, SkyMaterialDesc,
    SkyState, TextureDesc, TextureHandle, Vertex,
};
use glam::{Mat4, Vec3};

use super::super::test_support;
use crate::{BlendMode, Renderer};

fn frame() -> FrameParams {
    FrameParams {
        view: Mat4::IDENTITY,
        projection: Projection::Identity,
        camera_position: Vec3::ZERO,
        light_dir: Vec3::NEG_Z,
        clear_color: [0.0; 3],
        fog: None,
        scattering: None,
        surface_light: None,
        camera_cut: false,

        shadow_tuning: None,
    }
}

fn texture(renderer: &mut Renderer, label: &str, width: u32, rgba: &[u8]) -> TextureHandle {
    renderer.create_texture(&TextureDesc { label, width, height: 1, format: PixelFormat::Rgba8, mips: vec![rgba] })
}

fn mesh(
    renderer: &mut Renderer,
    material: blackbox_gfx::SkyMaterialHandle,
    secondary: [f32; 2],
) -> blackbox_gfx::MeshHandle {
    let vertices = [[-1.0, -1.0, 0.5], [3.0, -1.0, 0.5], [-1.0, 3.0, 0.5]].map(|position| Vertex {
        position,
        normal: [0.0; 3],
        color_bgra: [255; 4],
        uv: [0.25, 0.5],
    });
    renderer.create_mesh(&MeshDesc {
        label: "sky synthetic triangle",
        vertices: &vertices,
        secondary_uvs: Some(&[secondary; 3]),
        tangents: None,
        indices: &[0, 1, 2],
        draws: vec![DrawRange {
            first_index: 0,
            index_count: 3,
            base_vertex: 0,
            texture: None,
            blend: BlendMode::Opaque,
            shading: Shading::SkyLayer(material),
            two_sided: true,
        }],
    })
}

fn pixel(renderer: &mut Renderer, mesh: blackbox_gfx::MeshHandle, state: SkyState, frame: &FrameParams) -> [u8; 4] {
    renderer.set_sky_state(state);
    renderer.render(frame, &[Instance::new(mesh, Mat4::IDENTITY)]).unwrap();
    renderer.read_output().unwrap().unwrap().pixel(16, 16).unwrap()
}

fn close(actual: [u8; 4], expected: [u8; 3]) {
    for channel in 0..3 {
        assert!(actual[channel].abs_diff(expected[channel]) <= 2, "{actual:?} != {expected:?}");
    }
}

#[test]
#[ignore = "needs a GPU"]
fn original_sky_cloud_layers_match_recorded_alpha_channels_and_constant_clamps() {
    let _gpu = test_support::serial();
    let Some(mut renderer) = test_support::headless((32, 32)) else { return };
    let diffuse = texture(&mut renderer, "diffuse fixture", 1, &[64, 128, 192, 128]);
    let miscellaneous = texture(&mut renderer, "misc fixture", 1, &[192, 64, 32, 64]);
    // The original unblended D3D9 fixtures record Cloud2 alpha0/64/128 and
    // Cloud3 alpha0/32/64 for intensity0/.5/1. Standard alpha blend over black
    // gives these independent expected RGB values; intensity2 clamps like1.
    for (kind, expected) in [
        (SkyLayerKind::Cloud2, [[0, 0, 0], [16, 32, 48], [32, 64, 96], [32, 64, 96]]),
        (SkyLayerKind::Cloud3, [[0, 0, 0], [24, 8, 4], [48, 16, 8], [48, 16, 8]]),
    ] {
        let material = renderer.create_sky_material(&SkyMaterialDesc { kind, diffuse, miscellaneous }).unwrap();
        let mesh = mesh(&mut renderer, material, [0.25, 0.5]);
        for (intensity, expected) in [0.0, 0.5, 1.0, 2.0].into_iter().zip(expected) {
            close(
                pixel(&mut renderer, mesh, SkyState { cloud_intensity: [intensity; 4], scroll: 0.0 }, &frame()),
                expected,
            );
        }
        renderer.destroy_mesh(mesh);
        renderer.destroy_sky_material(material);
    }
}

#[test]
#[ignore = "needs a GPU"]
fn sky_preserves_independent_secondary_uvs_and_releases_registered_materials() {
    let _gpu = test_support::serial();
    let Some(mut renderer) = test_support::headless((32, 32)) else { return };
    let diffuse = texture(&mut renderer, "white", 1, &[255; 4]);
    let miscellaneous = texture(&mut renderer, "red green", 2, &[255, 0, 0, 255, 0, 255, 0, 255]);
    let material =
        renderer.create_sky_material(&SkyMaterialDesc { kind: SkyLayerKind::Cloud3, diffuse, miscellaneous }).unwrap();
    let red = mesh(&mut renderer, material, [0.25, 0.5]);
    let green = mesh(&mut renderer, material, [0.75, 0.5]);
    close(pixel(&mut renderer, red, SkyState::default(), &frame()), [255, 0, 0]);
    close(pixel(&mut renderer, green, SkyState::default(), &frame()), [0, 255, 0]);
    renderer.destroy_sky_material(material);
    close(pixel(&mut renderer, green, SkyState::default(), &frame()), [0, 0, 0]);
    renderer.destroy_mesh(red);
    renderer.destroy_mesh(green);
}

#[test]
#[ignore = "needs a GPU"]
fn original_sky_base_matches_independent_physical_scattering_fixtures() {
    let _gpu = test_support::serial();
    let Some(mut renderer) = test_support::headless((32, 32)) else { return };
    let diffuse = texture(&mut renderer, "diffuse fixture", 1, &[64, 128, 192, 128]);
    let miscellaneous = texture(&mut renderer, "misc fixture", 1, &[192, 64, 32, 64]);
    let material =
        renderer.create_sky_material(&SkyMaterialDesc { kind: SkyLayerKind::Base, diffuse, miscellaneous }).unwrap();
    let vertices = [[-1.0, -1.0, 1.0], [1.0, -1.0, 1.0], [1.0, 1.0, 1.0], [-1.0, 1.0, 1.0]].map(|position| Vertex {
        position,
        normal: [0.0; 3],
        color_bgra: [255; 4],
        uv: [0.5, 0.5],
    });
    let mesh = renderer.create_mesh(&MeshDesc {
        label: "physical sky fixture",
        vertices: &vertices,
        secondary_uvs: Some(&[[0.5; 2]; 4]),
        tangents: None,
        indices: &[0, 1, 2, 0, 2, 3],
        draws: vec![DrawRange {
            first_index: 0,
            index_count: 6,
            base_vertex: 0,
            texture: None,
            blend: BlendMode::Opaque,
            shading: Shading::SkyLayer(material),
            two_sided: true,
        }],
    });
    let mut frame = frame();
    frame.view = Mat4::from_scale(Vec3::new(1.0, 1.0, 0.5));
    frame.scattering = Some(blackbox_gfx::Scattering {
        rayleigh: [0.2, 0.3, 0.4],
        mie: [0.1, 0.2, 0.3],
        g: 0.1,
        to_sun: Vec3::Z,
        distance_scale: 1.0,
        in_scatter_scale: 1.0,
    });
    // Original PC vertex+pixel programs, synthetic textures, no recreated shader
    // used to generate these values. Its sky extinction distance is fixed15000.
    for (intensity, expected) in [(0.0, [18, 18, 17]), (0.5, [46, 37, 41]), (1.0, [73, 57, 65]), (2.0, [73, 57, 65])] {
        close(
            pixel(&mut renderer, mesh, SkyState { cloud_intensity: [intensity, 0.0, 0.0, 0.0], scroll: 0.0 }, &frame),
            expected,
        );
    }
    renderer.destroy_mesh(mesh);
    renderer.destroy_sky_material(material);
}
