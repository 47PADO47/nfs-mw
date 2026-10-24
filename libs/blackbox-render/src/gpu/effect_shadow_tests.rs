//! Streaks and glows that ask for the sun's shadow take it; the others do not.
use blackbox_gfx::{
    BlendMode, EffectLayer, EffectVertex, FrameParams, Instance, Projection, RenderBackend, RenderExperiments, Shading,
};
use blackbox_gfx_testkit::mesh::{MeshBuilder, prelit};
use glam::{Mat4, Vec3};

const SIZE: u32 = 96;

fn quad(layer: &mut EffectLayer, centre_y: f32, shadowed: f32) {
    let corner = |y: f32, z: f32, u: f32, v: f32| EffectVertex {
        position: [4.0, centre_y + y, z],
        color: [255; 4],
        uv: [u, v],
        detail: [0.0, shadowed],
    };
    let (a, b, c, d) = (
        corner(-0.3, -0.3, 0.0, 0.0),
        corner(0.3, -0.3, 1.0, 0.0),
        corner(0.3, 0.3, 1.0, 1.0),
        corner(-0.3, 0.3, 0.0, 1.0),
    );
    layer.streaks.extend([a, b, c, a, c, d]);
}

fn project(frame: &FrameParams, point: Vec3) -> (u32, u32) {
    let clip = frame.view_proj() * point.extend(1.0);
    let ndc = clip / clip.w;
    let size = SIZE as f32;
    (((ndc.x * 0.5 + 0.5) * size) as u32, ((0.5 - ndc.y * 0.5) * size) as u32)
}

#[test]
#[ignore = "needs a GPU"]
fn flagged_streaks_take_the_sun_shadow_and_unflagged_ones_do_not() {
    let _gpu = crate::gpu::test_support::serial();
    let mut r = crate::gpu::test_support::headless((SIZE, SIZE)).expect("GPU adapter");
    let mut wall = MeshBuilder::new();
    wall.state(None, BlendMode::Opaque, Shading::Prelit);
    wall.cuboid(Vec3::new(6.0, -6.0, -5.0), Vec3::new(6.1, 6.0, 7.0), prelit([0.0; 3], 1.0));
    let wall = [Instance::new(wall.finish().upload(&mut r, "probe wall"), Mat4::IDENTITY)];
    let mut blocker = MeshBuilder::new();
    blocker.state(None, BlendMode::Opaque, Shading::Prelit);
    blocker.cuboid(Vec3::new(1.0, -0.5, -0.5), Vec3::new(1.2, 0.5, 0.5), prelit([0.2; 3], 1.0));
    let blocker = blocker.finish().upload(&mut r, "probe blocker");
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
    r.set_shadow_casters(Some(&[Instance::new(blocker, Mat4::IDENTITY)]));
    // The streak behind the blocker sits in its shadow; the other, 1.5 m to the side, is lit.
    let mut shot = |flag: f32| {
        let mut layer = EffectLayer::default();
        quad(&mut layer, 0.0, flag);
        quad(&mut layer, 1.5, flag);
        r.set_effects(&layer);
        let backend: &mut dyn RenderBackend = &mut r;
        let id = backend.request_capture([SIZE, SIZE], &frame, &wall).expect("a capture");
        let image = backend.poll_capture(id).expect("completes at once").expect("an image");
        let at = |y: f32| {
            let (x, row) = project(&frame, Vec3::new(4.0, y, -0.2));
            image.pixel(x, row).expect("inside")[0]
        };
        (at(0.0), at(1.5))
    };
    let (plain_shadow, plain_lit) = shot(0.0);
    let (taken_shadow, taken_lit) = shot(1.0);
    assert!(plain_shadow > 80 && plain_lit > 80, "the streaks draw: {plain_shadow} {plain_lit}");
    assert!(plain_shadow.abs_diff(plain_lit) <= 8, "unflagged streaks ignore the shadow: {plain_shadow} {plain_lit}");
    assert!(taken_shadow + 40 < plain_shadow, "a flagged streak in shadow is darker: {taken_shadow} vs {plain_shadow}");
    assert!(taken_lit.abs_diff(plain_lit) <= 8, "a flagged streak in the light is not: {taken_lit} vs {plain_lit}");
}
