//! Full-resolution diagnostics of the AO estimator, denoiser and temporal convergence.
use crate::{Renderer, gpu::test_support};
use blackbox_gfx::{
    AoMethod, AoQuality, AoSettings, BlendMode, FrameParams, GraphicsSettings, Instance, Projection, RenderBackend,
    Shading,
};
use blackbox_gfx_testkit::mesh::MeshBuilder;
use glam::{Mat4, Vec3};

const SIZE: (u32, u32) = (3840, 2160);

#[test]
#[ignore = "needs a GPU"]
fn car_scale_4k_ao_convergence() {
    let _serial = test_support::serial();
    let mut r = test_support::headless(SIZE).expect("GPU adapter required");
    let (frame, instances) = scene(&mut r);
    for method in [AoMethod::XeGtao, AoMethod::TemporalXeGtao] {
        for quality in [AoQuality::Low, AoQuality::Medium, AoQuality::High] {
            r.apply_graphics(&GraphicsSettings {
                ao: AoSettings { method, quality, ..Default::default() },
                ..Default::default()
            });
            let count = match method.temporal() {
                true => 48,
                false => 1,
            };
            let mut previous: Option<Vec<u8>> = None;
            let mut variation = 0.0;
            let mut samples = 0;
            for i in 0..count {
                let current = blackbox_gfx::FrameParams { camera_cut: i == 0, ..frame };
                r.render(&current, &instances).unwrap();
                let mask = r.read_raw_texture(r.shared.shadows.ambient.texture(), SIZE, 1).unwrap();
                if i >= 16
                    && let Some(before) = &previous
                {
                    for (&a, &b) in mask.iter().zip(before) {
                        if a.min(b) >= 245 {
                            continue;
                        }
                        variation += f64::from(a.abs_diff(b));
                        samples += 1;
                    }
                }
                if i == 0 || i == count - 1 {
                    dump(&format!("{method}-{quality}-{i}-mask"), SIZE, &mask);
                    let ao = r.ao.as_ref().unwrap();
                    let images = ao.images.as_ref().unwrap();
                    for (name, image) in [("working", &images.raw), ("filtered", &images.filtered)] {
                        let bytes = r.read_raw_texture(image.texture(), images.size, 8).unwrap();
                        let values: Vec<u8> = bytes
                            .as_chunks::<8>()
                            .0
                            .iter()
                            .map(|p| {
                                (half::f16::from_le_bytes([p[0], p[1]]).to_f32().clamp(0.0, 1.0) * 255.0).round() as u8
                            })
                            .collect();
                        dump(&format!("{method}-{quality}-{i}-{name}"), images.size, &values);
                    }
                    dump_estimate(&r, &format!("{method}-{quality}-{i}-estimate"));
                }
                previous = Some(mask);
            }
            let mask = previous.unwrap();
            assert!(mask.iter().filter(|&&v| v < 245).count() > 1000, "contacts must remain visible");
            if samples > 0 {
                println!("4K {method}/{quality}: mean mask frame delta {:.4}", variation / f64::from(samples));
            }
            // Compare contact shape to a dense estimate using the same resolution/filter.
            // This detects bias or detail loss that a low frame-difference score misses.
            let original = r.ao.as_ref().unwrap().pipelines.xegtao.clone();
            r.ao.as_mut().unwrap().pipelines.xegtao = super::sampling_tests::pipeline(&r, 24, 24);
            let cut = FrameParams { camera_cut: true, ..frame };
            r.render(&cut, &instances).unwrap();
            let reference = r.read_raw_texture(r.shared.shadows.ambient.texture(), SIZE, 1).unwrap();
            r.ao.as_mut().unwrap().pipelines.xegtao = original;
            dump(&format!("{method}-{quality}-reference"), SIZE, &reference);
            let source = super::sources()[0].1.replace(
                "let noise = xe_phase(id.xy);",
                "let noise = fract(sample_phase(id.xy) + params.fog.w * vec2<f32>(0.754877666, 0.569840296));",
            );
            let original = r.ao.as_ref().unwrap().pipelines.xegtao.clone();
            r.ao.as_mut().unwrap().pipelines.xegtao = super::sampling_tests::source_pipeline(&r, source);
            for i in 0..count {
                r.render(&FrameParams { camera_cut: i == 0, ..frame }, &instances).unwrap();
            }
            let legacy = r.read_raw_texture(r.shared.shadows.ambient.texture(), SIZE, 1).unwrap();
            r.ao.as_mut().unwrap().pipelines.xegtao = original;
            dump(&format!("{method}-{quality}-legacy"), SIZE, &legacy);
            println!(
                "4K {method}/{quality}: dense RMS legacy={:.4}, candidate={:.4}; column error legacy={:.4}, candidate={:.4}",
                rms(&legacy, &reference),
                rms(&mask, &reference),
                columns(&legacy, &reference),
                columns(&mask, &reference)
            );
            assert!(
                columns(&mask, &reference) < columns(&legacy, &reference) * 0.7,
                "{method}/{quality}: remove coherent columns without extra filtering"
            );
            assert!(
                rms(&mask, &reference) <= rms(&legacy, &reference) * 1.05,
                "{method}/{quality}: preserve contact shape against dense reference"
            );
        }
    }
}

