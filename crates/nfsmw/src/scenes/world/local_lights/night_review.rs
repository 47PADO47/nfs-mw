//! Installed asset inventory and repeatable time/weather review captures.
use super::super::*;
use crate::viewer::Scene;
use blackbox_gfx::{RenderBackend, RenderExperiments};
use blackbox_render::{Renderer, RendererOptions};

#[test]
#[ignore = "requires installed scenery"]
fn inspect_night_review_assets() {
    use nfsmw_data::world::{WorldIndex, load_section};
    let dir = GameDir::open(std::env::var_os("NFSMW_GAME_DIR").unwrap()).unwrap();
    let index = WorldIndex::open(&dir, "L2RA").unwrap();
    let mut stream = dir.open_file(&index.stream_file).unwrap();
    let mut seen = std::collections::HashSet::new();
    for (i, section) in index.sections.iter().enumerate() {
        let Ok(data) = load_section(&mut stream, i, section) else { continue };
        for solid in &data.solids {
            let name = solid.name.to_ascii_uppercase();
            if !(name.starts_with("PAN_") || name.starts_with("XO_STREETLIGHT") || name.starts_with("XO_LIGHTPOST"))
                || !seen.insert(name.clone())
            {
                continue;
            }
            for (i, group) in solid.groups.iter().enumerate() {
                let Some(indices) =
                    solid.indices.get(group.first_index as usize..(group.first_index + group.num_indices) as usize)
                else {
                    continue;
                };
                let mut lo = Vec3::splat(f32::INFINITY);
                let mut hi = Vec3::splat(f32::NEG_INFINITY);
                for &idx in indices {
                    if let Some(v) = solid.vertices.get((u32::from(idx) + group.base_vertex) as usize) {
                        let p = Vec3::from_array(v.position);
                        lo = lo.min(p);
                        hi = hi.max(p);
                    }
                }
                eprintln!(
                    "{name} group={i} effect={} indices={} min={lo:?} max={hi:?}",
                    group.effect_id, group.num_indices
                );
            }
        }
    }
}

#[test]
#[ignore = "requires installed scenery"]
fn locate_dual_streetlights() {
    use nfsmw_data::world::{WorldIndex, load_section};
    let dir = GameDir::open(std::env::var_os("NFSMW_GAME_DIR").unwrap()).unwrap();
    let index = WorldIndex::open(&dir, "L2RA").unwrap();
    let mut stream = dir.open_file(&index.stream_file).unwrap();
    let mut found = Vec::new();
    for (i, section) in index.sections.iter().enumerate() {
        let Ok(data) = load_section(&mut stream, i, section) else { continue };
        for scenery in &data.scenery {
            for instance in &scenery.instances {
                let Some(info) = scenery.info_of(instance) else { continue };
                let name = info.name.to_ascii_uppercase();
                if !name.starts_with("XO_LIGHTPOSTB_") && !name.starts_with("XO_STREETLIGHTI_") {
                    continue;
                }
                let at = Vec3::from_array(instance.position);
                let distance = at.truncate().distance(glam::Vec2::new(1598.0, 2212.0));
                found.push((distance, name, at));
            }
        }
    }
    found.sort_by(|a, b| a.0.total_cmp(&b.0));
    for (distance, name, at) in found.iter().take(20) {
        eprintln!("dual {name} at {at:?} distance {distance:.1}");
    }
    assert!(!found.is_empty());
}

