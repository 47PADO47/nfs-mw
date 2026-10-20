//! The world raises the events the chase radio remarks on: hits between cars, a cop against a wall, a pass, a tunnel,
//! the road's direction, a roadblock's building and dodge, and the backup calls. Each is read from the feed's event log.

use blackbox_pursuit::speech::{Backup, Impact, RoadblockHit};
use blackbox_roads::TrackZone;
use glam::Quat;

use super::free_roam::{patrol, renderer, see};
use super::*;
use crate::audio::WorldEvent;
use crate::scenes::world::ai::PlayerView;
use crate::scenes::world::ai::roadblock::Cause;
use crate::scenes::world::drive::walls::ContactKind;
use crate::scenes::world::drive::{CarSim, SurfaceHit};

fn events(world: &TrafficWorld) -> Vec<WorldEvent> {
    world.speech_watch.log.all().copied().collect()
}

/// A player car at `at`, heading along +z at `speed`.
fn player_car(at: Vec3, speed: f32) -> CarSim {
    let spec = VehicleSpec::example();
    let bounds = CarBounds { half_dimensions: spec.dimension, pivot: Vec3::ZERO };
    let mut sim = CarSim::new(
        CarPhysics {
            tire_integration: Default::default(),
            jump_config: Default::default(),
            spec,
            bounds,
            walls: WallSpec::default(),
            geometry: None,
            events: None,
            reactions: None,
        },
        [0.2; 4],
    );
    sim.body_mut().place(at, Quat::IDENTITY);
    sim.body_mut().linear_velocity = Vec3::new(0.0, 0.0, speed);
    sim
}

/// A chase with one cop (a patrol car that joined) 20 m up the road, driving at 10 m/s.
fn chase_with_a_cop() -> TrafficWorld {
    let mut world = TrafficWorld::new(cop_road());
    patrol(&mut world, &mut renderer(), Vec3::new(2.0, 0.0, 20.0));
    world.player_position = Some(Vec3::ZERO);
    world.start_pursuit(1, vec![wave()], player());
    world
}

/// A traffic car (not a cop) at `at`.
fn traffic_car(world: &mut TrafficWorld, at: Vec3) {
    let model = cop_model(&mut renderer(), "traffic", player());
    let mut nav = RoadNav::traffic();
    assert!(nav.init_at_point(&world.network, &world.index, at, Vec3::Z, false));
    let heading = space::to_render(Vec3::Z.to_array());
    let car = AiCar::place(
        &model.model,
        Role::Traffic,
        nav,
        Spawn { position: space::to_render(at.to_array()), heading: heading.y.atan2(heading.x) },
        Start { speed: 5.0, stagger: 0, cruise: Cruise::default(), patrol: false },
        &WorldGround { collision: &floor(), surfaces: &SurfaceTable::default() },
    )
    .unwrap();
    world.cars.push(car);
}

#[test]
fn the_player_driving_into_the_back_of_a_cop_is_a_deliberate_rear_end() {
    let mut world = chase_with_a_cop();
    let cop = world.cars[0].physics_position();
    let mut player = player_car(cop - Vec3::new(0.0, 0.0, 4.0), 30.0);
    world.collide(Some(&mut player));
    let hits: Vec<_> = events(&world)
        .into_iter()
        .filter_map(|e| match e {
            WorldEvent::PlayerHitCop { cop, crash, impact, deliberate } => Some((cop, crash, impact, deliberate)),
            _ => None,
        })
        .collect();
    let [(id, crash, impact, deliberate)] = hits[..] else { panic!("{:?}", events(&world)) };
    assert_eq!(id, world.cars[0].speech_id());
    assert_eq!(impact, Impact::RearEnd);
    assert!(deliberate);
    assert!(crash.closing > 15.0 && crash.delta_v > 0.0, "{crash:?}");
    assert!(crash.speed_before > crash.speed_after, "{crash:?}");
}

#[test]
fn a_roadblock_car_that_is_hit_is_no_deliberate_ram_and_is_a_hit_on_the_roadblock() {
    let mut world = chase_with_a_cop();
    world.cars[0].set_roadblock();
    let cop = world.cars[0].physics_position();
    let mut player = player_car(cop - Vec3::new(0.0, 0.0, 4.0), 30.0);
    world.collide(Some(&mut player));
    let log = events(&world);
    assert!(log.iter().any(|e| matches!(e, WorldEvent::PlayerHitCop { deliberate: false, .. })), "{log:?}");
    assert!(log.contains(&WorldEvent::RoadblockHit(RoadblockHit::Vehicle)), "{log:?}");
}

