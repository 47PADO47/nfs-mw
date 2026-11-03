//! Light fog across views: an off-screen capture keeps its own fog state and leaves the main view's alone.
use blackbox_gfx::{
    BlendMode, CaptureId, EffectLayer, FrameParams, Instance, Projection, RenderBackend, RgbaImage, Shading,
};
use blackbox_gfx_testkit::mesh::{MeshBuilder, prelit};
use glam::{Mat4, Vec3};

fn frame(camera: Vec3) -> FrameParams {
    FrameParams {
        view: glam::camera::rh::view::look_at_mat4(camera, camera + Vec3::X, Vec3::Z),
        projection: Projection::PerspectiveInfiniteReverse { fov_y: 1.0, aspect: 16.0 / 9.0, near: 0.1 },
        camera_position: camera,
        light_dir: Vec3::X,
        clear_color: [0.0; 3],
        fog: None,
        scattering: None,
        surface_light: None,
        camera_cut: false,
        shadow_tuning: None,
    }
}

fn lamp() -> blackbox_gfx::LocalLight {
    blackbox_gfx::LocalLight {
        position: [6.0, 0.0, 3.0],
        range: 12.0,
        direction: [0.0, 0.0, -1.0],
        outer_cos: 0.2,
        inner_cos: 0.7,
        color: [5.0, 4.0, 3.0],
        upper_plane: None,
        casts_shadow: false,
        shadow_exclude: None,
    }
}

fn poll(renderer: &mut super::Renderer, id: CaptureId) -> RgbaImage {
    for _ in 0..200 {
        if let Some(result) = RenderBackend::poll_capture(renderer, id) {
            return result.expect("the view capture reads back");
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    panic!("the view capture never finished");
}

/// Main frames of a fogged scene, with a smaller view capture after each when `captures` is set. Returns the last
/// main image and the light fog's memory after each main frame.
fn run(captures: bool) -> (RgbaImage, Vec<u64>) {
    let mut renderer = super::test_support::headless((160, 90)).expect("GPU adapter");
    renderer.set_local_lights(&[lamp()]);
    let mut wall = MeshBuilder::new();
    wall.state(None, BlendMode::Opaque, Shading::Prelit);
    wall.cuboid(Vec3::new(-0.01, -8.0, -8.0), Vec3::new(0.01, 8.0, 8.0), prelit([0.0; 3], 1.0));
    let wall = wall.finish().upload(&mut renderer, "fog backdrop");
    let instances = [Instance::new(wall, Mat4::from_translation(Vec3::X * 20.0))];
    let mut bytes = Vec::new();
    let mut image = None;
    for step in 0..6 {
        renderer.set_effects(&EffectLayer { light_fog: [2.0, 1.0], ..Default::default() });
        renderer.render(&frame(Vec3::ZERO), &instances).unwrap();
        image = Some(renderer.read_output().unwrap().unwrap());
        bytes.push(renderer.effects.light_fog_bytes());
        if !captures {
            continue;
        }
        let mirror = frame(Vec3::new(0.0, 0.5 * step as f32, 0.0));
        let id = renderer.request_view_capture([96, 32], &mirror, &instances).unwrap();
        let picture = poll(&mut renderer, id);
        assert!(picture.rgba.as_chunks::<4>().0.iter().any(|p| p[0] > 0), "the capture shows the fog");
    }
    (image.unwrap(), bytes)
}

#[test]
#[ignore = "needs a GPU"]
fn view_captures_keep_their_own_fog_and_leave_the_main_view_alone() {
    let _gpu = super::test_support::serial();
    let (alone, alone_bytes) = run(false);
    let (mixed, mixed_bytes) = run(true);
    assert!(alone.rgba.as_chunks::<4>().0.iter().any(|p| p[0] > 0), "the main view shows the fog");
    assert!(alone_bytes.windows(2).all(|w| w[0] == w[1]), "a steady main view allocates nothing more");
    assert!(mixed_bytes[1] > mixed_bytes[0], "the capture has a state of its own");
    assert!(mixed_bytes[1..].windows(2).all(|w| w[0] == w[1]), "alternating views reallocate nothing");
    assert_eq!(alone.rgba, mixed.rgba, "captures between main frames leave the main fog unchanged");
}
