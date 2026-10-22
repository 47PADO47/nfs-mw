//! Each world numbers its own cars (`ai/entities.rs`, `docs/specs/multiplayer/determinism.md` D5): a server and a client
//! in one process do not take each other's ids, and the cars step in id order.

use super::*;

/// A world with a chase and `cops` cops placed 45 m apart along the cop road (a chase spawn keeps 40 m from every car).
fn world_with_cops(renderer: &mut NullBackend, cops: usize) -> TrafficWorld {
    let collision = floor();
    let surfaces = SurfaceTable::default();
    let mut world = TrafficWorld::new(cop_road());
    world.add_cop_model(cop_model(renderer, "copx", player()));
    world.player_position = Some(Vec3::new(2.0, 0.0, -40.0));
    world.start_pursuit(1, vec![wave()], player());
    for i in 0..cops {
        let at = Vec3::new(2.0, 0.0, -65.0 + 45.0 * i as f32);
        let spot = CopSpot { at, target: Vec3::new(2.0, 0.0, -40.0), speed: 0.0, player: player(), band: None };
        assert!(world.spawn_cop_at("copx", spot, &collision, &surfaces), "cop {i} must stand on the road");
    }
    world
}

#[test]
fn two_worlds_in_one_process_number_their_cars_independently() {
    let mut renderer = NullBackend::new(Capabilities::baseline("null", GraphicsApi::Vulkan));
    let server = world_with_cops(&mut renderer, 3);
    let client = world_with_cops(&mut renderer, 2);
    assert_eq!(server.entity_ids().collect::<Vec<_>>(), [1, 2, 3]);
    assert_eq!(client.entity_ids().collect::<Vec<_>>(), [1, 2], "the server's cars do not advance the client's ids");
}

#[test]
fn a_removed_car_keeps_the_others_in_id_order() {
    let mut renderer = NullBackend::new(Capabilities::baseline("null", GraphicsApi::Vulkan));
    let mut world = world_with_cops(&mut renderer, 3);
    world.cars.remove(1);
    let collision = floor();
    let spot = CopSpot {
        at: Vec3::new(2.0, 0.0, -20.0),
        target: Vec3::new(2.0, 0.0, -40.0),
        speed: 0.0,
        player: player(),
        band: None,
    };
    assert!(world.spawn_cop_at("copx", spot, &collision, &SurfaceTable::default()));
    assert_eq!(world.entity_ids().collect::<Vec<_>>(), [1, 3, 4], "an id is never given twice");
}
