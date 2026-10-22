//! The real streaming path must invalidate a rate-limited capture even with no elapsed game time.
use super::*;
use crate::input::ActionState;
use crate::viewer::Scene;
use blackbox_render::{Renderer, RendererOptions};
use std::time::{Duration, Instant};

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR) and a Vulkan GPU"]
fn streaming_refreshes_a_paused_low_quality_capture() {
    let dir = game_install::GameDir::open(std::env::var_os("NFSMW_GAME_DIR").expect("set NFSMW_GAME_DIR")).unwrap();
    let mut renderer = Renderer::headless(
        (64, 64),
        RendererOptions {
            backend: blackbox_gfx::GraphicsApi::Vulkan,
            vsync: false,
            force_fallback_adapter: false,
            pipeline_cache_dir: None,
        },
    )
    .unwrap();
    let mut graphics = *renderer.graphics();
    graphics.reflections = ReflectionQuality::Low;
    renderer.apply_graphics(&graphics);
    let mut world = WorldScene::open(
        &dir,
        super::super::Options {
            at: Some([2150.0, 1399.0]),
            height: 40.0,
            heading: 45.0,
            pitch: -20.0,
            fog_distance: 3000.0,
            wait_for_load: true,
            drive: Some(super::super::DriveOptions { car: super::super::DEFAULT_CAR.into(), script: None }),
        },
    )
    .unwrap();
    world.init(&mut renderer).unwrap();
    let start = Instant::now();
    let mut revisions = std::collections::HashSet::new();
    loop {
        world.update(&mut renderer, &ActionState::default(), 0.0);
        if let Some(stamp) = world.reflection_last {
            revisions.insert(stamp.scenery);
        }
        if world.ready() {
            break;
        }
        assert!(start.elapsed() < Duration::from_secs(60), "streaming timed out");
        std::thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(world.clock, 0.0);
    assert!(revisions.len() > 1, "loading scenery refreshed the capture with the clock paused");
    assert_eq!(world.reflection_last.unwrap().scenery, world.residency.visual_revision());
    assert!(world.reflection_faces.iter().map(Vec::len).sum::<usize>() > 100, "capture contains loaded scenery");
    let unchanged = world.reflection_last;
    world.update(&mut renderer, &ActionState::default(), 0.0);
    assert_eq!(world.reflection_last, unchanged, "an unchanged low-quality view does not need another capture");

    let authored: std::collections::HashSet<_> =
        world.residency.placed().filter(|p| p.original_shadow).map(|p| p.key).collect();
    assert!(!authored.is_empty(), "real assets contain identified shadow overlays");
    let before_faces: usize = world.reflection_faces.iter().map(Vec::len).sum();
    let mut experiments = renderer.render_experiments();
    experiments.original_shadows = false;
    renderer.set_render_experiments(experiments);
    world.update(&mut renderer, &ActionState::default(), 0.0);
    assert_ne!(world.reflection_last, unchanged, "toggling overlays invalidates a paused capture");
    assert!(world.reflection_faces.iter().map(Vec::len).sum::<usize>() < before_faces);
    assert!(world.reflection_faces.iter().flatten().all(|i| !authored.contains(&i.key)));
    assert!(world.frame(1.0).1.iter().all(|i| !authored.contains(&i.key)), "main view omits the same overlays");
    experiments.original_shadows = true;
    renderer.set_render_experiments(experiments);
    world.update(&mut renderer, &ActionState::default(), 0.0);
    assert_eq!(world.reflection_faces.iter().map(Vec::len).sum::<usize>(), before_faces);

    graphics.reflections = ReflectionQuality::High;
    renderer.apply_graphics(&graphics);
    world.update(&mut renderer, &ActionState::default(), 0.0);
    assert_eq!(world.reflection_last.unwrap().quality, ReflectionQuality::High);
    experiments.cached_reflections = true;
    renderer.set_render_experiments(experiments);
    graphics.reflection_resolution.cube = blackbox_gfx::reflection::ReflectionSize::Fixed(128);
    renderer.apply_graphics(&graphics);
    world.update(&mut renderer, &ActionState::default(), 0.0);
    assert_eq!(world.reflection_last.unwrap().size, 128);
    graphics.reflection_resolution.cube = blackbox_gfx::reflection::ReflectionSize::Fixed(256);
    renderer.apply_graphics(&graphics);
    world.update(&mut renderer, &ActionState::default(), 0.0);
    assert_eq!(world.reflection_last.unwrap().size, 256, "paused probe must update when its resolution changes");
    graphics.reflections = ReflectionQuality::Off;
    renderer.apply_graphics(&graphics);
    world.update(&mut renderer, &ActionState::default(), 0.0);
    assert!(world.reflection_last.is_none(), "disabling clears the cached snapshot");

    experiments.sun_shadows = true;
    experiments.freeze_sun = true;
    renderer.set_render_experiments(experiments);
    world.view = super::super::View::Fly;
    world.update_shadow_casters(&mut renderer);
    let identities = |world: &WorldScene| {
        let mut ids: Vec<_> = world.shadow_feed.casters.iter().map(|i| (i.mesh, i.key)).collect();
        ids.sort_unstable();
        ids
    };
    let before = identities(&world);
    let overlays: std::collections::HashSet<_> =
        world.residency.shadow_placed().filter(|p| p.original_shadow).map(|p| p.key).collect();
    assert!(world.shadow_feed.casters.iter().all(|i| !overlays.contains(&i.key)), "authored overlays never cast");
    world.camera.position += glam::Vec3::new(2.0, -2.0, 0.0);
    world.update_shadow_casters(&mut renderer);
    assert_eq!(identities(&world), before, "small camera movement preserves every resident caster silhouette");
    let direction = world.sun.to_sun();
    world.update(&mut renderer, &ActionState::default(), 1.0 / 60.0);
    assert_eq!(world.sun.to_sun(), direction, "freeze holds the scene lighting while its clock advances");
    assert!(world.clock > 0.0);
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR) and a Vulkan GPU"]
fn live_environment_changes_lighting_and_invalidates_frozen_reflections() {
    let dir = game_install::GameDir::open(std::env::var_os("NFSMW_GAME_DIR").expect("set NFSMW_GAME_DIR")).unwrap();
    let mut renderer = Renderer::headless(
        (64, 64),
        RendererOptions {
            backend: blackbox_gfx::GraphicsApi::Vulkan,
            vsync: false,
            force_fallback_adapter: false,
            pipeline_cache_dir: None,
        },
    )
    .unwrap();
    let mut graphics = *renderer.graphics();
    graphics.reflections = ReflectionQuality::Low;
    renderer.apply_graphics(&graphics);
    let mut world = WorldScene::open(
        &dir,
        super::super::Options {
            at: Some([2150.0, 1399.0]),
            height: 40.0,
            heading: 45.0,
            pitch: -20.0,
            fog_distance: 3000.0,
            wait_for_load: true,
            drive: Some(super::super::DriveOptions { car: super::super::DEFAULT_CAR.into(), script: None }),
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
    world.update(&mut renderer, &ActionState::default(), 0.0);
    assert!(world.reflection_last.is_some());
    let noon = world.sun.direction();
    world.set_environment(crate::viewer::Environment { hour: 18.0, seconds_per_hour: 60.0, ..Default::default() });
    assert_eq!(world.environment().unwrap().hour, 18.0);
    assert!(world.reflection_last.is_none(), "a manual jump invalidates even a frozen capture");
    assert_ne!(world.sun.direction(), noon);
    world.refresh_effects(&mut renderer);
    assert!(world.reflection_last.is_some());
    assert_eq!(world.clock, 0.0, "refresh must not advance simulation");
    for tuning in [
        [3.0, 2.0, 1.0, 1.0, 1.0],
        [3.0, 2.5, 1.0, 1.0, 1.0],
        [3.0, 2.5, 0.5, 1.0, 1.0],
        [3.0, 2.5, 0.5, 0.5, 1.0],
        [3.0, 2.5, 0.5, 0.5, 1.5],
    ] {
        let mut options = renderer.render_experiments();
        [
            options.sky_wind_speed,
            options.sky_sun_size,
            options.sky_sun_brightness,
            options.sky_brightness,
            options.sky_sunset_warmth,
        ] = tuning;
        renderer.set_render_experiments(options);
        world.refresh_effects(&mut renderer);
        assert_eq!(world.reflection_last.unwrap().sky_tuning, tuning, "sky edits refresh a frozen probe");
        assert_eq!(world.clock, 0.0);
    }
    world.set_rain(true, 0.4);
    assert_eq!(world.rain.target(), 1.0);
    world.set_rain(false, 0.4);
    assert_eq!(world.rain.target(), 0.0);
}
