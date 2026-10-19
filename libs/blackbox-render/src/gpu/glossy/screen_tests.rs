//! Actual hit/miss coverage for current-frame material reflections, including live toggles.
use crate::gpu::test_support;
use blackbox_gfx::*;
use blackbox_gfx_testkit::mesh::{MeshBuilder, prelit};
use glam::{Mat4, Vec3};

#[test]
#[ignore = "needs a GPU"]
fn selective_rays_hit_visible_scenery_and_misses_keep_the_probe() {
    let _gpu = test_support::serial();
    let size = (192, 144);
    let mut r = test_support::headless(size).expect("GPU required");
    let material = r.create_glossy_material(&GlossyMaterial {
        diffuse_min: [0.0, 0.0, 0.0, 1.0],
        diffuse_range: [0.0; 4],
        specular_min: [0.0; 3],
        specular_range: [0.0; 3],
        envmap_min: 2.0,
        envmap_range: 0.0,
        ..Default::default()
    });
    let black = [0, 0, 0, 255];
    r.set_environment(Environment::Faces { size: 1, faces: [&black; 6] });
    let mut mirror = MeshBuilder::new();
    mirror.state(None, BlendMode::Opaque, Shading::Glossy(material));
    mirror.flat_quad(
        [Vec3::new(-4.0, -4.0, 0.0), Vec3::new(4.0, -4.0, 0.0), Vec3::new(4.0, 4.0, 0.0), Vec3::new(-4.0, 4.0, 0.0)],
        prelit([1.0; 3], 1.0),
    );
    let mirror = Instance::new(mirror.finish().upload(&mut r, "selective mirror"), Mat4::IDENTITY);
    let mut wall = MeshBuilder::new();
    wall.state(None, BlendMode::Opaque, Shading::Prelit);
    wall.flat_quad(
        [Vec3::new(-5.0, 3.0, 0.0), Vec3::new(-5.0, 3.0, 5.0), Vec3::new(5.0, 3.0, 5.0), Vec3::new(5.0, 3.0, 0.0)],
        prelit([0.9, 0.0, 0.0], 1.0),
    );
    let wall = Instance::new(wall.finish().upload(&mut r, "visible red wall"), Mat4::IDENTITY);
    let eye = Vec3::new(0.0, -5.0, 3.0);
    let frame = FrameParams {
        view: glam::camera::rh::view::look_at_mat4(eye, Vec3::new(0.0, 1.0, 0.5), Vec3::Z),
        projection: Projection::PerspectiveInfiniteReverse {
            fov_y: 1.0,
            aspect: size.0 as f32 / size.1 as f32,
            near: 0.1,
        },
        camera_position: eye,
        light_dir: Vec3::NEG_Z,
        clear_color: [0.0; 3],
        fog: None,
        scattering: None,
        surface_light: None,
        camera_cut: false,
        shadow_tuning: None,
    };
    let scene = [mirror, wall];
    let old = r.capture(size.0, size.1, &frame, &scene).unwrap();
    r.set_render_experiments(RenderExperiments { screen_reflections: true, ..Default::default() });
    let new = r.capture(size.0, size.1, &frame, &scene).unwrap();
    let hits = old
        .as_chunks::<4>()
        .0
        .iter()
        .zip(new.as_chunks::<4>().0)
        .filter(|(a, b)| b[0] > a[0].saturating_add(30) && b[1] < 10 && b[2] < 10)
        .count();
    assert!(hits > 100, "visible red scenery must reach the mirror: {hits} hit pixels");
    let miss = r.capture(size.0, size.1, &frame, &[mirror]).unwrap();
    r.set_render_experiments(Default::default());
    let old_miss = r.capture(size.0, size.1, &frame, &[mirror]).unwrap();
    assert_eq!(miss, old_miss, "a miss preserves the exact probe response");
    assert_eq!(old, r.capture(size.0, size.1, &frame, &scene).unwrap(), "legacy path round trips");
}
