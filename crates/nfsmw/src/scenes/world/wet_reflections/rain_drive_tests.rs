//! Installed driving coverage for the rain-to-ground-particle feed.
use super::super::{DriveOptions, Options, WorldScene};
use crate::{
    app::test_backend::NullBackend,
    input::{ActionState, Bindings},
    viewer::Scene,
};
use std::time::{Duration, Instant};

#[test]
#[ignore = "requires installed highway and a Vulkan GPU"]
fn installed_highway_wet_road_changes_pixels_across_sections() {
    check_road_sections(&[[-130.0, 4089.0], [-78.0, 4108.0]], false);
}

#[test]
#[ignore = "known junction capture occlusion; original-game comparison pending"]
fn installed_junction_reflection_occlusion_reproduction() {
    check_road_sections(&[[2191.0, 2515.0]], false);
}

#[test]
#[ignore = "requires installed highway and a GPU"]
fn installed_enhanced_wet_material_changes_highway_pixels() {
    check_road_sections(&[[-130.0, 4089.0], [-527.0, 2507.0], [1749.0, 1954.0], [2363.0, 3125.0]], true);
}

fn check_road_sections(points: &[[f32; 2]], enhanced: bool) {
    use blackbox_gfx::{RenderBackend, RenderExperiments};
    let dir = game_install::GameDir::open(std::env::var_os("NFSMW_GAME_DIR").unwrap()).unwrap();
    let size = std::env::var("NFSMW_SURFACE_CAPTURE_SIZE")
        .ok()
        .map(|s| {
            let (w, h) = s.split_once('x').expect("capture size WIDTHxHEIGHT");
            (w.parse::<u32>().unwrap(), h.parse::<u32>().unwrap())
        })
        .unwrap_or((960, 540));
    let mut renderer = blackbox_render::Renderer::headless(
        size,
        blackbox_render::RendererOptions {
            backend: blackbox_gfx::GraphicsApi::Vulkan,
            vsync: false,
            force_fallback_adapter: false,
            pipeline_cache_dir: None,
        },
    )
    .unwrap();
    renderer.set_render_experiments(RenderExperiments {
        original_world_materials: true,
        enhanced_sky: true,
        enhanced_wet_surfaces: enhanced,
        ..Default::default()
    });
    for &at in points {
        let mut world = WorldScene::open(
            &dir,
            Options {
                at: Some(at),
                height: 3.0,
                // This shoulder faces the embankment at 20 degrees; look back across the road.
                heading: if at[0] == 2363.0 { 200.0 } else { 20.0 },
                pitch: -12.0,
                fog_distance: 1500.0,
                wait_for_load: true,
                drive: None,
            },
        )
        .unwrap();
        world.init(&mut renderer).unwrap();
        let start = Instant::now();
        while !world.ready() {
            world.update(&mut renderer, &ActionState::default(), 0.0);
            assert!(start.elapsed() < Duration::from_secs(60));
            std::thread::sleep(Duration::from_millis(5));
        }
        let mut images = Vec::new();
        for amount in [0.0, 1.0] {
            world.wet_road.override_amount = Some(amount);
            world.prepare_frame(&mut renderer, 16.0 / 9.0);
            if amount > 0.0 && !enhanced {
                let image = renderer.read_projected_scene().unwrap().unwrap();
                let lower = &image.rgba[(image.width * (image.height / 2) * 4) as usize..];
                let sky = lower
                    .as_chunks::<4>()
                    .0
                    .iter()
                    .filter(|p| p[2] > p[0].saturating_add(20) && p[2] > p[1].saturating_add(10))
                    .count();
                if let Some(folder) = std::env::var_os("NFSMW_SURFACE_CAPTURE_DIR") {
                    let path = std::path::PathBuf::from(folder).join(format!("projected-{enhanced}-{}.png", at[0]));
                    let mut encoder =
                        png::Encoder::new(std::fs::File::create(path).unwrap(), image.width, image.height);
                    encoder.set_color(png::ColorType::Rgba);
                    encoder.set_depth(png::BitDepth::Eight);
                    encoder.write_header().unwrap().write_image_data(&image.rgba).unwrap();
                }
                assert!(
                    sky > (image.width * image.height / 10) as usize,
                    "ordinary road must not obscure the reflected sky: {sky}"
                );
            }
            let (frame, instances) = world.frame(16.0 / 9.0);
            renderer.render(&frame, instances).unwrap();
            let image = renderer.read_output().unwrap().unwrap();
            if std::env::var_os("NFSMW_SURFACE_CAPTURE_SIZE").is_some() {
                let mut samples = Vec::new();
                for i in 0..20 {
                    renderer.render(&frame, instances).unwrap();
                    renderer.read_output().unwrap().unwrap();
                    if i >= 4
                        && let Some(ms) = renderer.experiment_stats().gpu_frame_ms
                    {
                        samples.push(ms);
                    }
                }
                samples.sort_by(f32::total_cmp);
                eprintln!(
                    "road GPU {size:?} {at:?} enhanced={enhanced} water={amount}: median {:?} ms ({} samples)",
                    samples.get(samples.len() / 2),
                    samples.len()
                );
            }
            if let Some(folder) = std::env::var_os("NFSMW_SURFACE_CAPTURE_DIR") {
                let path = std::path::PathBuf::from(folder).join(format!("highway-{enhanced}-{}-{amount}.png", at[0]));
                let mut encoder = png::Encoder::new(std::fs::File::create(path).unwrap(), image.width, image.height);
                encoder.set_color(png::ColorType::Rgba);
                encoder.set_depth(png::BitDepth::Eight);
                encoder.write_header().unwrap().write_image_data(&image.rgba).unwrap();
            }
            images.push(image);
        }
        let changed = images[0]
            .rgba
            .as_chunks::<4>()
            .0
            .iter()
            .zip(images[1].rgba.as_chunks::<4>().0)
            .filter(|(a, b)| (0..3).any(|c| a[c].abs_diff(b[c]) > 2))
            .count();
        eprintln!("highway {at:?}: {changed} pixels respond to wetness");
        assert!(changed > 100, "wet roads must respond in both highway sections");
        world.close(&mut renderer);
    }
}

