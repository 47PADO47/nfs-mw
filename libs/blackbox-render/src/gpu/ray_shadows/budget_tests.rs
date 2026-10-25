//! Small synthetic timing/quality probe; no game assets or wall-clock performance assertions.
use crate::{Renderer, gpu::test_support};
use blackbox_gfx::*;
use blackbox_gfx_testkit::mesh::{MeshBuilder, prelit};
use glam::{Mat4, Quat, Vec3};
#[test]
#[ignore = "Vulkan RT GPU diagnostic"]
fn shadow_budget_probe() {
    if std::env::var("BLACKBOX_RT_REFLECTIONS").as_deref() != Ok("1")
        || test_support::test_api() != crate::Backend::Vulkan
    {
        return;
    }
    let _gpu = test_support::serial();
    let size = (1280, 720);
    let mut r = Renderer::headless(size, test_support::options()).unwrap();
    let mut m = MeshBuilder::new();
    m.state(None, BlendMode::Opaque, Shading::Prelit);
    m.flat_quad(
        [
            Vec3::new(-100.0, -100.0, 0.0),
            Vec3::new(100.0, -100.0, 0.0),
            Vec3::new(100.0, 100.0, 0.0),
            Vec3::new(-100.0, 100.0, 0.0),
        ],
        prelit([0.7; 3], 1.0),
    );
    let ground = Instance::new(m.finish().upload(&mut r, "budget ground"), Mat4::IDENTITY);
    let mut m = MeshBuilder::new();
    m.state(None, BlendMode::Opaque, Shading::Prelit);
    for x in -4_i32..=4 {
        for y in -4..=4 {
            let p = Vec3::new(x as f32 * 6.0, y as f32 * 8.0, 0.4);
            m.cuboid(p, p + Vec3::new(1.5, 3.0, 1.2 + (x + y).unsigned_abs() as f32), prelit([0.5; 3], 1.0));
        }
    }
    let caster = Instance::new(m.finish().upload(&mut r, "budget blockers"), Mat4::IDENTITY);
    r.set_shadow_casters(Some(&[caster]));
    let eye = Vec3::new(3.0, -9.0, 4.0);
    let mut frame = FrameParams {
        view: glam::camera::rh::view::look_at_mat4(eye, Vec3::ZERO, Vec3::Z),
        projection: Projection::PerspectiveInfiniteReverse { fov_y: 0.9, aspect: 1280.0 / 720.0, near: 0.1 },
        camera_position: eye,
        light_dir: -Vec3::new(-0.7, 0.1, 0.7).normalize(),
        clear_color: [1.0; 3],
        fog: None,
        scattering: None,
        surface_light: None,
        camera_cut: false,
        shadow_tuning: None,
    };
    let mut cascade_acceleration = None;
    for (name, rt, soft, resolution, temporal) in [
        ("cascade", false, false, 100, false),
        ("cascade stabilized", false, false, 100, true),
        ("hard RT", true, false, 100, false),
        ("soft RT 100%", true, true, 100, false),
        ("soft RT 50%", true, true, 50, false),
    ] {
        r.set_render_experiments(RenderExperiments {
            sun_shadows: true,
            cascade_temporal: temporal,
            ray_sun_shadows: rt,
            ray_soft_shadows: soft,
            ray_shadow_temporal: true,
            ray_shadow_samples: 4,
            ray_shadow_resolution: resolution,
            shadow_resolution: 2048,
            shadow_distance: 160.0,
            soft_shadows: true,
            sun_angle: 0.55,
            lighting_debug: LightingDebug::SunVisibility,
            ..Default::default()
        });
        let mut times = Vec::new();
        let mut images: Vec<Vec<u8>> = Vec::new();
        for i in 0..24 {
            frame.light_dir = -(Quat::from_rotation_z(i as f32 * 0.00003) * Vec3::new(-0.7, 0.1, 0.7)).normalize();
            r.render(&frame, &[ground]).unwrap();
            let image = r.read_output().unwrap().unwrap().rgba;
            if i >= 8 {
                if let Some(t) = r.timing.as_ref().and_then(|t| t.latest) {
                    times.push(t);
                }
                images.push(image);
            }
        }
        let med = |index: usize| {
            let mut v: Vec<_> = times.iter().map(|t| t[index]).collect();
            v.sort_by(f32::total_cmp);
            v[v.len() / 2]
        };
        let acceleration: f64 = images
            .windows(3)
            .map(|f| {
                f[0].as_chunks::<4>()
                    .0
                    .iter()
                    .zip(f[1].as_chunks::<4>().0)
                    .zip(f[2].as_chunks::<4>().0)
                    .map(|((a, b), c)| (f64::from(c[0]) - 2.0 * f64::from(b[0]) + f64::from(a[0])).abs())
                    .sum::<f64>()
            })
            .sum();
        eprintln!(
            "{name}: GPU total={:.3} surface/RT={:.3} atlas={:.3} acceleration={acceleration:.0}",
            med(0),
            med(2),
            med(1)
        );
        if name == "cascade" {
            cascade_acceleration = Some(acceleration);
        }
        if temporal {
            assert!(
                acceleration < cascade_acceleration.unwrap() * 0.95,
                "short history must reduce moving-sun edge fluctuations"
            );
            r.set_shadow_casters(Some(&[]));
            r.render(&frame, &[ground]).unwrap();
            let cleared = r.read_output().unwrap().unwrap().rgba;
            assert!(cleared.as_chunks::<4>().0.iter().all(|p| p[0] >= 254), "removed blockers clear without a trail");
            r.set_shadow_casters(Some(&[caster]));
        }
    }
}
