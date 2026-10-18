//! Matched installed scene and GPU timestamps for near-detail allocation choices.
use super::*;
use blackbox_gfx::{AoMethod, LightingDebug};

#[test]
#[ignore = "needs the game and a GPU; diagnostic benchmark"]
fn world_shadow_layout_gpu_timings() {
    let dir = GameDir::open(std::env::var_os("NFSMW_GAME_DIR").expect("set NFSMW_GAME_DIR")).unwrap();
    let api = match std::env::var("BLACKBOX_TEST_API").as_deref() {
        Ok("dx12") => blackbox_gfx::GraphicsApi::Dx12,
        _ => blackbox_gfx::GraphicsApi::Vulkan,
    };
    let output = std::env::var_os("NFSMW_PERF_OUT").map(std::path::PathBuf::from);
    let size = (3840, 2160);
    let mut renderer = Renderer::headless(
        size,
        RendererOptions { backend: api, vsync: false, force_fallback_adapter: false, pipeline_cache_dir: None },
    )
    .unwrap();
    let mut graphics = *renderer.graphics();
    graphics.reflections = ReflectionQuality::High;
    graphics.post.antialiasing = Antialiasing::Smaa;
    graphics.ao.quality = AoQuality::High;
    graphics.ao.method = AoMethod::XeGtao;
    graphics.ao.intensity = 1.0;
    renderer.apply_graphics(&graphics);
    // Populate the separate resident caster feed before taking the fixed snapshot.
    renderer.set_render_experiments(RenderExperiments {
        original_world_materials: true,
        sun_shadows: true,
        far_shadows: true,
        shadow_distance: 320.0,
        shadow_resolution: 4096,
        ..Default::default()
    });
    let mut world = WorldScene::open(
        &dir,
        Options {
            at: Some([1858.0, 1936.0]),
            height: 12.0,
            heading: 45.0,
            pitch: -30.0,
            fog_distance: 3000.0,
            wait_for_load: true,
            drive: Some(DriveOptions { car: "ELISE".into(), script: None }),
        },
    )
    .unwrap();
    world.init(&mut renderer).unwrap();
    wait_ready(&mut world, &mut renderer);
    for _ in 0..120 {
        world.update(&mut renderer, &ActionState::default(), 1.0 / 60.0);
    }
    wait_ready(&mut world, &mut renderer);
    world.set_environment(crate::viewer::Environment {
        hour: 15.0,
        seconds_per_hour: 0.0,
        cloud_override: Some(0.14),
        settle_clouds: true,
        ..Default::default()
    });
    world.update(&mut renderer, &ActionState::default(), 0.0);
    assert!(world.shadow_feed.fed && !world.shadow_feed.casters.is_empty(), "resident shadow feed required");
    let (frame, instances) = world.frame(size.0 as f32 / size.1 as f32);
    let instances = instances.to_vec();
    let mut report =
        String::from("layout,map_mib,sample,gpu_main_ms,gpu_shadow_ms,gpu_geometry_ms,gpu_ao_ms,cpu_shadow_ms\n");
    for (name, base, detail, wide, focus, mib) in [
        ("regular4k", 4096, 4096, false, false, 256),
        ("focused4k", 4096, 4096, false, true, 256),
        ("wide4k", 4096, 4096, true, false, 256),
        ("wide8k", 4096, 8192, true, false, 640),
        ("regular8k", 8192, 8192, false, false, 1024),
        ("regular4k-repeat", 4096, 4096, false, false, 256),
    ] {
        renderer.set_render_experiments(RenderExperiments {
            original_world_materials: true,
            sun_shadows: true,
            far_shadows: true,
            shadow_distance: 320.0,
            shadow_resolution: base,
            shadow_detail_resolution: detail,
            wide_shadow_detail: wide,
            near_shadow_focus: focus,
            shadow_filter: 1.5,
            soft_shadows: false,
            dense_shadow_filter: true,
            original_shadows: false,
            clearcoat: true,
            car_skylight: true,
            lighting_debug: LightingDebug::Off,
            ..Default::default()
        });
        let mut samples = Vec::new();
        for index in 0..48 {
            renderer.render(&frame, &instances).unwrap();
            let image = renderer.read_output().unwrap().unwrap();
            let stats = renderer.experiment_stats();
            assert_eq!(stats.map_mib, mib, "actual map allocation");
            if index >= 12 {
                samples.push([
                    stats.gpu_frame_ms.expect("GPU timestamps required"),
                    stats.gpu_shadow_ms.expect("sun timestamps required"),
                    stats.gpu_contact_ms.unwrap(),
                    stats.gpu_ao_ms.unwrap(),
                    stats.shadow_cpu_ms,
                ]);
            }
            if index == 47
                && let Some(output) = &output
            {
                std::fs::create_dir_all(output).unwrap();
                save(&output.join(format!("shadow-{api:?}-{name}.png")), size, &image.rgba);
            }
        }
        assert!(samples.iter().flatten().all(|v| v.is_finite() && *v >= 0.0));
        let percentile = |column: usize, percent: usize| {
            let mut values: Vec<_> = samples.iter().map(|v| v[column]).collect();
            values.sort_by(f32::total_cmp);
            values[(values.len() - 1) * percent / 100]
        };
        eprintln!(
            "shadow-cost {api:?} {name}: GPU main p50 {:.3} ms p90 {:.3}, sun p50 {:.3}, geometry {:.3}, AO {:.3}, CPU sun {:.3}, {mib} MiB",
            percentile(0, 50),
            percentile(0, 90),
            percentile(1, 50),
            percentile(2, 50),
            percentile(3, 50),
            percentile(4, 50)
        );
        for (index, sample) in samples.iter().enumerate() {
            report.push_str(&format!(
                "{name},{mib},{index},{:.6},{:.6},{:.6},{:.6},{:.6}\n",
                sample[0], sample[1], sample[2], sample[3], sample[4]
            ));
        }
    }
    if let Some(output) = output {
        std::fs::write(output.join(format!("shadow-cost-{api:?}.csv")), report).unwrap();
    }
}

fn wait_ready(world: &mut WorldScene, renderer: &mut Renderer) {
    let start = Instant::now();
    while !world.ready() {
        world.update(renderer, &ActionState::default(), 0.0);
        assert!(start.elapsed() < Duration::from_secs(60), "streaming timeout");
        std::thread::sleep(Duration::from_millis(5));
    }
}
