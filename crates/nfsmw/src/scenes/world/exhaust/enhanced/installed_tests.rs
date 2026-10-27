//! Matched real-car captures, plus actual driving engagement and release.
use crate::garage::Selection;
use crate::scenes::world::*;
use crate::viewer::Scene;
use bevy_input::keyboard::KeyCode;
use blackbox_gfx::{EffectLayer, FrameParams, Instance, RgbaImage};
use blackbox_render::{Renderer, RendererOptions};
use clap::Parser;
use glam::Vec3;
use glam::camera::rh::view::look_at_mat4;
use std::time::{Duration, Instant};

fn save(name: &str, image: &RgbaImage) {
    let Some(dir) = std::env::var_os("NFSMW_EFFECTS_CAPTURE_DIR") else { return };
    let dir = std::path::PathBuf::from(dir);
    std::fs::create_dir_all(&dir).unwrap();
    let file = std::fs::File::create(dir.join(format!("{name}.png"))).unwrap();
    let mut encoder = png::Encoder::new(file, image.width, image.height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder.write_header().unwrap().write_image_data(&image.rgba).unwrap();
}

fn capture(renderer: &mut Renderer, frame: &FrameParams, instances: &[Instance], layer: &EffectLayer) -> RgbaImage {
    renderer.set_effects(layer);
    renderer.render(frame, instances).unwrap();
    renderer.read_output().unwrap().unwrap()
}

#[test]
#[ignore = "requires an installed game and a native GPU"]
fn installed_enhanced_exhaust_drives_and_captures_original_and_reference_flames() {
    let game = std::path::PathBuf::from(std::env::var_os("NFSMW_GAME_DIR").unwrap());
    let (dir, mods, settings) = match std::env::var_os("NFSMW_EFFECTS_MODS_DIR") {
        Some(root) => {
            let cli = crate::cli::Cli::parse_from([
                std::ffi::OsString::from("nfsmw"),
                "view-world".into(),
                "--mods-dir".into(),
                root,
            ]);
            let Some(crate::cli::Command::ViewWorld { view, .. }) = cli.command else { unreachable!() };
            let settings = crate::settings::Settings::load(view.settings_layer());
            let (dir, mods) = crate::modding::open_install(Some(&game), &view).unwrap();
            (dir, Some(mods), Some(settings))
        }
        None => (GameDir::open(&game).unwrap(), None, None),
    };
    let size = match settings.is_some() {
        true => (1920, 1080),
        false => (1280, 720),
    };
    let aspect = size.0 as f32 / size.1 as f32;
    let api = match std::env::var("WGPU_BACKEND").as_deref() {
        Ok("dx12") => blackbox_gfx::GraphicsApi::Dx12,
        _ => blackbox_gfx::GraphicsApi::Vulkan,
    };
    let mut renderer = Renderer::headless(
        size,
        RendererOptions { backend: api, vsync: false, force_fallback_adapter: false, pipeline_cache_dir: None },
    )
    .unwrap();
    let ray_queries = blackbox_gfx::RenderBackend::ray_reflections_enabled(&renderer);
    if std::env::var("BLACKBOX_RT_REFLECTIONS").as_deref() == Ok("1") && api == blackbox_gfx::GraphicsApi::Vulkan {
        assert!(ray_queries, "RT-enabled Vulkan demo must expose the real hardware RT capability to F8");
    }
    let graphics = settings.as_ref().map(|settings| {
        blackbox_gfx::RenderBackend::set_display_settings(&mut renderer, settings.appearance.display.request());
        let resolved = blackbox_gfx::RenderBackend::apply_graphics(&mut renderer, &settings.graphics());
        blackbox_gfx::RenderBackend::set_render_experiments(&mut renderer, settings.appearance.lighting);
        resolved
    });
    let mut scene = WorldScene::open(
        &dir,
        Options {
            at: Some([2974.2, 3001.9]),
            height: 40.0,
            heading: 45.0,
            pitch: -20.0,
            fog_distance: 1500.0,
            wait_for_load: false,
            drive: None,
        },
    )
    .unwrap();
    if let Some(settings) = &settings {
        scene.set_car_shading(settings.car_shading);
        scene.set_detail(settings.detail);
        scene.set_road_reflections(settings.road_reflections, settings.road_reflection_resolution);
        scene.set_world_tuning(crate::settings::WorldTuning::from(settings));
        scene.set_rain(settings.always_rain, settings.rain_amount.amplitude());
    }
    scene.init(&mut renderer).unwrap();
    let package = scene.garage.cars.iter().find(|c| c.folder == "BMWM3GTR").unwrap().performance.maximum();
    scene
        .apply_garage_selection(
            &mut renderer,
            Selection { car: "BMWM3GTR".into(), performance: package, ..Default::default() },
        )
        .unwrap();
    scene.set_exhaust_flames(true);
    scene.tuning.enhanced_exhaust = true;
    if settings.is_none() {
        scene.tuning.motion_blur.enabled = false;
    }
    if let Some(dir) = std::env::var_os("NFSMW_EFFECTS_CAPTURE_DIR") {
        let path = std::path::PathBuf::from(dir).join("garage-presets.toml");
        let payload = crate::garage::presets::encode(&scene.garage.applied).unwrap();
        crate::preset_store::save(&path, "NOS Demo", &payload).unwrap();
        let metadata = serde_json::json!({
            "size": [size.0, size.1],
            "mods": mods.as_ref().map(|m| m.loaded().into_iter().map(|m| m.id).collect::<Vec<_>>()),
            "graphics": format!("{graphics:?}"),
            "fixed_camera_flame_speed_mps": 60.0,
            "hardware_rt_available": ray_queries,
        });
        std::fs::write(path.with_file_name("capture-setup.json"), serde_json::to_string_pretty(&metadata).unwrap())
            .unwrap();
    }
    let idle = ActionState::default();
    let started = Instant::now();
    while !scene.ready() {
        scene.update(&mut renderer, &idle, 1.0 / 60.0);
        assert!(started.elapsed() < Duration::from_secs(90), "installed scene did not become ready");
        std::thread::sleep(Duration::from_millis(5));
    }
    for _ in 0..30 {
        scene.update(&mut renderer, &idle, 1.0 / 60.0);
    }
    let drive = scene.players.lead_mut().unwrap();
    let spawn = road::Spawn { position: drive.physics_pose().position, heading: drive.heading() };
    let ground = drive::WorldGround { collision: scene.residency.collision(), surfaces: &scene.physics.surfaces };
    assert!(drive.sim_mut().unwrap().place_moving(&ground, spawn, 30.0));
    let mut burning = ActionState::default();
    burning.update_for_test(&crate::input::Bindings::default(), &[KeyCode::ArrowUp, KeyCode::ShiftLeft], &[], false);
    for _ in 0..15 {
        scene.update(&mut renderer, &burning, 1.0 / 60.0);
    }
    assert!(scene.players.lead_mut().unwrap().sim_mut().unwrap().telemetry().nos_burning);

    let before = scene.players.lead().unwrap().physics_pose().position;
    let mut customized = scene.garage.applied.clone();
    customized.exhaust = crate::garage::ExhaustSelection::aggressive();
    customized.exhaust.root = [60, 255, 100];
    customized.exhaust.tail = [170, 50, 255];
    scene.apply_garage_selection(&mut renderer, customized.clone()).unwrap();
    assert_eq!(
        scene.players.lead().unwrap().physics_pose().position,
        before,
        "appearance edits must not respawn the car"
    );
    scene.upload_effects(&mut renderer);
    let (custom_frame, custom_instances) = scene.frame(aspect);
    renderer.render(&custom_frame, custom_instances).unwrap();
    save("custom-colours", &renderer.read_output().unwrap().unwrap());
    customized.exhaust = Default::default();
    scene.apply_garage_selection(&mut renderer, customized).unwrap();
    scene.upload_effects(&mut renderer);

    let (base, instances) = scene.frame(aspect);
    let instances = instances.to_vec();
    let pose = scene.players.lead().unwrap().pose();
    for (name, offset) in [("rear", Vec3::new(-4.0, -1.6, 1.1)), ("side", Vec3::new(-2.0, -2.8, 0.7))] {
        let eye = pose.transform().transform_point3(offset);
        let target = pose.transform().transform_point3(Vec3::new(-1.6, 0.0, 0.4));
        let frame = FrameParams { view: look_at_mat4(eye, target, Vec3::Z), camera_position: eye, ..base };
        let plain = capture(&mut renderer, &frame, &instances, &EffectLayer::default());
        save(&format!("{name}-plain"), &plain);
        for enhanced in [false, true] {
            let drive = scene.players.lead_mut().unwrap();
            drive.flames.set_enhanced(enhanced);
            let mut layer = EffectLayer::default();
            drive.flames.geometry(eye, (target - eye).normalize(), &mut layer);
            assert!(match enhanced {
                true => !layer.flames.is_empty(),
                false => !layer.textured.is_empty(),
            });
            let image = capture(&mut renderer, &frame, &instances, &layer);
            let changed = image
                .rgba
                .as_chunks::<4>()
                .0
                .iter()
                .zip(plain.rgba.as_chunks::<4>().0.iter())
                .filter(|(a, b)| a != b)
                .count();
            assert!(changed > 8, "{name}, enhanced={enhanced}: {changed} changed pixels");
            let appearance = match enhanced {
                true => "enhanced",
                false => "original",
            };
            save(&format!("{name}-{appearance}"), &image);
            scene.players.lead_mut().unwrap().flames.reclaim(&mut layer);
        }
    }

    // Freeze the camera and car pose while previewing the flame's flow at 216 km/h.
    let eye = pose.transform().transform_point3(Vec3::new(-3.5, -1.2, 0.8));
    let target = pose.transform().transform_point3(Vec3::new(-1.8, 0.0, 0.4));
    let frame = FrameParams { view: look_at_mat4(eye, target, Vec3::Z), camera_position: eye, ..base };
    let sequence = std::env::var_os("NFSMW_EXHAUST_SEQUENCE").is_some();
    if sequence {
        let drive = scene.players.lead_mut().unwrap();
        drive.flames.set_enhanced(true);
        let car = exhaust::CarState {
            to_world: pose.transform(),
            velocity: pose.transform().transform_vector3(Vec3::X * 60.0),
            gear: 3,
            throttle: 1.0,
            nitrous: true,
        };
        for index in 0..120 {
            drive.flames.step(1.0 / 60.0, &car);
            let mut layer = EffectLayer::default();
            drive.flames.geometry(eye, (target - eye).normalize(), &mut layer);
            let image = capture(&mut renderer, &frame, &instances, &layer);
            save(&format!("flow-{index:03}"), &image);
        }
        for index in 0..60 {
            scene.update(&mut renderer, &burning, 1.0 / 60.0);
            let (frame, instances) = scene.frame(aspect);
            renderer.render(&frame, instances).unwrap();
            save(&format!("drive-{index:03}"), &renderer.read_output().unwrap().unwrap());
        }
    }

    // The real input path must also stop emission after throttle and NOS are released.
    for _ in 0..30 {
        scene.update(&mut renderer, &idle, 1.0 / 60.0);
    }
    let drive = scene.players.lead_mut().unwrap();
    assert!(!drive.sim_mut().unwrap().telemetry().nos_burning);
    let mut layer = EffectLayer::default();
    drive.flames.geometry(eye, (target - eye).normalize(), &mut layer);
    assert!(layer.flames.iter().all(|v| !v.blue), "no sustained NOS flame after release");

    scene.set_garage_open(true);
    scene.garage.exhaust_preview = true;
    scene.upload_effects(&mut renderer);
    let drive = scene.players.lead_mut().unwrap();
    assert!(!drive.sim_mut().unwrap().telemetry().nos_burning, "preview cannot burn real NOS");
    layer.clear();
    drive.flames.geometry(eye, (target - eye).normalize(), &mut layer);
    assert!(layer.flames.iter().any(|v| v.blue), "F6 preview must emit while physics is idle");
    scene.set_garage_open(false);
    scene.upload_effects(&mut renderer);
    layer.clear();
    scene.players.lead_mut().unwrap().flames.geometry(eye, (target - eye).normalize(), &mut layer);
    assert!(layer.flames.iter().all(|v| !v.blue), "closing F6 removes its preview");
}
