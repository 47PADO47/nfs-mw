//! Compare density choices with a moving sun; report GPU time separately from capture readback.
use super::motion_tests::{SIZE, scene};
use crate::gpu::test_support;
use blackbox_gfx::{FrameParams, LightingDebug, Projection, RenderBackend, RenderExperiments};
use glam::{Quat, Vec3};

#[test]
#[ignore = "needs a GPU with 8192 texture support"]
fn moving_sun_density_choices_preserve_coverage_and_report_cost() {
    let _serial = test_support::serial();
    let Some(mut r) = test_support::headless(SIZE) else { return };
    if r.device.limits().max_texture_dimension_2d < 8192 {
        eprintln!("skipping: 8192 shadow textures unavailable");
        return;
    }
    let offset = Vec3::new(2151.0, 1399.0, 148.7);
    let (ground, casters) = scene(&mut r, offset);
    let eye = offset + Vec3::new(3.0, -8.0, 4.5);
    let mut frame = FrameParams {
        view: glam::camera::rh::view::look_at_mat4(eye, offset, Vec3::Z),
        projection: Projection::PerspectiveInfiniteReverse { fov_y: 1.0, aspect: 640.0 / 360.0, near: 0.1 },
        camera_position: eye,
        light_dir: Vec3::NEG_Z,
        clear_color: [1.0; 3],
        fog: None,
        scattering: None,
        surface_light: None,
        camera_cut: false,
        shadow_tuning: None,
    };
    r.set_shadow_casters(Some(&casters));
    let mut masses = Vec::new();
    for (resolution, focus, filter, wide) in [
        (4096, false, 0.75, false),
        (4096, true, 0.75, false),
        (4096, true, 1.5, false),
        (8192, false, 0.75, false),
        (8192, false, 1.5, false),
        (4096, false, 1.5, true),
    ] {
        r.set_render_experiments(RenderExperiments {
            sun_shadows: true,
            shadow_resolution: resolution,
            shadow_distance: 320.0,
            far_shadows: true,
            near_shadow_focus: focus,
            wide_shadow_detail: wide,
            shadow_detail_resolution: 8192,
            shadow_filter: filter,
            soft_shadows: false,
            lighting_debug: LightingDebug::SunVisibility,
            ..Default::default()
        });
        let mut frames: Vec<Vec<f32>> = Vec::new();
        let mut mass = 0.0;
        let mut times = Vec::new();
        for step in 0..48 {
            frame.light_dir =
                -(Quat::from_rotation_z(step as f32 * 0.00003) * Vec3::new(-0.932, -0.262, 0.249)).normalize();
            let rgba = r.capture(SIZE.0, SIZE.1, &frame, &ground).unwrap();
            let values: Vec<_> = rgba.as_chunks::<4>().0.iter().map(|p| f32::from(p[0]) / 255.0).collect();
            mass += values.iter().map(|v| 1.0 - v).sum::<f32>();
            frames.push(values);
            let stats = r.experiment_stats();
            let expected_mib = match wide {
                true => 640,
                false => resolution * resolution * 16 / (1024 * 1024),
            };
            assert_eq!(stats.map_mib, expected_mib);
            if step >= 8
                && let Some(time) = stats.gpu_shadow_ms
            {
                times.push(time);
            }
            if step == 24
                && let Ok(directory) = std::env::var("BLACKBOX_SHADOW_MOTION_DIR")
            {
                std::fs::write(
                    std::path::Path::new(&directory)
                        .join(format!("density-{resolution}-{focus}-{filter}-wide{wide}.rgba")),
                    &rgba,
                )
                .unwrap();
            }
        }
        let acceleration = frames
            .windows(3)
            .map(|f| f[0].iter().zip(&f[1]).zip(&f[2]).map(|((a, b), c)| (c - 2.0 * b + a).abs()).sum::<f32>())
            .sum::<f32>();
        assert!(mass > 1000.0, "must retain visible shadow coverage");
        times.sort_by(f32::total_cmp);
        let median = times.get(times.len() / 2);
        eprintln!(
            "density resolution={resolution} focus={focus} filter={filter} wide={wide}: acceleration={acceleration:.4}, shadow_mass={mass:.4}, shadow_gpu_median_ms={median:?}, samples={}",
            times.len()
        );
        masses.push(mass);
    }
    assert!(
        masses.iter().all(|mass| (mass - masses[0]).abs() / masses[0] < 0.03),
        "density must preserve coverage: {masses:?}"
    );
}