#[test]
#[ignore = "needs a GPU"]
fn moving_camera_4k_ao_noise_is_reduced_at_fixed_ground_points() {
    let _serial = test_support::serial();
    let mut r = test_support::headless(SIZE).expect("GPU adapter required");
    let (frame, instances) = scene(&mut r);
    for quality in [AoQuality::Low, AoQuality::High] {
        r.apply_graphics(&GraphicsSettings {
            ao: AoSettings { method: AoMethod::TemporalXeGtao, quality, ..Default::default() },
            ..Default::default()
        });
        r.render(&frame, &instances).unwrap();
        let original = r.ao.as_ref().unwrap().pipelines.xegtao.clone();
        let source = super::sources()[0].1.replace(
            "let noise = xe_phase(id.xy);",
            "let noise = fract(sample_phase(id.xy) + params.fog.w * vec2<f32>(0.754877666, 0.569840296));",
        );
        let legacy = super::sampling_tests::source_pipeline(&r, source);
        let mut scores = Vec::new();
        for pipeline in [legacy, original] {
            r.ao.as_mut().unwrap().pipelines.xegtao = pipeline;
            let mut before: Option<Vec<f64>> = None;
            let mut error = 0.0;
            let mut samples = 0;
            for i in 0..48 {
                let displacement = Vec3::X * (i as f32 * 0.0025);
                let current = FrameParams {
                    view: frame.view * Mat4::from_translation(-displacement),
                    camera_position: frame.camera_position + displacement,
                    camera_cut: i == 0,
                    ..frame
                };
                r.render(&current, &instances).unwrap();
                let mask = r.read_raw_texture(r.shared.shadows.ambient.texture(), SIZE, 1).unwrap();
                let values: Vec<f64> = (0..20)
                    .flat_map(|y| (0..30).map(move |x| Vec3::new(-0.8 + x as f32 * 0.05, -2.5 - y as f32 * 0.04, 0.0)))
                    .map(|p| ground_sample(&mask, &current, p))
                    .collect();
                if i >= 16
                    && let Some(old) = &before
                {
                    error += values.iter().zip(old).map(|(a, b)| (a - b).abs()).sum::<f64>();
                    samples += values.len();
                }
                before = Some(values);
            }
            scores.push(error / samples as f64);
        }
        println!("4K moving {quality}: ground frame delta legacy={:.4}, candidate={:.4}", scores[0], scores[1]);
        assert!(scores[1] < scores[0] * 0.95 + 0.02, "{quality}: moving contacts must not become noisier");
    }
}

fn ground_sample(mask: &[u8], frame: &FrameParams, point: Vec3) -> f64 {
    let p = frame.view_proj().project_point3(point);
    let x = (p.x + 1.0) * 0.5 * SIZE.0 as f32 - 0.5;
    let y = (1.0 - p.y) * 0.5 * SIZE.1 as f32 - 0.5;
    let sample = |dx, dy| f64::from(mask[(y as usize + dy) * SIZE.0 as usize + x as usize + dx]);
    let (fx, fy) = (f64::from(x.fract()), f64::from(y.fract()));
    (sample(0, 0) * (1.0 - fx) + sample(1, 0) * fx) * (1.0 - fy) + (sample(0, 1) * (1.0 - fx) + sample(1, 1) * fx) * fy
}