#[test]
#[ignore = "requires installed world and GPU"]
fn installed_dual_streetlights_emit_and_glow_at_night() {
    let dir = GameDir::open(std::env::var_os("NFSMW_GAME_DIR").unwrap()).unwrap();
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
            height: 8.0,
            heading: 0.0,
            pitch: 0.0,
            fog_distance: 2000.0,
            wait_for_load: true,
            drive: None,
        },
    )
    .unwrap();
    world.init(&mut renderer).unwrap();
    let start = std::time::Instant::now();
    while !world.ready() && start.elapsed() < std::time::Duration::from_secs(300) {
        world.update(&mut renderer, &ActionState::default(), 0.0);
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    assert!(world.ready());
    world.set_environment(crate::viewer::Environment { hour: 22.0, ..Default::default() });
    let base = RenderExperiments {
        original_world_materials: true,
        full_day_night: true,
        local_light_distance: 240.0,
        local_shadow_count: 0,
        ..Default::default()
    };
    let output = std::env::var_os("NFSMW_LAMP_OUT").map(std::path::PathBuf::from);
    let mut images = Vec::new();
    for (name, enabled, strength) in [("on", true, 1.5), ("off", false, 1.5), ("zero", true, 0.0)] {
        renderer.set_render_experiments(RenderExperiments {
            local_street_lights: enabled,
            local_street_strength: strength,
            ..base
        });
        world.update(&mut renderer, &ActionState::default(), 0.0);
        if enabled && strength > 0.0 {
            let fixtures: std::collections::HashMap<_, _> = world.residency.local_fixtures().collect();
            let both = world.residency.placed().any(|p| {
                let Some(fs) = p.lods.iter().flatten().find_map(|mesh| fixtures.get(mesh)) else { return false };
                fs.len() == 2
                    && fs.iter().all(|f| {
                        let at = p.transform.transform_point3(f.position);
                        world.street_glows.iter().any(|(glow, fade, _, _, _)| *fade > 0.0 && glow.distance(at) < 0.01)
                    })
            });
            assert!(both, "at least one installed dual fixture must keep both glow positions");
        } else {
            assert!(world.street_glows.is_empty());
        }
        for _ in 0..3 {
            let (frame, instances) = world.frame(size.0 as f32 / size.1 as f32);
            renderer.render(&frame, instances).unwrap();
        }
        let image = renderer.read_output().unwrap().unwrap();
        if let Some(out) = &output {
            std::fs::create_dir_all(out).unwrap();
            let file = std::fs::File::create(out.join(format!("dual-{name}.png"))).unwrap();
            let mut encoder = png::Encoder::new(file, size.0, size.1);
            encoder.set_color(png::ColorType::Rgba);
            encoder.set_depth(png::BitDepth::Eight);
            encoder.write_header().unwrap().write_image_data(&image.rgba).unwrap();
        }
        images.push(image.rgba);
    }
    assert_ne!(images[0], images[1], "streetlight toggle changes the scene");
}

