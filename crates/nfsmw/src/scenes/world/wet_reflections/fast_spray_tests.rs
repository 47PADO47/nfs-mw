//! High-speed installed-particle appearance fixture.
use super::super::{DriveOptions, Options, WorldScene};
use crate::{
    input::{ActionState, Bindings},
    viewer::Scene,
};
use std::time::{Duration, Instant};
#[test]
#[ignore = "requires installed highway, water textures and a Vulkan GPU"]
fn installed_fast_carrera_tire_spray_changes_visible_pixels() {
    let mut dir = game_install::GameDir::open(std::env::var_os("NFSMW_GAME_DIR").unwrap()).unwrap();
    if let Some(path) = std::env::var_os("NFSMW_TEST_OVERLAY") {
        dir.add_overlay("matched water", path, 300).unwrap();
    }
    let mut renderer = blackbox_render::Renderer::headless(
        (1920, 1080),
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
            at: Some([2191.0, 2515.0]),
            height: 3.0,
            heading: 20.0,
            pitch: -12.0,
            fog_distance: 1500.0,
            wait_for_load: true,
            drive: Some(DriveOptions { car: "CARRERAGT".into(), script: None }),
        },
    )
    .unwrap();
    blackbox_gfx::RenderBackend::set_render_experiments(
        &mut renderer,
        blackbox_gfx::RenderExperiments { original_world_materials: true, enhanced_sky: true, ..Default::default() },
    );
    world.init(&mut renderer).unwrap();
    let start = Instant::now();
    while !world.ready() {
        world.update(&mut renderer, &ActionState::default(), 0.0);
        assert!(start.elapsed() < Duration::from_secs(60));
        std::thread::sleep(Duration::from_millis(5));
    }
    world.set_camera_mode(crate::settings::CameraMode::Far);
    let drive = world.players.lead_mut().unwrap();
    let spawn = super::super::road::Spawn { position: drive.position(), heading: drive.heading() };
    drive.place_exactly(spawn, 80.0);
    world.update(&mut renderer, &ActionState::default(), 0.0);
    world.rain.configure(true, 1.0);
    world.atmosphere.configure(Some(1.0), true, [2191.0, 2515.0], 1.0);
    let mut throttle = ActionState::default();
    throttle.update_for_test(&Bindings::default(), &[bevy_input::keyboard::KeyCode::ArrowUp], &[], false);
    for _ in 0..60 {
        world.update(&mut renderer, &throttle, 1.0 / 60.0);
    }
    let drive = world.players.lead().unwrap();
    eprintln!("Carrera speed {} km/h; {}", drive.speed() * 3.6, drive.vehicle_effects.status());
    assert!(drive.speed() > 40.0, "fixture must remain moving quickly");
    let mut images = Vec::new();
    for clear in [false, true] {
        if clear {
            world.players.lead_mut().unwrap().vehicle_effects.clear();
        }
        world.prepare_frame(&mut renderer, 16.0 / 9.0);
        world.upload_effects(&mut renderer);
        blackbox_gfx::RenderBackend::set_focus_filter(&mut renderer, Default::default());
        let (frame, instances) = world.frame(16.0 / 9.0);
        renderer.render(&frame, instances).unwrap();
        let image = renderer.read_output().unwrap().unwrap();
        if let Some(folder) = std::env::var_os("NFSMW_SURFACE_CAPTURE_DIR") {
            let path = std::path::PathBuf::from(folder).join(format!("carrera-fast-clear-{clear}.png"));
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