fn dump_estimate(r: &Renderer, name: &str) {
    if std::env::var_os("BLACKBOX_AO_CAPTURE_DIR").is_none() {
        return;
    }
    let ao = r.ao.as_ref().unwrap();
    let images = ao.images.as_ref().unwrap();
    // The normal spatial chain overwrites raw as its ping-pong intermediate.
    // Replay just generation after capturing the final outputs to inspect true raw AO.
    let mut encoder = r.device.create_command_encoder(&Default::default());
    {
        let mut pass = encoder.begin_compute_pass(&Default::default());
        pass.set_pipeline(&ao.pipelines.xegtao);
        pass.set_bind_group(0, &ao.inputs.as_ref().unwrap().groups[0], &[]);
        pass.dispatch_workgroups(images.size.0.div_ceil(8), images.size.1.div_ceil(8), 1);
    }
    r.queue.submit([encoder.finish()]);
    let bytes = r.read_raw_texture(images.raw.texture(), images.size, 8).unwrap();
    let values: Vec<u8> = bytes
        .as_chunks::<8>()
        .0
        .iter()
        .map(|p| (half::f16::from_le_bytes([p[0], p[1]]).to_f32().clamp(0.0, 1.0) * 255.0).round() as u8)
        .collect();
    dump(name, images.size, &values);
}

fn rms(mask: &[u8], reference: &[u8]) -> f64 {
    let mut sum = 0.0;
    let mut count = 0;
    for (&value, &target) in mask.iter().zip(reference) {
        if target >= 245 {
            continue;
        }
        sum += f64::from(value.abs_diff(target)).powi(2);
        count += 1;
    }
    assert!(count > 1000);
    (sum / f64::from(count)).sqrt()
}

// A vertical sampling pattern survives averaging along its columns. Remove the
// slowly varying contact envelope before measuring this coherent residual.
fn columns(mask: &[u8], reference: &[u8]) -> f64 {
    let profile: Vec<f64> = (1550..2290)
        .map(|x| {
            (1370..1450)
                .map(|y| {
                    let at = y * SIZE.0 as usize + x;
                    f64::from(mask[at]) - f64::from(reference[at])
                })
                .sum::<f64>()
                / 80.0
        })
        .collect();
    let errors: Vec<f64> = (16..profile.len() - 16)
        .map(|i| {
            let mean = profile[i - 16..=i + 16].iter().sum::<f64>() / 33.0;
            (profile[i] - mean).powi(2)
        })
        .collect();
    (errors.iter().sum::<f64>() / errors.len() as f64).sqrt()
}

fn scene(r: &mut Renderer) -> (FrameParams, Vec<Instance>) {
    let mut floor = MeshBuilder::new();
    floor.state(None, BlendMode::Opaque, Shading::Lit);
    floor.flat_quad(
        [
            Vec3::new(-20.0, -20.0, 0.0),
            Vec3::new(20.0, -20.0, 0.0),
            Vec3::new(20.0, 20.0, 0.0),
            Vec3::new(-20.0, 20.0, 0.0),
        ],
        [255; 4],
    );
    let mut body = MeshBuilder::new();
    body.state(None, BlendMode::Opaque, Shading::Lit);
    body.cuboid(Vec3::new(-1.0, -2.2, 0.45), Vec3::new(1.0, 2.2, 1.2), [255; 4]);
    let mut body = Instance::new(body.finish().upload(r, "AO car-sized body"), Mat4::IDENTITY);
    body.key = blackbox_gfx::InstanceKey::new(8, 1);
    let floor = Instance::new(floor.finish().upload(r, "AO flat road"), Mat4::IDENTITY);
    let eye = Vec3::new(0.0, -8.0, 3.5);
    let frame = FrameParams {
        view: glam::camera::rh::view::look_at_mat4(eye, Vec3::new(0.0, 0.0, 0.5), Vec3::Z),
        projection: Projection::PerspectiveInfiniteReverse {
            fov_y: 1.0,
            aspect: SIZE.0 as f32 / SIZE.1 as f32,
            near: 0.1,
        },
        camera_position: eye,
        light_dir: Vec3::NEG_Z,
        clear_color: [0.2, 0.3, 0.4],
        fog: None,
        scattering: None,
        surface_light: None,
        camera_cut: false,
        shadow_tuning: None,
    };
    (frame, vec![body, floor])
}

fn dump(name: &str, size: (u32, u32), values: &[u8]) {
    let Ok(directory) = std::env::var("BLACKBOX_AO_CAPTURE_DIR") else { return };
    let mut bytes = format!("P5\n{} {}\n255\n", size.0, size.1).into_bytes();
    bytes.extend_from_slice(values);
    std::fs::write(std::path::Path::new(&directory).join(format!("{name}.pgm")), bytes).unwrap();
}