#[test]
fn the_player_hitting_a_traffic_car_is_a_hit_on_traffic() {
    let mut world = chase_with_a_cop();
    traffic_car(&mut world, Vec3::new(-2.0, 0.0, -40.0));
    let car = world.cars[1].physics_position();
    let mut player = player_car(car - Vec3::new(0.0, 0.0, 4.0), 25.0);
    world.collide(Some(&mut player));
    let log = events(&world);
    let hit = log.iter().find_map(|e| match e {
        WorldEvent::PlayerHitTraffic { crash, semi } => Some((*crash, *semi)),
        _ => None,
    });
    let (crash, semi) = hit.unwrap_or_else(|| panic!("{log:?}"));
    assert!(!semi);
    assert!(crash.closing > 15.0, "{crash:?}");
    assert!(!log.iter().any(|e| matches!(e, WorldEvent::PlayerHitCop { .. })), "a traffic car is no cop");
}

#[test]
fn a_cop_driving_into_traffic_is_a_cop_hit_on_traffic() {
    let mut world = chase_with_a_cop();
    let cop = world.cars[0].physics_position();
    traffic_car(&mut world, cop + Vec3::new(0.0, 0.0, 4.0));
    world.collide(None);
    let id = world.cars[0].speech_id();
    let log = events(&world);
    assert!(log.iter().any(|e| matches!(e, WorldEvent::CopHitTraffic { cop, .. } if *cop == id)), "{log:?}");
}

#[test]
fn a_cop_that_hit_a_wall_is_told_once() {
    let mut world = chase_with_a_cop();
    let id = world.cars[0].speech_id();
    world.cars[0].force_surface_hit(SurfaceHit { kind: ContactKind::Barrier, delta_v: 12.0, closing: 16.0 });
    world.watch_cops_against_walls();
    world.watch_cops_against_walls();
    assert_eq!(events(&world), vec![WorldEvent::CopHitWorld { cop: id, delta_v: 12.0, closing: 16.0 }]);
}

#[test]
fn a_cop_that_scraped_a_prop_is_not_a_wall_hit() {
    let mut world = chase_with_a_cop();
    world.cars[0].force_surface_hit(SurfaceHit { kind: ContactKind::PropRigid, delta_v: 12.0, closing: 16.0 });
    world.watch_cops_against_walls();
    assert!(events(&world).is_empty());
}

#[test]
fn a_cop_that_passes_fast_is_blown_by_once_a_second() {
    let mut world = chase_with_a_cop();
    let id = world.cars[0].speech_id();
    world.note_pass(0, 14.0);
    assert!(events(&world).is_empty(), "too slow a pass");
    world.note_pass(0, 20.0);
    world.note_pass(0, 22.0);
    assert_eq!(events(&world), vec![WorldEvent::BlewByCop { cop: id }]);
    world.tick_speech_watch(1.5);
    world.note_pass(0, 20.0);
    assert_eq!(events(&world).len(), 2, "after the pause it is told again");
}

#[test]
fn traffic_passing_fast_is_not_a_blown_by_cop() {
    let mut world = chase_with_a_cop();
    traffic_car(&mut world, Vec3::new(-2.0, 0.0, -40.0));
    world.note_pass(1, 30.0);
    assert!(events(&world).is_empty());
}

fn tunnel_zone() -> TrackZone {
    let corner = |x, y| Vec2::new(x, y);
    TrackZone {
        kind: 3,
        position: Vec2::ZERO,
        direction: Vec2::X,
        elevation: 0.0,
        visit_info: 0,
        data: [0; 4],
        bbox_min: Vec2::splat(-30.0),
        bbox_max: Vec2::splat(30.0),
        polygon: vec![corner(-30.0, -30.0), corner(30.0, -30.0), corner(30.0, 30.0), corner(-30.0, 30.0)],
    }
}

fn drive_to(world: &mut TrafficWorld, z: f32) {
    let view = PlayerView {
        position: space::to_render([0.0, 0.0, z]),
        forward: space::to_render(Vec3::Z.to_array()),
        speed: 20.0,
        nitrous: 0.0,
        throttle: 0.5,
    };
    world.update_police(0.25, &view, &|_, _| true);
}

#[test]
fn driving_into_and_out_of_a_tunnel_is_told_on_each_edge() {
    let mut world = chase_with_a_cop();
    world.set_speech_zones(vec![tunnel_zone()]);
    drive_to(&mut world, -100.0);
    assert!(events(&world).is_empty());
    drive_to(&mut world, 0.0);
    drive_to(&mut world, 5.0);
    drive_to(&mut world, 100.0);
    assert_eq!(events(&world), vec![WorldEvent::Tunnel { inside: true }, WorldEvent::Tunnel { inside: false }]);
}

