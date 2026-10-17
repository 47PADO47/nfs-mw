//! Installed input → physics → NOS particles and brake-flare rendering regression.
use crate::garage::Selection;
use crate::scenes::world::*;
use crate::viewer::Scene;
use bevy_input::keyboard::KeyCode;
use blackbox_render::{Renderer, RendererOptions};
use std::time::{Duration, Instant};

#[test]
#[ignore = "requires an installed game and a Vulkan GPU"]
fn installed_upgraded_elise_nitrous_input_drives_both_visuals() {
    let dir = GameDir::open(std::env::var_os("NFSMW_GAME_DIR").unwrap()).unwrap();
    let size = (640, 360);
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
    scene.init(&mut renderer).unwrap();
    let package = scene.garage.cars.iter().find(|c| c.folder == "ELISE").unwrap().performance.maximum();
    scene
        .apply_garage_selection(
            &mut renderer,
            Selection { car: "ELISE".into(), performance: package, ..Default::default() },
        )
        .unwrap();
    scene.set_exhaust_flames(true);
    let idle = ActionState::default();
    let started = Instant::now();
    while !scene.ready() {
        scene.update(&mut renderer, &idle, 1.0 / 60.0);
        assert!(started.elapsed() < Duration::from_secs(60), "installed scene did not become ready");
        std::thread::sleep(Duration::from_millis(5));
    }
    for _ in 0..30 {
        scene.update(&mut renderer, &idle, 1.0 / 60.0);
    }
    // Start rolling on the actual resident road, avoiding a route-dependent launch.
    let drive = scene.drive.as_mut().unwrap();
    let spawn = road::Spawn { position: drive.physics_pose().position, heading: drive.heading() };
    let ground = drive::WorldGround { collision: scene.residency.collision(), surfaces: &scene.physics.surfaces };
    assert!(drive.sim_mut().unwrap().place_moving(&ground, spawn, 20.0));
    let mut burn = ActionState::default();
    burn.update_for_test(&crate::input::Bindings::default(), &[KeyCode::ArrowUp, KeyCode::ShiftLeft], &[], false);
    for _ in 0..3 {
        scene.update(&mut renderer, &burn, 1.0 / 60.0);
    }
    let drive = scene.drive.as_mut().unwrap();
    assert!(drive.sim_mut().unwrap().telemetry().nos_burning, "actual NOS activation is required");
    assert!(drive.flames.live() > 0, "installed particle emitters must spawn from physics engagement");
    assert_eq!(drive.tail_trails.samples().count(), 24);
    let lens = scene.gameplay_lens(size.0 as f32 / size.1 as f32);
    let drive = scene.drive.as_mut().unwrap();
    let mut flames = blackbox_gfx::EffectLayer::default();
    drive.flames.geometry(lens.position, lens.forward, &mut flames);
    let mut tails = blackbox_gfx::EffectLayer::default();
    let (lamps, profiles, world, state) = drive.lamp_source();
    profiles.draw_trails(
        &lamps,
        &drive.tail_trails,
        world,
        state,
        car_light_flares::FlareView {
            camera: lens.position,
            forward: lens.forward,
            fov_degrees: lens.fov_y_degrees,
            height: size.1,
        },
        &mut tails.textured,
    );
    assert!(!flames.textured.is_empty() && !tails.textured.is_empty());
    let mut combined = tails.clone();
    combined.textured.extend(flames.textured.clone());
    let (frame, instances) = scene.frame(size.0 as f32 / size.1 as f32);
    let instances = instances.to_vec();
    renderer.set_effects(&blackbox_gfx::EffectLayer::default());
    renderer.render(&frame, &instances).unwrap();
    let plain = renderer.read_output().unwrap().unwrap();
    let mut captures = vec![("upgraded-elise-no-effects", plain)];
    for (name, layer) in
        [("upgraded-elise-flames", &flames), ("upgraded-elise-trails", &tails), ("upgraded-elise-nos", &combined)]
    {
        renderer.set_effects(layer);
        renderer.render(&frame, &instances).unwrap();
        let lit = renderer.read_output().unwrap().unwrap();
        let changed = lit
            .rgba
            .as_chunks::<4>()
            .0
            .iter()
            .zip(captures[0].1.rgba.as_chunks::<4>().0)
            .filter(|(a, b)| a != b)
            .count();
        assert!(changed > 8, "{name} must independently alter the actual output: {changed} pixels");
        captures.push((name, lit));
    }
    scene.drive.as_mut().unwrap().flames.reclaim(&mut flames);
    if let Some(path) = std::env::var_os("NFSMW_EFFECTS_CAPTURE_DIR") {
        let path = std::path::PathBuf::from(path);
        std::fs::create_dir_all(&path).unwrap();
        for (name, image) in &captures {
            let file = std::fs::File::create(path.join(format!("{name}.png"))).unwrap();
            let mut encoder = png::Encoder::new(file, size.0, size.1);
            encoder.set_color(png::ColorType::Rgba);
            encoder.set_depth(png::BitDepth::Eight);
            encoder.write_header().unwrap().write_image_data(&image.rgba).unwrap();
        }
    }
    // A held NOS key without throttle stops physics activation and the trail copies.
    let mut released = ActionState::default();
    released.update_for_test(&crate::input::Bindings::default(), &[KeyCode::ShiftLeft], &[], false);
    scene.update(&mut renderer, &released, 1.0 / 60.0);
    let drive = scene.drive.as_mut().unwrap();
    assert!(!drive.sim_mut().unwrap().telemetry().nos_burning);
    assert_eq!(drive.tail_trails.samples().count(), 0);
}
