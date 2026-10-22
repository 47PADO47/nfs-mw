//! What a server replicates of the traffic world (`net_export.rs`, `docs/specs/multiplayer/world-host.md`): the cars in
//! entity-id order with their kind, model and motion after the tick. The installed world's export is tested in
//! `scenes/world/net/tests`.

use super::*;
use crate::scenes::world::ai::NetCarKind;
use crate::scenes::world::props::PropWorld;

/// A chase with a cop of model `copx` on the cop road, stepped `ticks` times.
fn chase_with_a_cop(ticks: u32) -> TrafficWorld {
    let mut renderer = NullBackend::new(Capabilities::baseline("null", GraphicsApi::Vulkan));
    let collision = floor();
    let surfaces = SurfaceTable::default();
    let mut props = PropWorld::default();
    let mut world = TrafficWorld::new(cop_road());
    world.add_cop_model(cop_model(&mut renderer, "copx", player()));
    let at = Vec3::new(2.0, 0.0, -40.0);
    world.player_position = Some(at);
    world.start_pursuit(1, vec![wave()], player());
    let spot = CopSpot { at: Vec3::new(2.0, 0.0, -55.0), target: at, speed: 12.0, player: player(), band: None };
    assert!(world.spawn_cop_at("copx", spot, &collision, &surfaces));
    for _ in 0..ticks {
        world.step(focus(), &collision, &mut props, &surfaces);
    }
    world
}

#[test]
fn the_cars_come_in_entity_order_as_police_with_their_model_and_motion() {
    let world = chase_with_a_cop(30);
    let cars: Vec<_> = world.net_cars().collect();
    assert!(!cars.is_empty(), "the cop is in the world");
    let ids: Vec<u32> = cars.iter().map(|car| car.entity).collect();
    let mut sorted = ids.clone();
    sorted.sort_unstable();
    sorted.dedup();
    assert_eq!(ids, sorted, "ascending and unique");
    for car in cars.iter().filter(|car| car.model == "copx") {
        assert_eq!(car.kind, NetCarKind::Police);
        assert!(!car.wrecked);
        assert!(car.position.is_finite() && car.rotation.is_finite() && car.linear_velocity.is_finite());
        assert!((car.rotation.length() - 1.0).abs() < 1e-3, "a unit quaternion");
    }
    let cop = world.cars.iter().find(|car| car.name == "copx").expect("a cop");
    let exported = cars.iter().find(|car| car.entity == cop.key()).expect("the cop is exported");
    assert_eq!(exported.position, cop.position(), "the pose after the tick, render space");
    assert!(world.net_helicopter().is_none(), "no helicopter in this chase");
    assert_eq!(world.spike_strips().count(), 0, "no roadblock stands");
}
