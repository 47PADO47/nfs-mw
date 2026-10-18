//! Fixed real-asset snapshots, with GPU timestamps rather than a paced window's FPS.
use super::*;
#[path = "frame_cost_tests.rs"]
mod frame_cost_tests;
#[path = "shadow_cost_tests.rs"]
mod shadow_cost_tests;
use blackbox_gfx::{Antialiasing, AoQuality, ReflectionQuality, RenderExperiments};
use blackbox_render::{Renderer, RendererOptions};
use std::{
    path::Path,
    time::{Duration, Instant},
};

#[test]
#[ignore = "needs the game and a GPU; diagnostic benchmark"]
fn world_gpu_timings() {
    let dir = GameDir::open(std::env::var_os("NFSMW_GAME_DIR").expect("set NFSMW_GAME_DIR")).unwrap();
    let api = match std::env::var("BLACKBOX_TEST_API").as_deref() {
        Ok("dx12") => blackbox_gfx::GraphicsApi::Dx12,
        _ => blackbox_gfx::GraphicsApi::Vulkan,
    };
    let output = std::env::var_os("NFSMW_PERF_OUT").map(std::path::PathBuf::from);
    let max_quality = std::env::var("NFSMW_PERF_MAX").as_deref() == Ok("1");
    let size = (3840, 2160);
    for (name, at) in [("hill", [556.0, 1802.0]), ("city", [2150.0, 1399.0])] {
        let mut renderer = Renderer::headless(
            size,
            RendererOptions { backend: api, vsync: false, force_fallback_adapter: false, pipeline_cache_dir: None },
        )
        .unwrap();
        let mut graphics = *renderer.graphics();
        graphics.reflections = ReflectionQuality::Medium;
        graphics.post.antialiasing = Antialiasing::Smaa;
        if max_quality {
            graphics.reflections = ReflectionQuality::High;
            graphics.post.tonemap = blackbox_gfx::Tonemap::Aces;
            graphics.post.colour_treatment = crate::settings::VisualTreatment::Low.strength();
        }
        renderer.apply_graphics(&graphics);
        renderer.set_render_experiments(RenderExperiments {
            sun_shadows: true,
            far_shadows: true,
            shadow_resolution: match max_quality {
                true => 4096,
                false => 2048,
            },
            contact_shadows: max_quality,
            clearcoat: max_quality,
            car_skylight: true,
            world_ambient: true,
            original_shadows: false,
            freeze_sun: true,
            ..Default::default()
        });
        let mut world = WorldScene::open(
            &dir,
            Options {
                at: Some(at),
                height: 40.0,
                heading: 45.0,
                pitch: -20.0,
                fog_distance: 3000.0,
                wait_for_load: true,
                drive: Some(DriveOptions { car: DEFAULT_CAR.into(), script: None }),
            },
        )
        .unwrap();
        renderer.set_prepass_depth_reuse(std::env::var("NFSMW_PERF_SEPARATE_DEPTH").as_deref() != Ok("1"));
        world.init(&mut renderer).unwrap();
        let start = Instant::now();
        while !world.ready() {
            world.update(&mut renderer, &ActionState::default(), 0.0);
            assert!(start.elapsed() < Duration::from_secs(60), "streaming timed out");
            std::thread::sleep(Duration::from_millis(5));
        }
        // Placement and chase-camera convergence need simulation ticks after streaming is ready.
        for _ in 0..120 {
            world.update(&mut renderer, &ActionState::default(), 1.0 / 60.0);
        }
        while !world.ready() {
            world.update(&mut renderer, &ActionState::default(), 0.0);
            assert!(start.elapsed() < Duration::from_secs(60), "drive streaming timed out");
            std::thread::sleep(Duration::from_millis(5));
        }
        world.update(&mut renderer, &ActionState::default(), 0.0);
        let (frame, instances) = world.frame(size.0 as f32 / size.1 as f32);
        let instances = instances.to_vec();
        for quality in [AoQuality::Medium, AoQuality::High] {
            if max_quality && quality != AoQuality::High {
                continue;
            }
            graphics.ao.quality = quality;
            renderer.apply_graphics(&graphics);
            let mut samples = Vec::new();
            for index in 0..32 {
                renderer.render(&frame, &instances).unwrap();
                // Read the submitted frame without creating another scene/depth target or
                // re-rendering it. GPU timestamps exclude the transfer and CPU wait.
                let image = renderer.read_output().unwrap().unwrap();
                let stats = renderer.experiment_stats();
                if index >= 8 {
                    samples.push([
                        stats.gpu_frame_ms.expect("timestamps supported"),
                        stats.gpu_shadow_ms.unwrap(),
                        stats.gpu_contact_ms.unwrap(),
                        stats.gpu_ao_ms.unwrap(),
                        stats.shadow_cpu_ms,
                    ]);
                }
                if index == 31
                    && let Some(output) = &output
                {
                    std::fs::create_dir_all(output).unwrap();
                    save(&output.join(format!("{name}-{quality}.png")), size, &image.rgba);
                }
            }
            let median = |column: usize| {
                let mut values: Vec<_> = samples.iter().map(|s| s[column]).collect();
                values.sort_by(f32::total_cmp);
                values[values.len() / 2]
            };
            eprintln!(
                "world {api:?} {name} {quality} 4K (max {max_quality}): GPU {:.3} ms, sun {:.3}, geometry {:.3}, AO {:.3}, shadow CPU {:.3}",
                median(0),
                median(1),
                median(2),
                median(3),
                median(4)
            );
        }
    }
}

fn save(path: &Path, size: (u32, u32), rgba: &[u8]) {
    let file = std::fs::File::create(path).unwrap();
    let mut encoder = png::Encoder::new(std::io::BufWriter::new(file), size.0, size.1);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder.write_header().unwrap().write_image_data(rgba).unwrap();
}
