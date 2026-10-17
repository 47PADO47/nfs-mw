//! Real automatic placement at both distance presets, without a GPU or a running original game.
use super::*;
use crate::scenes::world::ai::{Focus, TrafficWorld, spawn::Pick, traffic::Cruise};
use crate::settings::TrafficPolicy;

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
        world.recycle(0.5, focus, &away, &PropWorld::default());
        assert_eq!(world.cars.len(), 1, "new distant actor must survive a short look away");
        eprintln!("traffic band {}-{}: supported spawn {distance:.1} m", policy.spawn_min, policy.spawn_max);
    }
}
