//! End-to-end captures: off-screen scenery reaches the native glossy shader.
use crate::{Renderer, RendererOptions};
use blackbox_gfx::reflection::ALL_FACES;
use blackbox_gfx::*;
use blackbox_gfx_testkit::mesh::{MeshBuilder, prelit};
use blackbox_gpu_passes::test_support::serial;
use glam::{Mat4, Quat, Vec3};

const SIZE: [u32; 2] = [64, 64];

fn frame(axis: Vec3) -> FrameParams {
    let up = match axis.z.abs() > 0.5 {
        true => Vec3::Y,
        false => Vec3::Z,
    };
    FrameParams {
        view: glam::camera::rh::view::look_at_mat4(axis * 4.0, Vec3::ZERO, up),
        projection: Projection::PerspectiveInfiniteReverse { fov_y: 1.0, aspect: 1.0, near: 0.1 },
        camera_position: axis * 4.0,
        light_dir: Vec3::Z,
        fog: None,
        scattering: None,
        surface_light: None,
        clear_color: [0.0; 3],
        camera_cut: true,
        shadow_tuning: None,
    }
}

fn capture(backend: &mut dyn RenderBackend, frame: &FrameParams, instances: &[Instance]) -> RgbaImage {
    let id = backend.request_capture(SIZE, frame, instances).unwrap();
    for _ in 0..100 {
        backend.render(frame, instances).unwrap();
        if let Some(image) = backend.poll_capture(id) {
            return image.unwrap();
        }
    }
    panic!("reflection capture timed out");
}

fn centre(image: &RgbaImage) -> [u8; 3] {
    let start = ((image.height / 2 * image.width + image.width / 2) * 4) as usize;
    image.rgba[start..start + 3].try_into().unwrap()
}

