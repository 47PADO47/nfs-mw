//! The long screen-space shadows: no self-shadowing on open planes, and a longer shadow than the contact march.
use super::wavefront::{WAVE, dispatch_list};
use crate::gpu::test_support;
use blackbox_gfx::{BlendMode, FrameParams, Instance, Projection, RenderBackend, RenderExperiments, Shading};
use blackbox_gfx_testkit::mesh::{MeshBuilder, prelit};
use glam::{Mat4, Vec3};

const SIZE: (u32, u32) = (640, 400);

fn scene(r: &mut crate::Renderer) -> (FrameParams, Instance, Instance) {
    let mut plane = MeshBuilder::new();
    plane.state(None, BlendMode::Opaque, Shading::Prelit);
    plane.flat_quad(
        [
            Vec3::new(-15.0, -15.0, 0.0),
            Vec3::new(15.0, -15.0, 0.0),
            Vec3::new(15.0, 15.0, 0.0),
            Vec3::new(-15.0, 15.0, 0.0),
        ],
        prelit([0.7; 3], 1.0),
    );
    let ground = Instance::new(plane.finish().upload(r, "screen shadow ground"), Mat4::IDENTITY);
    let mut post = MeshBuilder::new();
    post.state(None, BlendMode::Opaque, Shading::Prelit);
    post.cuboid(Vec3::new(-0.3, -0.3, 0.0), Vec3::new(0.3, 0.3, 2.0), prelit([0.6; 3], 1.0));
    let post = Instance::new(post.finish().upload(r, "screen shadow post"), Mat4::IDENTITY);
    let eye = Vec3::new(3.0, -6.0, 4.0);
    let frame = FrameParams {
        view: glam::camera::rh::view::look_at_mat4(eye, Vec3::ZERO, Vec3::Z),
        projection: Projection::PerspectiveInfiniteReverse { fov_y: 1.0, aspect: 1.6, near: 0.1 },
        camera_position: eye,
        light_dir: Vec3::new(1.0, 0.3, -0.6).normalize(),
        clear_color: [0.2; 3],
        fog: None,
        scattering: None,
        surface_light: None,
        camera_cut: false,
        shadow_tuning: None,
    };
    (frame, ground, post)
}

fn darker(a: &[u8], b: &[u8]) -> usize {
    a.as_chunks::<4>().0.iter().zip(b.as_chunks::<4>().0).filter(|(a, b)| b[0].saturating_add(8) < a[0]).count()
}

#[test]
fn dispatches_cover_the_viewport_for_lights_on_and_far_off_screen() {
    for light in [[0.2, -0.3, 0.0, 1.0], [0.9, 0.9, 0.0, -1.0], [3.0, 0.1, 0.0, 0.5], [1.0, 0.0, 0.0, 0.0]] {
        let list = dispatch_list(light, [1920, 1080], [0, 0], [1920, 1080]);
        assert!((1..=8).contains(&list.dispatches.len()), "{light:?}: {} dispatches", list.dispatches.len());
        let threads: u64 =
            list.dispatches.iter().map(|d| d.waves.iter().map(|&w| u64::from(w)).product::<u64>()).sum::<u64>() * 64;
        assert!(threads >= 1920 * 1080, "{light:?}: {threads} threads cannot cover the viewport");
        assert!(threads < 1920 * 1080 * 3, "{light:?}: {threads} threads is far more than the viewport");
        assert!(list.dispatches.iter().all(|d| d.waves[0] == WAVE as u32 && d.offset.iter().all(|o| o % WAVE == 0)));
    }
}

#[test]
#[ignore = "needs a GPU"]
fn screen_shadows_leave_open_planes_lit_and_outreach_the_contact_march() {
    let _gpu = test_support::serial();
    let Some(mut r) = test_support::headless(SIZE) else { return };
    let (frame, ground, post) = scene(&mut r);
    let short = RenderExperiments { contact_shadows: true, ..Default::default() };
    let long = RenderExperiments { screen_space_shadows: true, ..short };
    for rotation in [Mat4::IDENTITY, Mat4::from_rotation_x(std::f32::consts::FRAC_PI_2)] {
        let wall = [Instance::new(ground.mesh, rotation)];
        r.set_render_experiments(Default::default());
        let off = r.capture(SIZE.0, SIZE.1, &frame, &wall).unwrap();
        r.set_render_experiments(long);
        assert_eq!(off, r.capture(SIZE.0, SIZE.1, &frame, &wall).unwrap(), "an open plane must not shadow itself");
    }
    let both = [ground, post];
    r.set_render_experiments(Default::default());
    let off = r.capture(SIZE.0, SIZE.1, &frame, &both).unwrap();
    r.set_render_experiments(short);
    let marched = darker(&off, &r.capture(SIZE.0, SIZE.1, &frame, &both).unwrap());
    r.set_render_experiments(long);
    let on = r.capture(SIZE.0, SIZE.1, &frame, &both).unwrap();
    // Wavefronts meet along seams where two threads may round to one pixel, so a repeat may differ there.
    let again = r.capture(SIZE.0, SIZE.1, &frame, &both).unwrap();
    let changed = on.as_chunks::<4>().0.iter().zip(again.as_chunks::<4>().0).filter(|(a, b)| a != b).count();
    assert!(changed <= 8, "the same frame changed in {changed} pixels");
    let long_pixels = darker(&off, &on);
    assert!(long_pixels > marched * 3 && long_pixels > 600, "the long shadow: {long_pixels} vs {marched} marched");
    for (a, b) in off.as_chunks::<4>().0.iter().zip(on.as_chunks::<4>().0) {
        assert!(b[0] <= a[0] + 1, "screen shadows only darken");
    }
    r.set_render_experiments(Default::default());
    assert_eq!(off, r.capture(SIZE.0, SIZE.1, &frame, &both).unwrap(), "off restores the frame");
}
