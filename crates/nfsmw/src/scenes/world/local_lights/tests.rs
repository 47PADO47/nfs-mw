//! Installed fixed-view comparison; GPU timing includes local shadow passes and scene sampling.
use super::super::*;
use crate::viewer::Scene;
use blackbox_gfx::{RenderBackend, RenderExperiments};
use blackbox_render::{Renderer, RendererOptions};
#[test]
#[ignore = "requires installed game and GPU; local lamp capture and timing"]
fn installed_tunnel_local_light_capture_and_timing() {
    let dir = GameDir::open(std::env::var_os("NFSMW_GAME_DIR").expect("set NFSMW_GAME_DIR")).unwrap();
    let api = match std::env::var("BLACKBOX_TEST_API").as_deref() {
        Ok("dx12") => blackbox_gfx::GraphicsApi::Dx12,
        _ => blackbox_gfx::GraphicsApi::Vulkan,
    };
    let output = std::env::var_os("NFSMW_LAMP_OUT").map(std::path::PathBuf::from);
    let size = (1280, 720);
    let mut renderer = Renderer::headless(
        size,
        RendererOptions { backend: api, vsync: false, force_fallback_adapter: false, pipeline_cache_dir: None },
    )
    .unwrap();
    let mut world = WorldScene::open(
        &dir,
        Options {
            at: Some([337.0, 1896.0]),
            height: 3.0,
            heading: 150.0,
            pitch: -8.0,
            fog_distance: 2000.0,
            wait_for_load: true,
            drive: Some(DriveOptions { car: "ELISE".into(), script: None }),
        },
    )
    .unwrap();
    world.init(&mut renderer).unwrap();
    wait_ready(&mut world, &mut renderer);
    world.traffic = None;
    world.players.lead_mut().expect("player").headlights = false;
    renderer.set_render_experiments(RenderExperiments { experimental_night: true, ..Default::default() });
    world.update(&mut renderer, &ActionState::default(), 0.0);
    assert!(world.players.lead().expect("player").headlights, "night activates the headlight switch");
    renderer.set_render_experiments(RenderExperiments::default());
    world.update(&mut renderer, &ActionState::default(), 0.0);
    assert!(!world.players.lead().expect("player").headlights, "day restores the previous headlight switch");
    world.players.lead_mut().expect("player").headlights = true;
    let mut originals = None;
    let mut report = String::from("mode,sample,gpu_frame_ms,cpu_world_update_ms,cpu_render_ms\n");
    for (name, lamps, heads, shadows, resolution, night) in [
        ("off", false, false, 0, 1024, false),
        ("tunnel-unshadowed", true, false, 0, 1024, false),
        ("both-4-1024", true, true, 4, 1024, false),
        ("both-8-1024", true, true, 8, 1024, false),
        ("both-16-1024", true, true, 16, 1024, false),
        ("both-8-2048", true, true, 8, 2048, false),
        ("night", true, true, 8, 1024, true),
        ("range-64", true, true, 8, 1024, true),
        ("range-128", true, true, 8, 1024, true),
    ] {
        renderer.set_render_experiments(RenderExperiments {
            original_world_materials: true,
            local_lamps: lamps,
            local_headlights: heads,
            local_light_distance: match name.starts_with("range-") {
                true => 240.0,
                false => 160.0,
            },
            local_light_limit: match name == "range-64" {
                true => 64,
                false => 128,
            },
            local_shadow_count: shadows,
            local_shadow_resolution: resolution,
            experimental_night: night,
            ..Default::default()
        });
        let mut samples = Vec::new();
        for sample in 0..28 {
            let start = std::time::Instant::now();
            world.update(&mut renderer, &ActionState::default(), 0.0);
            let cpu = start.elapsed().as_secs_f64() * 1000.0;
            let (frame, instances) = world.frame(size.0 as f32 / size.1 as f32);
            renderer.render(&frame, instances).unwrap();
            let image = renderer.read_output().unwrap().unwrap();
            if sample >= 8 {
                let gpu = renderer.experiment_stats().gpu_frame_ms.expect("GPU timestamps required");
                assert!(gpu.is_finite() && gpu > 0.0);
                samples.push(gpu);
                report.push_str(&format!(
                    "{name},{sample},{gpu:.6},{cpu:.6},{:.6}\n",
                    renderer.frame_performance().cpu_render_ms
                ));
            }
            if sample == 27 {
                match &originals {
                    None => originals = Some(image.rgba.clone()),
                    Some(off) => assert_ne!(off, &image.rgba, "enabled lights change the installed scene"),
                }
                if let Some(output) = &output {
                    std::fs::create_dir_all(output).unwrap();
                    let file = std::fs::File::create(output.join(format!("{name}-{api:?}.png"))).unwrap();
                    let mut encoder = png::Encoder::new(file, size.0, size.1);
                    encoder.set_color(png::ColorType::Rgba);
                    encoder.set_depth(png::BitDepth::Eight);
                    encoder.write_header().unwrap().write_image_data(&image.rgba).unwrap();
                }
            }
        }
        samples.sort_by(f32::total_cmp);
        eprintln!(
            "local-lamp-cost {api:?} {name}: GPU p50 {:.3} ms, p90 {:.3} ms",
            samples[samples.len() / 2],
            samples[(samples.len() - 1) * 9 / 10]
        );
    }
    if let Some(output) = output {
        std::fs::write(output.join(format!("timings-{api:?}.csv")), report).unwrap();
    }
}