fn exercise(backend: &mut dyn RenderBackend) -> Vec<[u8; 3]> {
    let material = backend.create_glossy_material(&GlossyMaterial {
        diffuse_min: [0.0, 0.0, 0.0, 1.0],
        diffuse_range: [0.0; 4],
        specular_min: [0.0; 3],
        specular_range: [0.0; 3],
        envmap_min: 2.0,
        envmap_range: 0.0,
        ..Default::default()
    });
    let black = [0, 0, 0, 255];
    backend.set_environment(Environment::Faces { size: 1, faces: [&black; 6] });
    let mut mirror = MeshBuilder::new();
    mirror.state(None, BlendMode::Opaque, Shading::Glossy(material));
    mirror.card(Vec3::new(0.0, 0.0, -1.0), 2.0, 2.0, 0.0, prelit([1.0; 3], 1.0));
    let mirror = mirror.finish().upload(backend, "reflection test mirror");
    let axes = [Vec3::X, Vec3::NEG_X, Vec3::Y, Vec3::NEG_Y, Vec3::Z, Vec3::NEG_Z];
    let colours =
        [[0.75, 0.15, 0.2], [0.15, 0.65, 0.25], [0.2, 0.3, 0.8], [0.7, 0.55, 0.15], [0.5, 0.2, 0.7], [0.15, 0.6, 0.65]];
    let mut walls = Vec::new();
    for (i, axis) in axes.into_iter().enumerate() {
        let mut wall = MeshBuilder::new();
        wall.state(None, BlendMode::Opaque, Shading::Prelit);
        wall.card(Vec3::new(0.0, -8.0, -8.0), 16.0, 16.0, 0.0, prelit(colours[i], 1.0));
        let mesh = wall.finish().upload(backend, "reflection test offscreen wall");
        let rotation = Quat::from_rotation_arc(Vec3::NEG_Y, axis);
        walls.push(Instance::keyed(mesh, Mat4::from_quat(rotation), InstanceKey::new(2, i as u32)));
    }
    let mut samples = Vec::new();
    for (i, axis) in axes.into_iter().enumerate() {
        let frame = frame(axis);
        let main = [Instance::keyed(
            mirror,
            Mat4::from_quat(Quat::from_rotation_arc(Vec3::NEG_Y, axis)),
            InstanceKey::new(1, 0),
        )];
        // The wall is behind the main camera; only the mirror is handed to the player view.
        // Repeated updates also cover the asset/pipeline warm-up of a freshly enabled probe.
        for _ in 0..8 {
            backend.set_reflection_scene(Some(ReflectionScene {
                origin: Vec3::ZERO,
                quality: ReflectionQuality::Low,
                frame,
                faces: [&walls; 6],
                refresh: ALL_FACES,
            }));
            backend.render(&frame, &main).unwrap();
        }
        let sample = centre(&capture(backend, &frame, &main));
        for (channel, expected) in colours[i].into_iter().enumerate() {
            assert!(
                sample[channel].abs_diff((expected * 255.0) as u8) < 8,
                "{} face {i}: {sample:?}",
                backend.info().renderer
            );
        }
        samples.push(sample);
    }
    let frame = frame(Vec3::NEG_Y);
    let main = [Instance::keyed(mirror, Mat4::IDENTITY, InstanceKey::new(1, 0))];
    backend.set_reflection_scene(Some(ReflectionScene {
        origin: Vec3::ZERO,
        quality: ReflectionQuality::High,
        frame,
        faces: [&walls; 6],
        refresh: ALL_FACES,
    }));
    for _ in 0..6 {
        backend.render(&frame, &main).unwrap();
    }
    assert!(centre(&capture(backend, &frame, &main))[0] > 100, "quality switch preserves capture");
    backend.set_reflection_scene(Some(ReflectionScene {
        origin: Vec3::new(40.0, 40.0, 40.0),
        quality: ReflectionQuality::High,
        frame,
        faces: [&walls; 6],
        refresh: ALL_FACES,
    }));
    for _ in 0..6 {
        backend.render(&frame, &main).unwrap();
    }
    assert!(centre(&capture(backend, &frame, &main)).into_iter().all(|c| c < 8), "probe follows its origin");
    backend.set_reflection_scene(None);
    assert!(centre(&capture(backend, &frame, &main)).into_iter().all(|c| c < 4), "off restores static environment");
    // Asymmetric scenery catches a vertically mirrored cube; a bright HUD must not enter it.
    let mut wall = MeshBuilder::new();
    wall.state(None, BlendMode::Opaque, Shading::Prelit);
    wall.card(Vec3::new(0.0, -8.0, 0.0), 16.0, 8.0, 0.0, prelit([0.8, 0.05, 0.05], 1.0));
    wall.card(Vec3::new(0.0, -8.0, -8.0), 16.0, 8.0, 0.0, prelit([0.05, 0.05, 0.8], 1.0));
    let wall = wall.finish().upload(backend, "asymmetric reflection wall");
    let walls = [Instance::keyed(wall, Mat4::IDENTITY, InstanceKey::new(3, 0))];
    // The player view and probe may pick different LOD meshes of the same world object. Its main-view
    // green mesh must not replace the red/blue probe mesh when the entity pool combines their keys.
    let mut player_lod = MeshBuilder::new();
    player_lod.state(None, BlendMode::Opaque, Shading::Prelit);
    player_lod.card(Vec3::new(0.0, -8.0, -8.0), 16.0, 16.0, 0.0, prelit([0.05, 0.8, 0.05], 1.0));
    let player_lod = player_lod.finish().upload(backend, "player LOD of reflection wall");
    let main = [main[0], Instance::keyed(player_lod, Mat4::IDENTITY, InstanceKey::new(3, 0))];
    let ui = UiTextureId::from_raw(1234);
    backend.update_ui_texture(&UiTexturePatch { id: ui, offset: None, size: [1, 1], rgba: &[255; 4] });
    backend.set_ui_layer(UiLayer {
        compositions: Vec::new(),
        pixels_per_point: 1.0,
        meshes: vec![UiMesh {
            vertices: [[0.0, 0.0], [64.0, 0.0], [64.0, 64.0], [0.0, 64.0]]
                .map(|position| UiVertex { position, uv: [0.0; 2], color_rgba: [0, 255, 0, 255] })
                .to_vec(),
            indices: vec![0, 1, 2, 0, 2, 3],
            texture: ui,
            clip: [0.0, 0.0, 64.0, 64.0],
        }],
    });
    for _ in 0..8 {
        backend.set_reflection_scene(Some(ReflectionScene {
            origin: Vec3::ZERO,
            quality: ReflectionQuality::Low,
            frame,
            faces: [&[], &[], &[], &walls, &[], &[]],
            refresh: ALL_FACES,
        }));
        backend.render(&frame, &main).unwrap();
    }
    assert!(centre(&capture(backend, &frame, &main))[1] > 240, "HUD was actually rendered");
    backend.set_ui_layer(UiLayer::default());
    let image = capture(backend, &frame, &main);
    let top = (20 * image.width as usize + 32) * 4;
    let bottom = (44 * image.width as usize + 32) * 4;
    assert!(image.rgba[top] > 180 && image.rgba[top + 2] < 25, "reflection top is red; HUD excluded");
    assert!(image.rgba[bottom + 2] > 180 && image.rgba[bottom] < 25, "reflection bottom is blue");
    backend.set_reflection_scene(None);
    backend.free_ui_texture(ui);
    samples
}

#[test]
#[ignore = "needs a GPU"]
fn local_reflections_capture_all_axes_and_follow_quality_origin_and_off() {
    let _gpu = serial();
    let mut native = Renderer::headless(
        (SIZE[0], SIZE[1]),
        RendererOptions {
            backend: GraphicsApi::Vulkan,
            vsync: false,
            force_fallback_adapter: false,
            pipeline_cache_dir: None,
        },
    )
    .expect("Vulkan adapter");
    exercise(&mut native);
}