#[test]
fn the_roads_direction_follows_the_car_and_stays_put_when_the_car_turns_round() {
    let mut world = chase_with_a_cop();
    see(&mut world, true);
    let forward = world.pursuit_feed().unwrap().speech.player.road_forward.expect("on the road");
    assert!(forward[2].abs() > 0.99, "the road runs along z: {forward:?}");
    let view = |dir: Vec3| PlayerView {
        position: Vec3::ZERO,
        forward: space::to_render(dir.to_array()),
        speed: 10.0,
        nitrous: 0.0,
        throttle: 0.0,
    };
    world.update_police(0.25, &view(Vec3::Z), &|_, _| true);
    let along = world.pursuit_feed().unwrap().speech.player.road_forward.unwrap();
    world.update_police(0.25, &view(-Vec3::Z), &|_, _| true);
    let turned = world.pursuit_feed().unwrap().speech.player.road_forward.unwrap();
    assert_eq!(along, turned, "a U-turn does not turn the road round");
    assert!(along[2] > 0.99);
}

#[test]
fn a_built_roadblock_reports_its_spike_strip_and_a_dodge_is_told_once() {
    let mut world = TrafficWorld::new(straight_road(1000.0));
    world.add_cop_model(cop_model(&mut renderer(), "copx", player()));
    world.start_pursuit(9, vec![wave()], player());
    let view = |z: f32| PlayerView {
        position: space::to_render([0.0, 0.0, z]),
        forward: space::to_render(Vec3::Z.to_array()),
        speed: 20.0,
        nitrous: 0.0,
        throttle: 0.5,
    };
    world.build_roadblock(Cause::Console, view(-150.0), &floor(), None, &SurfaceTable::default()).unwrap();
    let built = events(&world);
    assert!(
        matches!(built.last(), Some(WorldEvent::RoadblockBuilt { spike_offset: Some(_) })),
        "heat 9 takes the spike layouts: {built:?}"
    );
    for z in [-150.0, 160.0, 200.0] {
        world.update_roadblocks(0.25, view(z), &floor(), None, &SurfaceTable::default());
    }
    let dodged = events(&world).iter().filter(|e| **e == WorldEvent::RoadblockDodged).count();
    assert_eq!(dodged, 1);
}

fn backup(world: &TrafficWorld) -> Vec<WorldEvent> {
    events(world)
        .into_iter()
        .filter(|e| matches!(e, WorldEvent::BackupRequested(_) | WorldEvent::BackupDenied(_)))
        .collect()
}

#[test]
fn a_ram_request_calls_the_rhino_units_and_a_leader_request_the_super_pursuit_ones() {
    let (collision, surfaces) = (floor(), SurfaceTable::default());
    let mut world = TrafficWorld::new(straight_road(300.0));
    world.start_pursuit(1, vec![wave()], player());
    world.add_cop_model(cop_model(&mut renderer(), "copsuvl", player()));
    world.add_cop_model(cop_model(&mut renderer(), "copcross", player()));
    assert!(world.ask_support(support(Strategy::Ram)));
    world.update_pursuit(FRAME, support_focus(-150.0), &collision, &surfaces);
    assert_eq!(backup(&world), vec![WorldEvent::BackupRequested(Backup::Rhino)]);
    world.pursuit.as_mut().unwrap().support = Default::default();
    assert!(world.ask_support(support(Strategy::CrossFollow)));
    world.update_pursuit(FRAME, support_focus(0.0), &collision, &surfaces);
    assert_eq!(backup(&world).last(), Some(&WorldEvent::BackupRequested(Backup::SuperPursuit)));
}

#[test]
fn a_leader_request_that_finds_no_place_is_denied() {
    let (collision, surfaces) = (floor(), SurfaceTable::default());
    let mut world = TrafficWorld::new(straight_road(300.0));
    world.start_pursuit(1, vec![wave()], player());
    world.add_cop_model(cop_model(&mut renderer(), "copcross", player()));
    assert!(world.ask_support(support(Strategy::CrossFollow)));
    // The player stands far off the end of the road: the car behind it lies off the world.
    world.update_pursuit(FRAME, support_focus(-10_000.0), &collision, &surfaces);
    let told = backup(&world);
    assert!(told.iter().any(|e| matches!(e, WorldEvent::BackupDenied(Backup::SuperPursuit))), "{told:?}");
    assert!(!told.iter().any(|e| matches!(e, WorldEvent::BackupRequested(_))));
}

#[test]
fn the_feed_carries_what_was_noted() {
    let mut world = chase_with_a_cop();
    world.note_speech(WorldEvent::TireBlown);
    let feed = world.pursuit_feed().unwrap();
    assert_eq!(feed.speech.events.all().copied().collect::<Vec<_>>(), vec![WorldEvent::TireBlown]);
}
