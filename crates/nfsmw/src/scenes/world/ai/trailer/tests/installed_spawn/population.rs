//! Real automatic placement at both distance presets, without a GPU or a running original game.
use super::*;
use crate::scenes::world::ai::{Focus, TrafficWorld, spawn::Pick, traffic::Cruise};
use crate::settings::TrafficPolicy;

#[test]
#[ignore = "needs installed traffic and world collision selected by NFSMW_GAME_DIR"]
fn installed_traffic_population_uses_live_manager_and_collision() {
    let dir = GameDir::open(std::env::var_os("NFSMW_GAME_DIR").expect("set NFSMW_GAME_DIR")).unwrap();
    let index = WorldIndex::open(&dir, DEFAULT_TRACK).unwrap();
    let data = PhysicsData::load(&dir).unwrap();
    let mut collision = CollisionWorld::new(index.collision_grid.clone());
    for section in &index.sections {
        if !section.is_spatial() {
            continue;
        }
        let bytes = dir.read_range(&index.stream_file, u64::from(section.file_offset), section.size as usize).unwrap();
        for pack in blackbox_collision::read_collision_packs(&bytes).unwrap() {
            collision.insert(pack);
        }
    }
    let mut renderer = NullBackend::new(Capabilities::baseline("null", GraphicsApi::Vulkan));
    for policy in [TrafficPolicy::default(), TrafficPolicy::extended()] {
        let mut world = TrafficWorld::new(index.road_network.clone().unwrap());
        world.load_data(data.database(), index.track_zones.clone());
        let focus = Focus { position: [2395.0, 2913.0], heading: std::f32::consts::FRAC_PI_2, speed: 0.0, body: None };
        world.configure(64, 0);
        world.set_policy(policy);
        world.manage(0.5, focus, &collision, &data.surfaces);
        for request in world.take_model_requests() {
            let crate::scenes::world::ai::ModelRequest::Traffic(name) = request else { continue };
            let mut car = model(&dir, &data, &name, &mut renderer);
            if let Some(name) = data.trailer_of(&name) {
                car.trailer = Some(Box::new(model(&dir, &data, &name, &mut renderer)));
            }
            world.models.push(car);
        }
        let mut props = PropWorld::default();
        for tick in 0..3600 {
            // Stationary encounter probe: actual manager, vehicle physics, ground and recycling.
            let eye = Vec3::new(focus.position[0], focus.position[1] - 8.0, 150.0);
            let lens = crate::viewer::camera::Lens::look_at(eye, eye + Vec3::Y, 60.0, 0.1, 16.0 / 9.0);
            world.step(focus, &collision, &mut props, &data.surfaces);
            world.finish_step(focus, &lens, &lens, &props, &collision, &data.surfaces);
            if tick % 600 == 599 {
                eprintln!("population {policy:?} t={}: {}", (tick + 1) / 60, world.status(focus));
            }
        }
        assert!(
            world.population_stats.spawned >= 16,
            "failed probes must not starve the manager: {}",
            world.status(focus)
        );
        assert_eq!(world.population_stats.waiting_collision, 0, "fixture has complete road collision");
        let activated = world.population_stats.spawned;
        world.set_policy(TrafficPolicy { density_percent: 0, ..policy });
        for _ in 0..600 {
            let eye = Vec3::new(focus.position[0], focus.position[1] - 8.0, 150.0);
            let lens = crate::viewer::camera::Lens::look_at(eye, eye + Vec3::Y, 60.0, 0.1, 16.0 / 9.0);
            world.step(focus, &collision, &mut props, &data.surfaces);
            world.finish_step(focus, &lens, &lens, &props, &collision, &data.surfaces);
        }
        assert_eq!(world.population_stats.spawned, activated, "live zero density must stop activations");
    }
}

#[test]
#[ignore = "needs installed traffic and world collision selected by NFSMW_GAME_DIR"]
fn installed_reference_and_extended_traffic_spawn_within_supported_bands() {
    let dir = GameDir::open(std::env::var_os("NFSMW_GAME_DIR").expect("set NFSMW_GAME_DIR")).unwrap();
    let index = WorldIndex::open(&dir, DEFAULT_TRACK).unwrap();
    let focus = Focus { position: [2395.0, 2913.0], heading: std::f32::consts::FRAC_PI_2, speed: 0.0, body: None };
    let mut collision = CollisionWorld::new(index.collision_grid.clone());
    for section in &index.sections {
        if !section.is_spatial() || section.distance_to(focus.position[0], focus.position[1]) > 1100.0 {
            continue;
        }
        let bytes = dir.read_range(&index.stream_file, u64::from(section.file_offset), section.size as usize).unwrap();
        for pack in blackbox_collision::read_collision_packs(&bytes).unwrap() {
            collision.insert(pack);
        }
    }
    let data = PhysicsData::load(&dir).unwrap();
    let mut renderer = NullBackend::new(Capabilities::baseline("null", GraphicsApi::Vulkan));
    for policy in [TrafficPolicy::default(), TrafficPolicy::extended()] {
        let mut world = TrafficWorld::new(index.road_network.clone().unwrap());
        world.models.push(model(&dir, &data, "traf4dseda", &mut renderer));
        world.configure(20, 0);
        world.set_policy(policy);
        assert!(
            (0..128).any(|_| world.spawn_one(Pick::Traffic(0), Cruise::default(), focus, &collision, &data.surfaces)),
            "no supported automatic placement for {policy:?}"
        );
        let position = world.cars[0].position();
        let distance = position.truncate().distance(glam::Vec2::from_array(focus.position));
        assert!(distance >= policy.spawn_min as f32 && distance <= policy.spawn_max as f32 + 50.0);
        assert_ne!(collision.resident_near(world.cars[0].physics_position().to_array(), 40.0), Some(false));
        let eye = Vec3::new(focus.position[0], focus.position[1], position.z + 3.0);
        let away = crate::viewer::camera::Lens::look_at(eye, eye - Vec3::Y, 60.0, 0.1, 1.0);
        world.recycle(0.5, focus, &away, &away, &PropWorld::default());
        assert_eq!(world.cars.len(), 1, "new distant actor must survive a short look away");
        eprintln!("traffic band {}-{}: supported spawn {distance:.1} m", policy.spawn_min, policy.spawn_max);
    }
}