#[test]
#[ignore = "requires installed highway, water textures and a Vulkan GPU"]
fn installed_highway_tire_spray_changes_visible_pixels() {
    let dir = game_install::GameDir::open(std::env::var_os("NFSMW_GAME_DIR").unwrap()).unwrap();
    let mut renderer = blackbox_render::Renderer::headless(
        (960, 540),
        blackbox_render::RendererOptions {
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
            at: Some([-130.0, 4089.0]),
            height: 3.0,
            heading: 20.0,
            pitch: -12.0,
            fog_distance: 1500.0,
            wait_for_load: true,
            drive: Some(DriveOptions { car: "ELISE".into(), script: None }),
        },
    )
    .unwrap();
    world.init(&mut renderer).unwrap();
    let start = Instant::now();
    while !world.ready() {
        world.update(&mut renderer, &ActionState::default(), 0.0);
        assert!(start.elapsed() < Duration::from_secs(60));
        std::thread::sleep(Duration::from_millis(5));
    }
    world.rain.configure(true, 1.0);
    let mut throttle = ActionState::default();
    throttle.update_for_test(&Bindings::default(), &[bevy_input::keyboard::KeyCode::ArrowUp], &[], false);
    for _ in 0..180 {
        world.update(&mut renderer, &throttle, 1.0 / 60.0);
    }
    let mut images = Vec::new();
    for clear in [false, true] {
        if clear {
            world.players.lead_mut().unwrap().vehicle_effects.clear();
        }
        world.prepare_frame(&mut renderer, 16.0 / 9.0);
        world.upload_effects(&mut renderer);
        let (frame, instances) = world.frame(16.0 / 9.0);
        renderer.render(&frame, instances).unwrap();
        let image = renderer.read_output().unwrap().unwrap();
        if let Some(folder) = std::env::var_os("NFSMW_SURFACE_CAPTURE_DIR") {
            let path = std::path::PathBuf::from(folder).join(format!("spray-clear-{clear}.png"));
            let mut encoder = png::Encoder::new(std::fs::File::create(path).unwrap(), image.width, image.height);
            encoder.set_color(png::ColorType::Rgba);
            encoder.set_depth(png::BitDepth::Eight);
            encoder.write_header().unwrap().write_image_data(&image.rgba).unwrap();
        }
        images.push(image);
    }
    let changed = images[0]
        .rgba
        .as_chunks::<4>()
        .0
        .iter()
        .zip(images[1].rgba.as_chunks::<4>().0)
        .filter(|(a, b)| (0..3).any(|c| a[c].abs_diff(b[c]) > 2))
        .count();
    eprintln!("visible highway spray changes {changed} pixels");
    assert!(changed > 100, "emission counters alone must not pass an invisible spray");
    world.close(&mut renderer);
}

#[test]
#[ignore = "requires installed highway, vehicle and surface profiles"]
fn installed_highway_rain_driving_emits_visible_tire_water() {
    check_drive("ELISE", [-130.0, 4089.0]);
    check_drive("BMWM3GTR", [2191.0, 2515.0]);
}

fn check_drive(car: &str, at: [f32; 2]) {
    let dir = game_install::GameDir::open(std::env::var_os("NFSMW_GAME_DIR").unwrap()).unwrap();
    let mut world = WorldScene::open(
        &dir,
        Options {
            at: Some(at),
            height: 3.0,
            heading: 25.0,
            pitch: -12.0,
            fog_distance: 1500.0,
            wait_for_load: true,
            drive: Some(DriveOptions { car: car.into(), script: None }),
        },
    )
    .unwrap();
    let mut renderer = NullBackend::new(crate::settings::test_caps::native());
    world.init(&mut renderer).unwrap();
    let start = Instant::now();
    while !world.ready() {
        world.update(&mut renderer, &ActionState::default(), 0.0);
        assert!(start.elapsed() < Duration::from_secs(60));
        std::thread::sleep(Duration::from_millis(5));
    }
    world.rain.configure(true, 1.0);
    let mut throttle = ActionState::default();
    throttle.update_for_test(&Bindings::default(), &[bevy_input::keyboard::KeyCode::ArrowUp], &[], false);
    for tick in 0..300 {
        world.update(&mut renderer, &throttle, 1.0 / 60.0);
        if tick % 60 == 59 {
            let drive = world.players.lead().unwrap();
            eprintln!(
                "highway rain {} at {:?}: {}",
                world.rain.intensity(),
                drive.position(),
                drive.vehicle_effects.status()
            );
        }
    }
    let drive = world.players.lead().unwrap();
    let mut geometry = Vec::new();
    drive.vehicle_effects.textured(glam::Vec3::X, &world.residency.shared.materials, &mut geometry);
    assert!(geometry.iter().any(|layer| !layer.vertices.is_empty()), "{}", drive.vehicle_effects.status());
    let z = drive.position().z;
    eprintln!(
        "water vertices {} range {:?}",
        geometry.iter().map(|g| g.vertices.len()).sum::<usize>(),
        geometry
            .iter()
            .flat_map(|g| &g.vertices)
            .map(|v| v.position[2] - z)
            .fold((f32::INFINITY, f32::NEG_INFINITY), |a, v| (a.0.min(v), a.1.max(v)))
    );
    world.close(&mut renderer);
}