#[test]
#[ignore = "requires installed world and GPU"]
fn installed_signal_poles_have_powered_lamps_at_night() {
    let dir = GameDir::open(std::env::var_os("NFSMW_GAME_DIR").unwrap()).unwrap();
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
            at: Some([1204.0, 2130.0]),
            height: 3.0,
            heading: 0.0,
            pitch: 0.0,
            fog_distance: 2000.0,
            wait_for_load: true,
            drive: None,
        },
    )
    .unwrap();
    world.init(&mut renderer).unwrap();
    let start = std::time::Instant::now();
    while !world.ready() && start.elapsed() < std::time::Duration::from_secs(300) {
        world.update(&mut renderer, &ActionState::default(), 0.0);
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    assert!(world.ready());
    world.set_environment(crate::viewer::Environment { hour: 22.0, ..Default::default() });
    renderer.set_render_experiments(RenderExperiments {
        full_day_night: true,
        local_street_lights: true,
        local_street_glow: 1.0,
        local_light_distance: 240.0,
        local_shadow_count: 0,
        ..Default::default()
    });
    world.update(&mut renderer, &ActionState::default(), 0.0);
    let fixtures: std::collections::HashMap<_, _> = world.residency.local_fixtures().collect();
    let signal_at = world
        .residency
        .cull_tiles()
        .flat_map(|tile| tile.placed.iter())
        .find_map(|placed| {
            let signal = placed.lods.iter().flatten().find_map(|mesh| fixtures.get(mesh))?;
            let fixture = signal
                .iter()
                .find(|f| f.street && (8.2..8.6).contains(&f.position.z) && (-3.1..-2.5).contains(&f.position.y))?;
            let (transform, _) = super::super::visibility::current_pose(placed, world.residency.props())?;
            let at = transform.transform_point3(fixture.position);
            (at.truncate().distance(glam::Vec2::new(1204.0, 2130.0)) < 50.0).then_some(at)
        })
        .expect("a nearby signal pole with a lamp is resident");
    assert!(
        world.street_glows.iter().any(|(at, intensity, _, _, _)| *intensity > 0.0 && at.distance(signal_at) < 0.02)
    );
    // View the actual fixture from below. Only the bulb glow changes between captures.
    world.camera.position = signal_at + Vec3::new(8.0, 0.0, -3.0);
    let toward = (signal_at - world.camera.position).normalize();
    world.camera.yaw = toward.y.atan2(toward.x);
    world.camera.pitch = toward.z.asin();
    let mut images = Vec::new();
    for (name, strength) in [("off", 0.0), ("on", 1.0)] {
        let options = renderer.render_experiments();
        renderer.set_render_experiments(RenderExperiments { local_street_glow: strength, ..options });
        world.update(&mut renderer, &ActionState::default(), 0.0);
        for _ in 0..3 {
            let (frame, instances) = world.frame(size.0 as f32 / size.1 as f32);
            renderer.render(&frame, instances).unwrap();
        }
        let image = renderer.read_output().unwrap().unwrap();
        if let Some(out) = std::env::var_os("NFSMW_LAMP_OUT") {
            let out = std::path::PathBuf::from(out);
            std::fs::create_dir_all(&out).unwrap();
            let file = std::fs::File::create(out.join(format!("signal-{name}.png"))).unwrap();
            let mut encoder = png::Encoder::new(file, size.0, size.1);
            encoder.set_color(png::ColorType::Rgba);
            encoder.set_depth(png::BitDepth::Eight);
            encoder.write_header().unwrap().write_image_data(&image.rgba).unwrap();
        }
        images.push(image);
    }
    let mut brightest_gain = 0;
    for y in (size.1 / 2 - 30)..(size.1 / 2 + 30) {
        for x in (size.0 / 2 - 30)..(size.0 / 2 + 30) {
            let before = images[0].pixel(x, y).unwrap();
            let after = images[1].pixel(x, y).unwrap();
            brightest_gain = brightest_gain.max(i32::from(after[0]) - i32::from(before[0]));
        }
    }
    assert!(brightest_gain >= 20, "the visible bulb should brighten without changing its light: {brightest_gain}");

    // A side view checks the separate light-in-air effect without changing the lamp's surface illumination.
    world.camera.position = signal_at + Vec3::new(12.0, 0.0, -3.0);
    let toward = (signal_at - Vec3::Z * 3.0 - world.camera.position).normalize();
    world.camera.yaw = toward.y.atan2(toward.x);
    world.camera.pitch = toward.z.asin();
    world.set_environment(crate::viewer::Environment {
        hour: 22.0,
        cloud_override: Some(1.0),
        settle_clouds: true,
        ..Default::default()
    });
    let mut haze_images = Vec::new();
    for (name, strength) in [("haze-off", 0.0), ("haze-on", 2.0)] {
        let options = renderer.render_experiments();
        renderer.set_render_experiments(RenderExperiments { local_light_fog: strength, ..options });
        world.update(&mut renderer, &ActionState::default(), 0.0);
        for _ in 0..3 {
            let (frame, instances) = world.frame(size.0 as f32 / size.1 as f32);
            renderer.render(&frame, instances).unwrap();
        }
        let image = renderer.read_output().unwrap().unwrap();
        if let Some(out) = std::env::var_os("NFSMW_LAMP_OUT") {
            let out = std::path::PathBuf::from(out);
            std::fs::create_dir_all(&out).unwrap();
            let file = std::fs::File::create(out.join(format!("signal-{name}.png"))).unwrap();
            let mut encoder = png::Encoder::new(file, size.0, size.1);
            encoder.set_color(png::ColorType::Rgba);
            encoder.set_depth(png::BitDepth::Eight);
            encoder.write_header().unwrap().write_image_data(&image.rgba).unwrap();
        }
        haze_images.push(image.rgba);
    }
    assert_ne!(haze_images[0], haze_images[1], "haze must contribute in the installed scene");
}

