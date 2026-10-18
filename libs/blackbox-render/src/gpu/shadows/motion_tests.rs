//! Isolate sun motion at city coordinates, holding camera, casters, filter and lighting fixed.
use super::grid::Grid;
use crate::{Renderer, gpu::test_support};
use blackbox_gfx::{
    BlendMode, FrameParams, Instance, LightingDebug, Projection, RenderBackend, RenderExperiments, Shading,
};
use blackbox_gfx_testkit::mesh::{MeshBuilder, prelit};
use glam::{Mat4, Quat, Vec3};

pub(super) const SIZE: (u32, u32) = (640, 360);

pub(super) fn scene(r: &mut Renderer, offset: Vec3) -> (Vec<Instance>, Vec<Instance>) {
    let mut mesh = MeshBuilder::new();
    mesh.state(None, BlendMode::Opaque, Shading::Prelit);
    mesh.flat_quad(
        [
            Vec3::new(-50.0, -50.0, 0.0),
            Vec3::new(50.0, -50.0, 0.0),
            Vec3::new(50.0, 50.0, 0.0),
            Vec3::new(-50.0, 50.0, 0.0),
        ],
        prelit([0.7; 3], 1.0),
    );
    let ground = mesh.finish().upload(r, "moving sun ground");
    let mut mesh = MeshBuilder::new();
    mesh.state(None, BlendMode::Opaque, Shading::Prelit);
    mesh.cuboid(Vec3::new(-1.0, -2.2, 0.45), Vec3::new(1.0, 2.2, 1.2), prelit([0.5; 3], 1.0));
    let body = mesh.finish().upload(r, "raised body shadow caster");
    let transform = Mat4::from_translation(offset);
    (vec![Instance::new(ground, transform)], vec![Instance::new(body, transform)])
}

#[test]
#[ignore = "needs a GPU"]
fn local_grid_reduces_moving_sun_flicker_without_widening_the_filter() {
    let _serial = test_support::serial();
    let mut measures = Vec::new();
    for legacy in [true, false] {
        let Some(mut r) = test_support::headless(SIZE) else { return };
        let offset = Vec3::new(2151.0, 1399.0, 148.7);
        let (ground, casters) = scene(&mut r, offset);
        let eye = offset + Vec3::new(3.0, -8.0, 4.5);
        let mut frame = FrameParams {
            view: glam::camera::rh::view::look_at_mat4(eye, offset, Vec3::Z),
            projection: Projection::PerspectiveInfiniteReverse {
                fov_y: 1.0,
                aspect: SIZE.0 as f32 / SIZE.1 as f32,
                near: 0.1,
            },
            camera_position: eye,
            light_dir: -Vec3::new(-0.932, -0.262, 0.249).normalize(),
            clear_color: [1.0; 3],
            fog: None,
            scattering: None,
            surface_light: None,
            camera_cut: false,
            shadow_tuning: None,
        };
        r.set_shadow_casters(Some(&casters));
        r.set_render_experiments(RenderExperiments {
            sun_shadows: true,
            shadow_resolution: 4096,
            shadow_distance: 320.0,
            far_shadows: true,
            shadow_filter: 0.75,
            soft_shadows: false,
            lighting_debug: LightingDebug::SunVisibility,
            ..Default::default()
        });
        r.capture(SIZE.0, SIZE.1, &frame, &ground).unwrap();
        if legacy {
            r.shadows.as_mut().unwrap().grid = Grid::world_origin();
        }
        let mut frames: Vec<Vec<f32>> = Vec::new();
        let mut mass = 0.0;
        for step in 0..48 {
            // Approximately 0.1 degree/second at 60 Hz, matching a 150-second clock hour.
            frame.light_dir =
                -(Quat::from_rotation_z(step as f32 * 0.00003) * Vec3::new(-0.932, -0.262, 0.249)).normalize();
            let rgba = r.capture(SIZE.0, SIZE.1, &frame, &ground).unwrap();
            let values: Vec<_> = rgba.as_chunks::<4>().0.iter().map(|p| f32::from(p[0]) / 255.0).collect();
            mass += values.iter().map(|v| 1.0 - v).sum::<f32>();
            if step == 24
                && let Ok(directory) = std::env::var("BLACKBOX_SHADOW_MOTION_DIR")
            {
                let name = if legacy { "world-origin" } else { "local-origin" };
                std::fs::write(std::path::Path::new(&directory).join(format!("{name}.rgba")), &rgba).unwrap();
            }
            frames.push(values);
        }
        let acceleration = frames
            .windows(3)
            .map(|f| f[0].iter().zip(&f[1]).zip(&f[2]).map(|((a, b), c)| (c - 2.0 * b + a).abs()).sum::<f32>())
            .sum::<f32>();
        assert!(mass > 1000.0, "fixture must exercise visible shadow coverage");
        eprintln!("legacy={legacy}: acceleration={acceleration:.4}, shadow_mass={mass:.4}");
        measures.push((acceleration, mass));
    }
    assert!((measures[0].1 - measures[1].1).abs() / measures[0].1 < 0.03, "preserve shadow coverage");
    assert!(
        measures[1].0 < measures[0].0 * 0.8,
        "local grid must reduce flicker at the same filter width: {measures:?}"
    );
}