#[test]
#[ignore = "needs installed traffic and world collision selected by NFSMW_GAME_DIR"]
fn installed_traffic_population_presets_increase_sustained_highway_counts() {
    use crate::settings::{Partial, PopulationPreset, Settings};
    let dir = GameDir::open(std::env::var_os("NFSMW_GAME_DIR").expect("set NFSMW_GAME_DIR")).unwrap();
    let index = WorldIndex::open(&dir, DEFAULT_TRACK).unwrap();
    let data = PhysicsData::load(&dir).unwrap();
    let mut collision = CollisionWorld::new(index.collision_grid.clone());
    for section in index.sections.iter().filter(|s| s.is_spatial()) {
        let bytes = dir.read_range(&index.stream_file, u64::from(section.file_offset), section.size as usize).unwrap();
        for pack in blackbox_collision::read_collision_packs(&bytes).unwrap() {
            collision.insert(pack);
        }
    }
    let mut renderer = NullBackend::new(Capabilities::baseline("null", GraphicsApi::Vulkan));
    let position = Vec3::new(2395.0, 2913.0, 146.6);
    let focus = Focus {
        position: position.truncate().to_array(),
        heading: std::f32::consts::FRAC_PI_2,
        speed: 0.0,
        body: Some(crate::scenes::world::ai::Body {
            position: Vec3::from(space::to_physics(position)),
            velocity: Vec3::ZERO,
            forward: glam::Vec2::NEG_X,
            half_width: 1.0,
            half_length: 2.5,
        }),
    };
    let eye = position + Vec3::new(0.0, -8.0, 3.0);
    let lens = crate::viewer::camera::Lens::look_at(eye, eye + Vec3::Y, 60.0, 0.1, 16.0 / 9.0);
    let mut means = Vec::new();
    for preset in [PopulationPreset::Normal, PopulationPreset::High, PopulationPreset::VeryHigh] {
        let mut settings = Settings::from(Partial::default());
        settings.select_traffic_preset(crate::settings::TrafficPreset::Extended);
        settings.select_population_preset(preset);
        let mut world = TrafficWorld::new(index.road_network.clone().unwrap());
        world.load_data(data.database(), index.track_zones.clone());
        world.configure(settings.traffic as usize, 0);
        world.set_policy(settings.traffic_policy);
        world.manage(0.5, focus, &collision, &data.surfaces);
        for request in world.take_model_requests() {
            let crate::scenes::world::ai::ModelRequest::Traffic(name) = request else { continue };
            let mut car = model(&dir, &data, &name, &mut renderer);
            if let Some(name) = data.trailer_of(&name) {
                car.trailer = Some(Box::new(model(&dir, &data, &name, &mut renderer)));
            }
            world.models.push(car);
        }
        let rules = world.patterns.as_ref().unwrap().current().unwrap().rules.clone();
        let mut props = PropWorld::default();
        let mut count = 0usize;
        for tick in 0..3600 {
            world.step(focus, &collision, &mut props, &data.surfaces);
            world.finish_step(focus, &lens, &lens, &props, &collision, &data.surfaces);
            assert!(world.population_slots(focus) <= settings.traffic as usize);
            for rule in rules.iter().filter(|r| r.max_instances > 0) {
                let active = world.cars.iter().filter(|c| c.name.eq_ignore_ascii_case(&rule.name)).count();
                assert!(active <= rule.max_instances as usize, "rare limit: {}", rule.name);
            }
            if tick >= 1800 {
                count += world.cars.len();
            }
        }
        let mean = count as f32 / 1800.0;
        eprintln!("{preset:?} mean actors t=30..60: {mean:.2}; {}", world.status(focus));
        assert_eq!(world.population_stats.waiting_collision, 0);
        means.push(mean);
        let spawned = world.population_stats.spawned;
        world.set_policy(TrafficPolicy { density_percent: 0, ..settings.traffic_policy });
        for _ in 0..60 {
            world.step(focus, &collision, &mut props, &data.surfaces);
            world.finish_step(focus, &lens, &lens, &props, &collision, &data.surfaces);
        }
        assert_eq!(spawned, world.population_stats.spawned);
    }
    assert!(means[1] > means[0] + 3.0, "High must increase sustained encounters: {means:?}");
    assert!(means[2] > means[1] + 1.0, "Very High must increase sustained encounters: {means:?}");
}
