use super::*;
use crate::settings::{RoadReflectionResolution, RoadReflections};
use crate::{app::test_backend::NullBackend, input::ActionState, viewer::Scene};
use std::time::{Duration, Instant};

#[test]
#[ignore = "needs installed city, sky and vehicle models selected by NFSMW_GAME_DIR"]
fn installed_wet_road_feed_refreshes_quality_pose_and_dry_lifecycle() {
    let dir = game_install::GameDir::open(std::env::var_os("NFSMW_GAME_DIR").unwrap()).unwrap();
    let mut renderer = NullBackend::new(crate::settings::test_caps::native()).with_mesh_tracking();
    renderer.set_render_experiments(blackbox_gfx::RenderExperiments {
        original_world_materials: true,
        ..Default::default()
    });
    let mut world = WorldScene::open(
        &dir,
        super::super::Options {
            at: Some([2398.0, 3352.0]),
            height: 3.0,
            heading: 45.0,
            pitch: -12.0,
            fog_distance: 1500.0,
            wait_for_load: true,
            drive: None,
        },
    )
    .unwrap();
    world.init(&mut renderer).unwrap();
    let start = Instant::now();
    loop {
        world.update(&mut renderer, &ActionState::default(), 0.0);
        if world.ready() {
            break;
        }
        assert!(start.elapsed() < Duration::from_secs(60));
        std::thread::sleep(Duration::from_millis(5));
    }
    let mut environment = world.environment().unwrap();
    environment.road_wetness_override = Some(1.0);
    world.set_environment(environment);
    world.prepare_frame(&mut renderer, 16.0 / 9.0);
    let (frame, size, blur, instances) = renderer.projected.as_ref().expect("wet native feed");
    assert_eq!(*size, [320, 240]);
    assert!(!blur && !instances.is_empty(), "real authored road-view sources");
    assert!(instances.iter().all(|instance| renderer.mesh_labels.contains_key(&instance.mesh)));
    assert_eq!(frame.view, world.gameplay_lens(16.0 / 9.0).view, "source shares the main camera");
    let mesh_count = renderer.mesh_labels.len();
    world.set_road_reflections(RoadReflections::Blurred, RoadReflectionResolution::Quadruple);
    world.prepare_frame(&mut renderer, 16.0 / 9.0);
    let (_, size, blur, _) = renderer.projected.as_ref().unwrap();
    assert_eq!(*size, [1280, 960]);
    assert!(blur);
    assert_eq!(renderer.mesh_labels.len(), mesh_count);
    environment.road_wetness_override = Some(0.0);
    world.set_environment(environment);
    world.prepare_frame(&mut renderer, 16.0 / 9.0);
    assert!(renderer.projected.is_none(), "dry transitions remove the active view");
    assert_eq!(world.surface_light().unwrap().reflection_scale, 0.0);
    renderer.set_render_experiments(blackbox_gfx::RenderExperiments { enhanced_sky: false, ..Default::default() });
    world.prepare_frame(&mut renderer, 16.0 / 9.0);
    assert!(!world.original_sky, "paused material changes refresh sky selection before either view");
    renderer.set_render_experiments(blackbox_gfx::RenderExperiments {
        original_world_materials: true,
        ..Default::default()
    });
    world.prepare_frame(&mut renderer, 16.0 / 9.0);
    assert!(world.original_sky);
    // Wetness shares the simulation clock with rain and pauses with the garage.
    world.wet_road.override_amount = None;
    world.wet_road.wetness = 0.0;
    world.rain.configure(true, 1.0);
    world.tuning.game_speed = 0.25;
    for _ in 0..60 {
        world.update(&mut renderer, &ActionState::default(), 1.0 / 60.0);
    }
    assert!((world.wet_road.wetness - (1.0 - 0.995_f32.powi(15))).abs() < 0.0001);
    let wet = world.wet_road.wetness;
    world.set_garage_open(true);
    world.update(&mut renderer, &ActionState::default(), 1.0);
    assert_eq!(world.wet_road.wetness, wet, "pause holds wetness history");
    world.close(&mut renderer);
    assert!(renderer.projected.is_none());
    assert!(world.wet_road.instances.is_empty());
}
