//! Installed-model material and power regression.
use super::super::*;
use crate::viewer::Scene;
use blackbox_gfx::{RenderBackend, RenderExperiments};
use blackbox_render::{Renderer, RendererOptions};

#[test]
#[ignore = "requires installed world and Vulkan GPU"]
fn installed_lantern_material_multiple_views() {
    let mut dir = GameDir::open(std::env::var_os("NFSMW_GAME_DIR").expect("game dir")).unwrap();
    if let Some(path) = std::env::var_os("NFSMW_LAMP_MOD_DIR") {
        dir.add_overlay("personal-test", path, 0).unwrap();
    }
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
            at: Some([1674.0, -19.0]),
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
        local_street_lights: true,
        local_street_strength: 3.05,
        local_street_glow: 1.0,
        local_light_distance: 240.0,
        local_shadow_count: 0,
        ..Default::default()
    };
    renderer.set_render_experiments(base);
    // A lantern shows emissive glass only once the main view has drawn a LOD with glass, so render first.
    for _ in 0..2 {
        world.update(&mut renderer, &ActionState::default(), 0.0);
        let (frame, instances) = world.frame(size.0 as f32 / size.1 as f32);
        renderer.render(&frame, instances).unwrap();
    }
    world.update(&mut renderer, &ActionState::default(), 0.0);
    let at = world
        .street_glows
        .iter()
        .find(|(_, fade, _, _, z)| *fade > 0.0 && z.length_squared() > 0.0)
        .map(|g| g.0)
        .expect("a lantern must be resident");
    let fixture_map: std::collections::HashMap<_, _> = world.residency.local_fixtures().collect();
    let placed =
        world
            .residency
            .placed()
            .find(|p| {
                p.lods.iter().flatten().find_map(|m| fixture_map.get(m)).is_some_and(|fs| {
                    fs.iter().any(|f| p.transform.transform_point3(f.glow_position).distance(at) < 0.05)
                })
            })
            .unwrap();
    let key = placed.key;
    assert!(world.fixture_emission.get(&key).is_some_and(|power| *power > 0.0));
    for (view, offset) in [
        ("front", Vec3::new(4.0, 0.0, -0.5)),
        ("side", Vec3::new(0.0, 4.0, -0.5)),
        ("oblique", Vec3::new(-3.0, -3.0, -0.5)),
        ("distance-12", Vec3::new(12.0, 0.0, -0.5)),
        ("distance-24", Vec3::new(24.0, 0.0, -0.5)),
    ] {
        world.camera.position = at + offset;
        let toward = (at - world.camera.position).normalize();
        world.camera.yaw = toward.y.atan2(toward.x);
        world.camera.pitch = toward.z.asin();
        let mut images = Vec::new();
        for (name, strength) in [("off", 0.0), ("on", 1.0)] {
            renderer.set_render_experiments(RenderExperiments { local_street_glow: strength, ..base });
            world.update(&mut renderer, &ActionState::default(), 0.0);
            for _ in 0..3 {
                let (frame, instances) = world.frame(size.0 as f32 / size.1 as f32);
                assert!(instances.iter().any(|i| i.owner_key == key && (i.emission - strength).abs() < 0.01));
                renderer.render(&frame, instances).unwrap();
            }
            let image = renderer.read_output().unwrap().unwrap();
            if let Some(out) = std::env::var_os("NFSMW_LAMP_OUT") {
                let out = std::path::PathBuf::from(out);
                std::fs::create_dir_all(&out).unwrap();
                let file = std::fs::File::create(out.join(format!("material-{view}-{name}.png"))).unwrap();
                let mut encoder = png::Encoder::new(file, size.0, size.1);
                encoder.set_color(png::ColorType::Rgba);
                encoder.set_depth(png::BitDepth::Eight);
                encoder.write_header().unwrap().write_image_data(&image.rgba).unwrap();
            }
            images.push(image);
        }
        let mut gain = 0;
        for y in size.1 / 2 - 30..size.1 / 2 + 30 {
            for x in size.0 / 2 - 30..size.0 / 2 + 30 {
                gain = gain
                    .max(i32::from(images[1].pixel(x, y).unwrap()[0]) - i32::from(images[0].pixel(x, y).unwrap()[0]));
            }
        }
        assert!(gain >= 5, "{view}: the installed glass did not brighten: {gain}");
    }
}