fn wait_ready(world: &mut WorldScene, renderer: &mut Renderer) {
    let start = std::time::Instant::now();
    while !world.ready() && start.elapsed() < std::time::Duration::from_secs(300) {
        world.update(renderer, &ActionState::default(), 0.0);
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    assert!(world.ready(), "tunnel must finish streaming before comparison");
}

#[test]
#[ignore = "requires installed game and GPU; shared fog capture and timing"]
fn installed_tunnel_shared_fog_capture_and_timing() {
    let dir = GameDir::open(std::env::var_os("NFSMW_GAME_DIR").expect("set NFSMW_GAME_DIR")).unwrap();
    let size = (1280, 720);
    let mut renderer = Renderer::headless(
        size,
        RendererOptions {
            backend: blackbox_gfx::GraphicsApi::Vulkan,
            vsync: false,
            force_fallback_adapter: false,
            pipeline_cache_dir: None,
        },
    )
    .unwrap();
    let mut world = WorldScene::open(
        &dir,
        Options {
            at: Some([337.0, 1896.0]),
            height: 3.0,
            heading: 150.0,
            pitch: -8.0,
            fog_distance: 2000.0,
            wait_for_load: true,
            drive: Some(DriveOptions { car: "ELISE".into(), script: None }),
        },
    )
    .unwrap();
    world.init(&mut renderer).unwrap();
    wait_ready(&mut world, &mut renderer);
    world.set_environment(crate::viewer::Environment {
        hour: 22.0,
        cloud_override: Some(1.0),
        settle_clouds: true,
        ..Default::default()
    });
    let mut reference = None;
    for (name, strength, quality) in [("off", 0.0, 2), ("low", 0.4, 1), ("medium", 0.4, 2), ("high", 0.4, 3)] {
        renderer.set_render_experiments(RenderExperiments {
            full_day_night: true,
            local_lamps: true,
            local_light_distance: 280.0,
            local_light_limit: 128,
            local_light_strength: 1.8,
            local_shadow_count: 16,
            local_shadow_distance: 140.0,
            local_shadow_resolution: 2048,
            local_light_fog: strength,
            local_light_fog_distance: 460.0,
            local_light_fog_count: 64,
            local_light_fog_quality: quality,
            ..Default::default()
        });
        let mut timings = Vec::new();
        for sample in 0..16 {
            world.update(&mut renderer, &ActionState::default(), 0.0);
            let (frame, instances) = world.frame(size.0 as f32 / size.1 as f32);
            renderer.render(&frame, instances).unwrap();
            let image = renderer.read_output().unwrap().unwrap();
            if sample >= 6 {
                timings.push(renderer.experiment_stats().gpu_frame_ms.expect("GPU timestamps"));
            }
            if sample == 15 {
                match &reference {
                    None => reference = Some(image.rgba.clone()),
                    Some(off) => assert_ne!(off, &image.rgba, "fog changes the tunnel"),
                }
                if let Some(out) = std::env::var_os("NFSMW_LAMP_OUT") {
                    let out = std::path::PathBuf::from(out);
                    std::fs::create_dir_all(&out).unwrap();
                    let file = std::fs::File::create(out.join(format!("volume-{name}.png"))).unwrap();
                    let mut encoder = png::Encoder::new(file, size.0, size.1);
                    encoder.set_color(png::ColorType::Rgba);
                    encoder.set_depth(png::BitDepth::Eight);
                    encoder.write_header().unwrap().write_image_data(&image.rgba).unwrap();
                }
            }
        }
        timings.sort_by(f32::total_cmp);
        eprintln!(
            "shared fog {name}: GPU p50 {:.3} ms, p90 {:.3} ms",
            timings[timings.len() / 2],
            timings[(timings.len() - 1) * 9 / 10]
        );
    }
}

#[test]
#[ignore = "requires installed tunnel fixtures"]
fn installed_tunnel_profiles_split_combined_meshes_into_individual_emitters() {
    use nfsmw_data::world::{WorldIndex, load_section};
    let dir = GameDir::open(std::env::var_os("NFSMW_GAME_DIR").expect("install")).unwrap();
    let index = WorldIndex::open(&dir, "L2RA").unwrap();
    let mut stream = dir.open_file(&index.stream_file).unwrap();
    let mut seen = std::collections::BTreeSet::new();
    let mut combined = 0;
    for (i, section) in index.sections.iter().enumerate() {
        let Ok(data) = load_section(&mut stream, i, section) else {
            continue;
        };
        for solid in &data.solids {
            let fixtures = super::Fixture::read(solid);
            if fixtures.is_empty() || !seen.insert(solid.name.clone()) {
                continue;
            }
            for (i, fixture) in fixtures.iter().enumerate() {
                assert!(fixture.position.is_finite() && fixture.direction.is_normalized());
                assert!(fixtures[..i].iter().all(|other| other.position.distance(fixture.position) > 0.1));
            }
            if solid.name.contains("_DEINST") {
                assert!(fixtures.len() > 1, "combined lamps must not collapse to one midpoint");
                combined += 1;
            }
            eprintln!("tunnel emitters {}: {}", solid.name, fixtures.len());
        }
    }
    assert!(seen.len() >= 9 && combined >= 4, "all installed tunnel families: {seen:?}");
}

#[test]
#[ignore = "requires installed streetlight meshes"]
fn installed_streetlight_profiles_match_lens_geometry() {
    use nfsmw_data::world::{WorldIndex, load_section};
    let dir = GameDir::open(std::env::var_os("NFSMW_GAME_DIR").expect("install")).unwrap();
    let index = WorldIndex::open(&dir, "L2RA").unwrap();
    let mut stream = dir.open_file(&index.stream_file).unwrap();
    let mut seen = std::collections::BTreeSet::new();
    for (i, section) in index.sections.iter().enumerate() {
        let Ok(data) = load_section(&mut stream, i, section) else { continue };
        for solid in &data.solids {
            let name = solid.name.to_ascii_uppercase();
            if ![
                "XO_STREETLIGHTC_1B_00",
                "XO_STREETLIGHTC_1C_00",
                "XO_STREETLIGHTCB_1B_00",
                "XO_STREETLIGHTCN_1B_00",
                "XO_STREETLIGHTCC_1B_00",
                "XO_STREETLIGHTD_1B_00",
                "XO_STREETLIGHTD2_1B_00",
                "XO_STREETLIGHT_ITALYJG_1B_00",
                "XO_STREETLIGHT_ITALYJG_1C_00",
                "XO_STREETLIGHT_ITALYJG_2B_00",
                "XO_STREETLIGHT_ITALYJG_2C_00",
                "XO_LIGHTPOSTA_1A_00",
                "XO_STREETLIGHTDRIFT_1B_00",
                "XO_STREETLIGHTDRIFT_1C_00",
                "XO_LIGHTPOSTB_1A_00",
                "XO_LIGHTPOSTB_1C_00",
                "XO_LIGHTPOSTB_1Z_00",
                "XO_LIGHTPOSTB_WALL_1B_00",
                "XO_STREETLIGHTI_1B_00",
                "XO_STREETLIGHTI_1C_00",
                "XO_STREETLIGHTI_1Z_00",
                "XO_TRAFFICLIGHTA_1B_00",
                "XO_TRAFFICLIGHTA_1C_00",
                "XO_TRAFFICLIGHTA_1Z_00",
                "XO_TRAFFICLIGHTB_1B_00",
                "XO_TRAFFICLIGHTB_1C_00",
                "XO_TRAFFICLIGHTB_1Z_00",
            ]
            .contains(&name.as_str())
                || !seen.insert(name.clone())
            {
                continue;
            }
            let fixtures = super::Fixture::read(solid);
            let dual = name.starts_with("XO_LIGHTPOSTB_") || name.starts_with("XO_STREETLIGHTI_");
            assert_eq!(fixtures.len(), if dual { 2 } else { 1 }, "{name} lens count");
            for fixture in &fixtures {
                assert!(fixture.street && fixture.direction == Vec3::NEG_Z);
                assert!(fixture.position.z > 7.0, "{name} light must be near the pole top");
            }
            if dual {
                assert!(
                    fixtures[0].position.y < -1.0 && fixtures[1].position.y > 1.0,
                    "{name} needs one light on each arm"
                );
            }
        }
    }
    assert_eq!(seen.len(), 27, "all supported streetlight meshes");
}

#[test]
#[ignore = "requires installed game and GPU; full day/night captures"]
fn installed_full_cycle_tracks_time_and_player_lights() {
    let dir = GameDir::open(std::env::var_os("NFSMW_GAME_DIR").expect("install")).unwrap();
    let size = (1280, 720);
    let mut renderer = Renderer::headless(
        size,
        RendererOptions {
            backend: blackbox_gfx::GraphicsApi::Vulkan,
            vsync: false,
            force_fallback_adapter: false,
            pipeline_cache_dir: None,
        },
    )
    .unwrap();
    let mut world = WorldScene::open(
        &dir,
        Options {
            at: Some([1598.0, 2212.0]),
            height: 3.0,
            heading: 45.0,
            pitch: -8.0,
            fog_distance: 2000.0,
            wait_for_load: true,
            drive: Some(DriveOptions { car: "BMWM3GTR".into(), script: None }),
        },
    )
    .unwrap();
    world.set_car_shading(crate::settings::CarShading::GlossyLamps);
    world.init(&mut renderer).unwrap();
    wait_ready(&mut world, &mut renderer);
    world.traffic = None;
    renderer.set_render_experiments(RenderExperiments {
        original_world_materials: true,
        full_day_night: true,
        auto_player_lights: true,
        local_headlights: true,
        local_tail_lights: true,
        ..Default::default()
    });
    let output = std::env::var_os("NFSMW_LAMP_OUT").map(std::path::PathBuf::from);
    let mut images = Vec::new();
    for (name, hour, headlights) in [
        ("noon", 12.0, false),
        ("sunset", 18.0, false),
        ("night", 22.0, true),
        ("midnight", 0.0, true),
        ("dawn", 5.5, true),
        ("morning", 7.0, false),
    ] {
        world.set_environment(crate::viewer::Environment { hour, ..Default::default() });
        world.update(&mut renderer, &ActionState::default(), 0.0);
        assert_eq!(world.players.lead().unwrap().headlights, headlights, "{name} automatic lights");
        for _ in 0..3 {
            let (frame, instances) = world.frame(size.0 as f32 / size.1 as f32);
            renderer.render(&frame, instances).unwrap();
        }
        let image = renderer.read_output().unwrap().unwrap();
        if let Some(output) = &output {
            std::fs::create_dir_all(output).unwrap();
            let file = std::fs::File::create(output.join(format!("cycle-{name}.png"))).unwrap();
            let mut encoder = png::Encoder::new(file, size.0, size.1);
            encoder.set_color(png::ColorType::Rgba);
            encoder.set_depth(png::BitDepth::Eight);
            encoder.write_header().unwrap().write_image_data(&image.rgba).unwrap();
        }
        images.push(image.rgba);
    }
    assert_ne!(images[0], images[2], "night changes the installed scene");
    world.players.lead_mut().unwrap().headlights = true;
    world.update(&mut renderer, &ActionState::default(), 0.0);
    assert!(world.players.lead().unwrap().headlights, "manual override remains between phase changes");
}

#[test]
#[ignore = "requires installed game and GPU; streetlight fixture comparison"]
fn installed_streetlights_add_night_lighting_near_gray_point() {
    let dir = GameDir::open(std::env::var_os("NFSMW_GAME_DIR").expect("install")).unwrap();
    let size = (1280, 720);
    let mut renderer = Renderer::headless(
        size,
        RendererOptions {
            backend: blackbox_gfx::GraphicsApi::Vulkan,
            vsync: false,
            force_fallback_adapter: false,
            pipeline_cache_dir: None,
        },
    )
    .unwrap();
    let mut world = WorldScene::open(
        &dir,
        Options {
            at: Some([2960.0, 3002.0]),
            height: 8.0,
            heading: 0.0,
            pitch: -10.0,
            fog_distance: 2000.0,
            wait_for_load: true,
            drive: None,
        },
    )
    .unwrap();
    world.init(&mut renderer).unwrap();
    wait_ready(&mut world, &mut renderer);
    world.traffic = None;
    world.set_environment(crate::viewer::Environment { hour: 22.0, ..Default::default() });
    let base = RenderExperiments {
        original_world_materials: true,
        full_day_night: true,
        local_shadow_count: 0,
        ..Default::default()
    };
    let mut images = Vec::new();
    for (name, enabled) in [("on", true), ("off", false)] {
        renderer.set_render_experiments(RenderExperiments { local_street_lights: enabled, ..base });
        world.update(&mut renderer, &ActionState::default(), 0.0);
        assert_eq!(world.street_glows.is_empty(), !enabled, "street glow follows the feature switch");
        let fixture_map: std::collections::HashMap<_, _> = world.residency.local_fixtures().collect();
        let mut distances = Vec::new();
        for tile in world.residency.cull_tiles() {
            for placed in tile.placed {
                if let Some(fs) = placed.lods.iter().flatten().find_map(|mesh| fixture_map.get(mesh)) {
                    for fixture in fs.iter().filter(|fixture| fixture.street) {
                        distances.push(
                            placed
                                .transform
                                .transform_point3(fixture.position)
                                .distance(world.gameplay_lens(size.0 as f32 / size.1 as f32).position),
                        );
                    }
                }
            }
        }
        distances.sort_by(f32::total_cmp);
        eprintln!(
            "{name}: {} resident street fixtures, nearest {:?}",
            distances.len(),
            distances.iter().take(8).collect::<Vec<_>>()
        );
        for _ in 0..3 {
            let (frame, instances) = world.frame(size.0 as f32 / size.1 as f32);
            renderer.render(&frame, instances).unwrap();
        }
        let image = renderer.read_output().unwrap().unwrap();
        if let Some(output) = std::env::var_os("NFSMW_LAMP_OUT") {
            let file =
                std::fs::File::create(std::path::PathBuf::from(output).join(format!("street-{name}.png"))).unwrap();
            let mut encoder = png::Encoder::new(file, size.0, size.1);
            encoder.set_color(png::ColorType::Rgba);
            encoder.set_depth(png::BitDepth::Eight);
            encoder.write_header().unwrap().write_image_data(&image.rgba).unwrap();
        }
        images.push(image.rgba);
    }
    let lit = images[0]
        .as_chunks::<4>()
        .0
        .iter()
        .zip(images[1].as_chunks::<4>().0)
        .filter(|(a, b)| a[0] > b[0].saturating_add(4))
        .count();
    assert!(lit > 25, "recognized streetlights must illuminate the installed scene: {lit}");
}
