//! Synthetic receiver geometry, alpha, mirrored winding and source-plane clipping.
use super::{Renderer, test_support};
use blackbox_gfx::*;
use glam::{Mat4, Vec3};

fn frame(mirrored: bool) -> FrameParams {
    let z = match mirrored {
        true => -0.25,
        false => 0.25,
    };
    FrameParams {
        view: Mat4::from_translation(Vec3::Z * 0.5) * Mat4::from_scale(Vec3::new(1.0, 1.0, z)),
        projection: Projection::Identity,
        camera_position: Vec3::Z,
        light_dir: Vec3::NEG_Z,
        clear_color: [0.0, 0.2, 0.0],
        fog: None,
        scattering: None,
        surface_light: None,
        camera_cut: false,
        shadow_tuning: None,
    }
}
fn texture(r: &mut Renderer, rgba: [u8; 4]) -> TextureHandle {
    r.create_texture(&TextureDesc {
        label: "planar synthetic texel",
        width: 1,
        height: 1,
        format: PixelFormat::Rgba8,
        mips: vec![&rgba],
    })
}
fn mesh(r: &mut Renderer, points: &[[f32; 3]], texture: TextureHandle, blend: BlendMode, sided: bool) -> MeshHandle {
    let vertices: Vec<_> = points
        .iter()
        .map(|&position| Vertex { position, normal: [0.0, 0.0, 1.0], color_bgra: [128, 128, 128, 255], uv: [0.5; 2] })
        .collect();
    r.create_mesh(&MeshDesc {
        label: "planar synthetic triangle",
        vertices: &vertices,
        indices: &[0, 1, 2],
        tangents: None,
        secondary_uvs: None,
        draws: vec![DrawRange {
            first_index: 0,
            index_count: 3,
            base_vertex: 0,
            texture: Some(texture),
            blend,
            shading: Shading::Prelit,
            two_sided: sided,
        }],
    })
}
fn pixel(image: &RgbaImage, xy: [u32; 2], expected: [u8; 3]) {
    let value = image.pixel(xy[0], xy[1]).unwrap();
    assert!((0..3).all(|c| value[c].abs_diff(expected[c]) <= 4), "{xy:?}: {value:?}, expected {expected:?}");
}
fn capture(r: &mut Renderer, parts: &[Instance], receivers: &[TextureHandle]) {
    r.set_planar_reflection_scene(Some(PlanarReflectionScene {
        frame: frame(true),
        size: [64; 2],
        instances: parts,
        receivers,
        clip_plane: [0.0, 0.0, 1.0, 0.0],
    }));
}

#[test]
#[ignore = "requires selected GPU"]
fn subtractive_decal_preserves_black_and_darkens_bright_pixels() {
    let _gpu = test_support::serial();
    let mut r = Renderer::headless((64, 64), test_support::options()).expect("selected GPU required");
    let triangle = [[-1.0, -1.0, 0.0], [-1.0, 3.0, 0.0], [3.0, -1.0, 0.0]];
    for (shade, expected) in [(0, 51), (128, 25), (255, 0)] {
        let tex = texture(&mut r, [shade, shade, shade, 255]);
        let decal = mesh(&mut r, &triangle, tex, BlendMode::Subtractive, true);
        r.render(&frame(false), &[Instance::new(decal, Mat4::IDENTITY)]).unwrap();
        pixel(&r.read_output().unwrap().unwrap(), [32, 32], [0, expected, 0]);
    }
}

#[test]
#[ignore = "requires selected GPU"]
fn planar_capture_preserves_floor_alpha_geometry_and_empty_background() {
    let _gpu = test_support::serial();
    let mut r = Renderer::headless((64, 64), test_support::options()).expect("selected GPU required");
    let floor = texture(&mut r, [0, 0, 128, 128]);
    let red = texture(&mut r, [200, 0, 0, 255]);
    let triangle = [[-0.8, -0.8, 0.0], [-0.8, 0.8, 0.0], [0.8, 0.8, 0.0]];
    let receiver = mesh(&mut r, &triangle, floor, BlendMode::AlphaBlend, true);
    let source = mesh(&mut r, &[[-1.0, -1.0, 0.5], [-1.0, 3.0, 0.5], [3.0, -1.0, 0.5]], red, BlendMode::Opaque, true);
    let actual = [Instance::new(receiver, Mat4::IDENTITY)];
    capture(&mut r, &[Instance::new(source, Mat4::IDENTITY)], &[floor]);
    r.render(&frame(false), &actual).unwrap();
    let image = r.read_output().unwrap().unwrap();
    pixel(&image, [16, 16], [100, 0, 64]);
    pixel(&image, [48, 48], [0, 51, 0]);
    r.set_texture_filtering(TextureFiltering::Trilinear);
    r.render(&frame(false), &actual).unwrap();
    pixel(&r.read_output().unwrap().unwrap(), [16, 16], [100, 0, 64]);
    capture(&mut r, &[], &[floor]);
    r.render(&frame(false), &actual).unwrap();
    pixel(&r.read_output().unwrap().unwrap(), [16, 16], [0, 25, 64]);
    let glass = texture(&mut r, [200, 0, 0, 128]);
    let translucent =
        mesh(&mut r, &[[-1.0, -1.0, 0.5], [-1.0, 3.0, 0.5], [3.0, -1.0, 0.5]], glass, BlendMode::AlphaBlend, true);
    capture(&mut r, &[Instance::new(translucent, Mat4::IDENTITY)], &[floor]);
    r.render(&frame(false), &actual).unwrap();
    pixel(&r.read_output().unwrap().unwrap(), [16, 16], [50, 13, 64]);
    let solid_floor = texture(&mut r, [0, 0, 128, 255]);
    let opaque_receiver = mesh(&mut r, &triangle, solid_floor, BlendMode::AlphaBlend, true);
    capture(&mut r, &[Instance::new(source, Mat4::IDENTITY)], &[solid_floor]);
    r.render(&frame(false), &[Instance::new(opaque_receiver, Mat4::IDENTITY)]).unwrap();
    pixel(&r.read_output().unwrap().unwrap(), [16, 16], [0, 0, 128]);
    r.set_planar_reflection_scene(None);
    r.render(&frame(false), &actual).unwrap();
    pixel(&r.read_output().unwrap().unwrap(), [16, 16], [0, 25, 64]);
}

#[test]
#[ignore = "requires selected GPU"]
fn planar_capture_clips_source_world_and_reverses_winding_once() {
    let _gpu = test_support::serial();
    let mut r = Renderer::headless((64, 64), test_support::options()).expect("selected GPU required");
    let red = texture(&mut r, [200, 0, 0, 255]);
    let source = mesh(&mut r, &[[-0.8, -0.8, 0.5], [-0.8, 0.8, 0.5], [0.8, 0.8, 0.5]], red, BlendMode::Opaque, false);
    capture(&mut r, &[Instance::new(source, Mat4::IDENTITY)], &[]);
    pixel(&r.read_projected_scene().unwrap().unwrap(), [16, 16], [200, 0, 0]);
    capture(&mut r, &[Instance::new(source, Mat4::from_translation(Vec3::NEG_Z))], &[]);
    let image = r.read_projected_scene().unwrap().unwrap();
    assert!(
        image.rgba.as_chunks::<4>().0.iter().all(|p| *p == [0, 0, 0, 0]),
        "clipped geometry must leave transparent pixels"
    );
}
