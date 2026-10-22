//! Current-frame shadows in reflection captures; raw/reference captures stay unchanged.
use super::*;
use blackbox_gfx::{BlendMode, FrameParams, Instance, Projection, RenderBackend, RenderExperiments, Shading};
use blackbox_gfx_testkit::mesh::{MeshBuilder, prelit};
use glam::{Mat4, Vec3};

#[test]
#[ignore = "needs a GPU"]
fn captured_sun_visibility_tracks_casters_without_a_stale_frame() {
    let _gpu = crate::gpu::test_support::serial();
    let mut r = crate::gpu::test_support::headless((96, 96)).expect("GPU adapter");
    let mut wall = MeshBuilder::new();
    wall.state(None, BlendMode::Opaque, Shading::Prelit);
    wall.cuboid(Vec3::new(4.0, -6.0, -5.0), Vec3::new(4.1, 6.0, 7.0), prelit([0.7; 3], 1.0));
    let wall = [Instance::new(wall.finish().upload(&mut r, "probe wall"), Mat4::IDENTITY)];
    let mut blocker = MeshBuilder::new();
    blocker.state(None, BlendMode::Opaque, Shading::Prelit);
    blocker.cuboid(Vec3::new(1.0, -3.0, -3.0), Vec3::new(1.2, 3.0, 5.0), prelit([0.2; 3], 1.0));
    let blocker = blocker.finish().upload(&mut r, "probe light blocker");
    let frame = FrameParams {
        view: glam::camera::rh::view::look_at_mat4(Vec3::ZERO, Vec3::X, Vec3::Z),
        projection: Projection::PerspectiveInfiniteReverse { fov_y: 1.0, aspect: 1.0, near: 0.1 },
        camera_position: Vec3::ZERO,
        light_dir: Vec3::X,
        clear_color: [0.0; 3],
        fog: None,
        scattering: None,
        surface_light: None,
        camera_cut: false,
        shadow_tuning: None,
    };
    r.set_render_experiments(RenderExperiments { sun_shadows: true, shadow_resolution: 1024, ..Default::default() });
    let mut samples = Vec::new();
    for (offset, shadowed) in [(20.0, true), (0.0, true), (20.0, true), (0.0, false)] {
        r.set_shadow_casters(Some(&[Instance::new(blocker, Mat4::from_translation(Vec3::Y * offset))]));
        r.set_reflection_scene(Some(ReflectionScene {
            sun_shadows: shadowed,
            fade_distance: None,
            origin: Vec3::ZERO,
            quality: ReflectionQuality::Low,
            frame,
            faces: [&wall; 6],
        }));
        r.capture(96, 96, &frame, &wall).unwrap();
        let capture = r.reflections.as_ref().unwrap();
        let bytes = r.read_raw_texture(capture.faces[0].texture(), (64, 64), 4).unwrap();
        samples.push(bytes[(32 * 64 + 32) * 4]);
    }
    assert!(samples[1] + 30 < samples[0], "a shaded wall must be darker in the capture: {samples:?}");
    assert!(samples[0].abs_diff(samples[2]) <= 2, "moving a caster away updates the same frame: {samples:?}");
    assert!(samples[0].abs_diff(samples[3]) <= 2, "raw/reference capture bypasses added shadows: {samples:?}");
}