#[test]
#[ignore = "requires installed world and GPU"]
fn installed_night_weather_review() {
    let dir = GameDir::open(std::env::var_os("NFSMW_GAME_DIR").unwrap()).unwrap();
    let out = std::path::PathBuf::from(std::env::var_os("NFSMW_LAMP_OUT").unwrap());
    std::fs::create_dir_all(&out).unwrap();
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
    for (location, at, heading) in [("city", [1772.0, 2471.0], 0.0), ("distant", [2492.0, 1116.0], 0.0)] {
        let mut world = WorldScene::open(
            &dir,
            Options {
                at: Some(at),
                height: 3.0,
                heading,
                pitch: -8.0,
                fog_distance: 2000.0,
                wait_for_load: true,
                drive: Some(DriveOptions { car: "BMWM3GTR".into(), script: None }),
            },
        )
        .unwrap();
        world.set_car_shading(crate::settings::CarShading::GlossyLamps);
        world.init(&mut renderer).unwrap();
        let start = std::time::Instant::now();
        while !world.ready() && start.elapsed() < std::time::Duration::from_secs(300) {
            world.update(&mut renderer, &ActionState::default(), 0.0);
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        assert!(world.ready());
        renderer.set_render_experiments(RenderExperiments {
            original_world_materials: true,
            full_day_night: true,
            auto_player_lights: true,
            local_lamps: true,
            local_street_lights: true,
            local_headlights: true,
            local_traffic_headlights: true,
            local_tail_lights: true,
            local_light_limit: 128,
            local_light_distance: 240.0,
            ..Default::default()
        });
        for _ in 0..360 {
            world.update(&mut renderer, &ActionState::default(), 1.0 / 60.0);
        }
        eprintln!("{location}: {} traffic cars", world.traffic.as_ref().map_or(0, |t| t.net_cars().count()));
        for (name, hour, cloud) in [
            ("noon-clear", 12.0, 0.0),
            ("dusk-clear", 18.0, 0.0),
            ("night-clear", 22.0, 0.0),
            ("night-cloudy", 22.0, 1.0),
            ("dawn-cloudy", 5.5, 1.0),
        ] {
            world.set_environment(crate::viewer::Environment {
                hour,
                cloud_override: Some(cloud),
                settle_clouds: true,
                ..Default::default()
            });
            world.update(&mut renderer, &ActionState::default(), 0.0);
            for _ in 0..3 {
                let (frame, instances) = world.frame(size.0 as f32 / size.1 as f32);
                renderer.render(&frame, instances).unwrap();
            }
            let image = renderer.read_output().unwrap().unwrap();
            let file = std::fs::File::create(out.join(format!("review-{location}-{name}.png"))).unwrap();
            let mut encoder = png::Encoder::new(file, size.0, size.1);
            encoder.set_color(png::ColorType::Rgba);
            encoder.set_depth(png::BitDepth::Eight);
            encoder.write_header().unwrap().write_image_data(&image.rgba).unwrap();
        }
        world.close(&mut renderer);
    }
}

#[test]
#[ignore = "requires installed car, world and GPU"]
fn installed_lookback_headlamp_review() {
    let dir = GameDir::open(std::env::var_os("NFSMW_GAME_DIR").unwrap()).unwrap();
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
            heading: 0.0,
            pitch: -8.0,
            fog_distance: 2000.0,
            wait_for_load: true,
            drive: Some(DriveOptions { car: "BMWM3GTR".into(), script: None }),
        },
    )
    .unwrap();
    world.set_car_shading(crate::settings::CarShading::GlossyLamps);
    world.init(&mut renderer).unwrap();
    let start = std::time::Instant::now();
    while !world.ready() && start.elapsed() < std::time::Duration::from_secs(300) {
        world.update(&mut renderer, &ActionState::default(), 0.0);
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    assert!(world.ready());
    renderer.set_render_experiments(RenderExperiments {
        full_day_night: true,
        auto_player_lights: false,
        local_headlights: true,
        ..Default::default()
    });
    world.set_environment(crate::viewer::Environment { hour: 22.0, ..Default::default() });
    world.traffic = None;
    world.players.lead_mut().unwrap().headlights = true;
    let mut input = ActionState::default();
    input.update_menu_buttons(&[crate::input::Action::LookBack]);
    for _ in 0..120 {
        world.update(&mut renderer, &input, 1.0 / 60.0);
    }
    for _ in 0..3 {
        let (frame, instances) = world.frame(size.0 as f32 / size.1 as f32);
        renderer.render(&frame, instances).unwrap();
    }
    let image = renderer.read_output().unwrap().unwrap();
    let out = std::path::PathBuf::from(std::env::var_os("NFSMW_LAMP_OUT").unwrap());
    std::fs::create_dir_all(&out).unwrap();
    let file = std::fs::File::create(out.join("lookback-headlights.png")).unwrap();
    let mut encoder = png::Encoder::new(file, size.0, size.1);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder.write_header().unwrap().write_image_data(&image.rgba).unwrap();
    assert!(world.players.lead().unwrap().headlights);
}
