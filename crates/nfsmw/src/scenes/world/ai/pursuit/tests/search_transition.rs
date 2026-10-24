//! Changing an existing pursuit car's action is distinct from admitting a fresh civilian spawn.

use blackbox_roads::{RoadNav, SplitMix, flags, forward_traffic_lanes, travel_profile};

use super::free_roam::renderer;
use super::*;

#[test]
fn an_existing_cop_enters_search_on_a_decision_connector_that_rejects_civilian_spawning() {
    let mut world = TrafficWorld::new(cop_road());
    world.add_cop_model(cop_model(&mut renderer(), "copx", player()));
    let spot = CopSpot {
        at: Vec3::new(2.0, 0.0, 20.0),
        target: Vec3::new(2.0, 0.0, 60.0),
        speed: 12.0,
        player: player(),
        band: None,
    };
    assert!(world.spawn_cop_at("copx", spot, &floor(), &SurfaceTable::default()));

    // The car is already driving on this connector. The initial cop placement deliberately precedes the flag change,
    // because a fresh scripted placement excludes decision connectors for its own independent admission rules.
    world.network.segments[0].flags |= flags::DECISION | flags::INTERSECTION;
    let car = &world.cars[0];
    let before = (car.audio_emitter().id, car.physics_position(), car.body().velocity, car.physics_forward());
    let mut fresh_spawn = RoadNav::traffic();
    assert!(fresh_spawn.init_at_point(
        &world.network,
        &world.index,
        car.physics_position(),
        car.physics_forward(),
        false,
    ));
    let profile = travel_profile(&world.network, fresh_spawn.segment, fresh_spawn.node_ind, false);
    assert!(!forward_traffic_lanes(&profile).is_empty(), "the connector has a valid forward traffic lane");
    assert!(world.network.segment(fresh_spawn.segment).is_decision());
    assert!(
        !fresh_spawn.can_traffic_spawn(&world.network, &mut SplitMix(17)),
        "fresh civilian spawning must remain forbidden on a decision connector"
    );

    let car = &mut world.cars[0];
    car.search_mode(true, Cruise::patrol(10.0, 20.0), &world.network, &world.index);
    assert!(car.is_searching(), "an existing cop must enter the search action without a civilian spawn gate");
    assert!(car.is_cop(), "changing the driving action must retain the police role");
    assert_eq!(
        (car.audio_emitter().id, car.physics_position(), car.body().velocity, car.physics_forward()),
        before,
        "changing the driving action must preserve the physical vehicle"
    );
}
