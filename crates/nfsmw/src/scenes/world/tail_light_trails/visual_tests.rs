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

pub(super) fn save(name: &str, image: &RgbaImage) {
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
fn installed_nos_trail_visual_comparisons() {
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
        scene.set_car_shading(settings.effective_car_shading());
        scene.set_detail(settings.detail);
        scene.set_road_reflections(settings.road_reflections, settings.road_reflection_resolution);
        scene.set_world_tuning(crate::settings::WorldTuning::from(settings));
        scene.set_rain(settings.always_rain, settings.rain_amount.amplitude());
    }
    scene.init(&mut renderer).unwrap();
    let car = std::env::var("NFSMW_TRAIL_CAR").unwrap_or_else(|_| "BMWM3GTRE46".into());
    let package = scene.garage.cars.iter().find(|c| c.folder == car).unwrap().performance.maximum();
    scene.apply_garage_selection(&mut renderer, Selection { car, performance: package, ..Default::default() }).unwrap();
    scene.set_exhaust_flames(true);
    scene.tuning.enhanced_exhaust = true;
    if settings.is_none() {
        scene.tuning.motion_blur.enabled = false;
    }

    if let Some(path) = std::env::var_os("NFSMW_EFFECTS_CAPTURE_DIR") {
        let metadata = serde_json::json!({
            "size": size, "mods": mods.as_ref().map(|m| m.loaded().into_iter().map(|m| m.id).collect::<Vec<_>>()),
            "graphics": format!("{graphics:?}"), "rt_available": ray_queries,
            "simulation_hz": 60, "presentation_hz": 120,
        });
        std::fs::write(std::path::PathBuf::from(path).join("capture.json"), metadata.to_string()).unwrap();
    }
    let idle = ActionState::default();
    let started = Instant::now();
    while !scene.ready() {
        scene.update(&mut renderer, &idle, 1.0 / 120.0);
        assert!(started.elapsed() < Duration::from_secs(120));
        std::thread::sleep(Duration::from_millis(5));
    }
    scene.extra.testing.traffic_off = true;
    scene.extra.testing.police_off = true;
    scene.extra.testing.unlimited_nos = true;
    scene.extra.testing.unlimited_breaker = true;
    scene.set_camera_mode(crate::settings::CameraMode::Far);
    scene.tuning.tail_trail_style.enhanced = true;
    let spawn = road::Spawn {
        position: scene.players.lead().unwrap().physics_pose().position,
        heading: scene.players.lead().unwrap().heading(),
    };
    if std::env::var("NFSMW_TRAIL_PREVIEW_ONLY").as_deref() == Ok("1") {
        super::preview_visual::check(&mut scene, &mut renderer, aspect);
        return;
    }
    for (scenario, hour, keys) in [
        ("day", 12.0, vec![KeyCode::ArrowUp, KeyCode::ShiftLeft]),
        ("dusk", 18.5, vec![KeyCode::ArrowUp, KeyCode::ShiftLeft]),
        ("turn", 12.0, vec![KeyCode::ArrowUp, KeyCode::ShiftLeft, KeyCode::ArrowLeft]),
        ("breaker", 12.0, vec![KeyCode::ArrowUp, KeyCode::ShiftLeft, KeyCode::ArrowLeft, KeyCode::ControlLeft]),
    ] {
        let mut env = scene.environment().unwrap();
        env.sun_mode = crate::viewer::SunMode::CustomClock;
        env.hour = hour;
        scene.set_environment(env);
        {
            let ground =
                drive::WorldGround { collision: scene.residency.collision(), surfaces: &scene.physics.surfaces };
            let drive = scene.players.lead_mut().unwrap();
            assert!(drive.sim_mut().unwrap().place_moving(&ground, spawn, 65.0));
            drive.tail_trails.clear();
        }
        let mut burn = ActionState::default();
        burn.update_for_test(&crate::input::Bindings::default(), &keys, &[], false);
        for _ in 0..60 {
            scene.update(&mut renderer, &burn, 1.0 / 120.0);
            let (frame, instances) = scene.frame(aspect);
            renderer.render(&frame, instances).unwrap();
        }
        assert!(scene.players.lead_mut().unwrap().sim_mut().unwrap().telemetry().nos_burning);
        if scenario == "breaker" {
            assert!(scene.players.lead().unwrap().speedbreaker_active(), "Speedbreaker must really be active");
        }
        let (base, instances) = scene.frame(aspect);
        let instances = instances.to_vec();
        let pose = scene.players.lead().unwrap().pose();
        if scenario == "day" {
            let mut off: Option<Vec<u8>> = None;
            for level in [0, 100] {
                let mut style = scene.tuning.tail_trail_style;
                style.lamp_glow = level;
                scene.players.lead_mut().unwrap().set_tail_style(style);
                assert_eq!(scene.players.lead().unwrap().lamp_source().3.brake, 0.0);
                let (f, instances) = scene.frame(aspect);
                renderer.set_effects(&EffectLayer::default());
                renderer.render(&f, instances).unwrap();
                let image = renderer.read_output().unwrap().unwrap();
                if let Some(off) = &off {
                    assert!(
                        image.rgba.iter().zip(off).filter(|(a, b)| a != b).count() > 8,
                        "NOS must illuminate actual lamp surfaces"
                    );
                }
                save(
                    match level {
                        0 => "nos-lamps-off",
                        _ => "nos-lamps-on",
                    },
                    &image,
                );
                off = Some(image.rgba);
            }
            let saved_traffic = scene.traffic.take();
            scene.tuning.motion_blur.strength = 1.0;
            let mut off: Option<Vec<u8>> = None;
            for enabled in [false, true] {
                scene.tuning.motion_blur.enabled = enabled;
                for _ in 0..60 {
                    scene.feed_focus(&mut renderer, 1.0 / 120.0);
                }
                renderer.render(&base, &instances).unwrap();
                let image = renderer.read_output().unwrap().unwrap();
                if let Some(off) = &off {
                    let changed = image.rgba.iter().zip(off.iter()).filter(|(a, b)| a != b).count();
                    assert!(changed > 1000, "blur must change actual pixels without traffic: {changed}");
                }
                save(
                    match enabled {
                        true => "motion-blur-on",
                        false => "motion-blur-off",
                    },
                    &image,
                );
                off = Some(image.rgba);
            }
            scene.traffic = saved_traffic;
            scene.tuning.motion_blur.enabled = false;
            scene.feed_focus(&mut renderer, 0.0);
        }
        for (camera, offset) in [
            ("chase", Vec3::new(-8.0, -1.2, 2.6)),
            ("close", Vec3::new(-4.5, -2.4, 1.5)),
            ("rear", Vec3::new(-6.0, 0.0, 1.0)),
        ] {
            let eye = pose.transform().transform_point3(offset);
            let target = pose.transform().transform_point3(Vec3::new(-0.5, 0.0, 0.6));
            let forward = (target - eye).normalize();
            let frame = FrameParams { view: look_at_mat4(eye, target, Vec3::Z), camera_position: eye, ..base };
            let view = car_light_flares::FlareView { camera: eye, forward, fov_degrees: 55.0, height: size.1 };
            for (variant, width, brightness, glow) in [
                ("original", 100, 100, 100),
                ("baseline", 100, 100, 100),
                ("refined", 100, 100, 100),
                ("soft", 80, 85, 60),
                ("wide", 200, 200, 200),
                ("strong", 115, 140, 140),
            ] {
                let drive = scene.players.lead_mut().unwrap();
                let (lamps, flares, world, state) = drive.lamp_source();
                let mut layer = EffectLayer::default();
                flares.draw(&lamps, world, state, view, &mut layer.textured);
                drive.flames.geometry(eye, forward, &mut layer);
                match variant {
                    "original" => {
                        flares.draw_trails(&lamps, &drive.tail_trails, world, state, view, &mut layer.textured)
                    }
                    "baseline" => legacy(&lamps, &drive.tail_trails, world, view, &mut layer.streaks),
                    _ => flares.draw_smooth_trails(
                        &lamps,
                        &drive.tail_trails,
                        world,
                        state,
                        view,
                        crate::settings::TailTrailStyle {
                            enhanced: true,
                            width,
                            brightness,
                            glow,
                            ..Default::default()
                        },
                        drive.spark_body().1,
                        &mut layer,
                    ),
                }
                if variant != "original" {
                    assert!(!layer.streaks.is_empty(), "{scenario}: missing ribbon geometry");
                }
                let image = capture(&mut renderer, &frame, &instances, &layer);
                save(&format!("{scenario}-{camera}-{variant}"), &image);
                scene.players.lead_mut().unwrap().flames.reclaim(&mut layer);
            }
        }
    }
}

// Short-exposure reference with healthy lamps and no brake input; not a byte-exact v12 capture.
fn legacy(
    lamps: &car_lights::CarLights,
    history: &tail_light_trails::TailLightTrails,
    world: glam::Mat4,
    view: car_light_flares::FlareView,
    out: &mut Vec<blackbox_gfx::EffectVertex>,
) {
    for lamp in lamps.lamps().iter().filter(|lamp| lamp.kind == car_lights::Kind::Tail) {
        let at = world.transform_point3(lamp.at);
        let to_eye = view.camera - at;
        let facing = world.transform_vector3(lamp.direction).normalize_or_zero().dot(to_eye.normalize_or_zero());
        if facing <= 0.0 {
            continue;
        }
        let fade = ((40000.0 - to_eye.length_squared()) / 8000.0).clamp(0.0, 1.0);
        let gain = 0.5 * facing.powi(2) * fade * history.fade();
        let path = history.path(lamp.at, 21, 21);
        for (radius, alpha) in [(0.10, 100.0), (0.032, 230.0)] {
            EffectLayer::ribbon(
                out,
                &path,
                radius,
                [255, 45, 20, (alpha * gain).round() as u8],
                view.camera,
                view.forward,
            );
        }
    }
}
