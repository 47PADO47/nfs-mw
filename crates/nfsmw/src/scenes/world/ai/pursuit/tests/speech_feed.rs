//! The world feeds the chase radio: the cops with their sight, distance and state, the player's road and heading, and how
//! the pursuit stands.

use super::*;
use crate::audio::Vehicle;
use crate::scenes::world::ai::PlayerView;

fn chasing_cop(world: &mut TrafficWorld, at: Vec3) {
    let mut renderer = NullBackend::new(Capabilities::baseline("null", GraphicsApi::Vulkan));
    let model = cop_model(&mut renderer, "copx", player());
    let mut nav = RoadNav::traffic();
    assert!(nav.init_at_point(&world.network, &world.index, at, Vec3::Z, false));
    let position = space::to_render(at.to_array());
    let heading = space::to_render(Vec3::Z.to_array());
    let car = AiCar::place(
        &model.model,
        Role::Traffic,
        nav,
        Spawn { position, heading: heading.y.atan2(heading.x) },
        Start { speed: 10.0, stagger: 0, cruise: Cruise::default(), patrol: true },
        &WorldGround { collision: &floor(), surfaces: &SurfaceTable::default() },
    )
    .unwrap();
    world.cars.push(car);
    world.add_cop_model(model);
}

fn tick(world: &mut TrafficWorld) {
    world.update_sight(0.25, &|_, _| None);
    world.update_police(
        0.25,
        &PlayerView {
            nos_input: false,
            nitrous_boost: 0.0,
            velocity: (Vec3::X) * (10.0),
            position: Vec3::ZERO,
            forward: Vec3::X,
            speed: 10.0,
            nitrous: 0.0,
            throttle: 0.0,
        },
        &|_, _| true,
    );
}

fn running() -> TrafficWorld {
    let mut world = TrafficWorld::new(cop_road());
    chasing_cop(&mut world, Vec3::new(2.0, 0.0, 20.0));
    world.player_position = Some(Vec3::ZERO);
    world.start_pursuit(1, vec![wave()], player());
    // Radio fixtures hold the cars still rather than running their physical AI tasks.
    // Supply their clear observation explicitly; task cadence is covered in visibility.rs.
    for car in &mut world.cars {
        car.report_sight(false, true);
    }
    world.player_speed = 40.0;
    for _ in 0..8 {
        tick(&mut world);
    }
    world
}

#[test]
fn the_chase_feed_lists_the_cop_with_its_distance_and_sight() {
    let world = running();
    let feed = world.pursuit_feed().expect("a chase");
    let cops = feed.speech.cops();
    assert_eq!(cops.len(), 1, "{cops:?}");
    let cop = cops[0];
    assert!(cop.active && !cop.destroyed);
    assert!(cop.id != 0, "the cop has an identity");
    assert!((15.0..30.0).contains(&cop.distance), "{}", cop.distance);
    assert!(cop.los, "the world's sight test sees the player and the cop is in range");
    assert_eq!(cop.vehicle, Vehicle::Patrol);
    assert!(cop.health > 0.99);
}

#[test]
fn another_cops_global_sighting_does_not_give_a_blocked_cop_radio_sight() {
    let mut world = running();
    world.cars[0].report_sight(false, false);
    world.seen_by_cops = true;
    let feed = world.pursuit_feed().unwrap();
    assert!(!feed.speech.cops()[0].los);
}

#[test]
fn the_player_is_on_the_named_road_and_heads_the_way_it_faces() {
    let world = running();
    let player = world.pursuit_feed().unwrap().speech.player;
    assert_eq!(player.road, Some(0), "the test road has the unnamed road id");
    let fw = player.forward;
    assert!((fw[0] * fw[0] + fw[1] * fw[1] + fw[2] * fw[2] - 1.0).abs() < 1e-3, "{fw:?}");
    assert!(player.velocity_dir.iter().any(|v| v.abs() > 0.5), "moving at 10 m/s: {:?}", player.velocity_dir);
}

#[test]
fn a_wrecked_cop_stays_in_the_feed_as_destroyed_and_inactive() {
    let mut world = running();
    assert!(world.cars[0].destroy_by_breaker());
    let feed = world.pursuit_feed().unwrap();
    let cop = feed.speech.cops()[0];
    assert!(cop.destroyed && !cop.active, "{cop:?}");
    assert!(cop.health <= 1.0);
}

#[test]
fn a_roadblock_car_is_listed_as_one() {
    let mut world = running();
    world.cars[0].set_roadblock();
    let feed = world.pursuit_feed().unwrap();
    assert!(feed.speech.cops().iter().any(|c| c.roadblock), "{:?}", feed.speech.cops());
}

#[test]
fn the_infractions_of_the_chase_ride_in_the_feed() {
    let world = running();
    let feed = world.pursuit_feed().unwrap();
    let log = feed.speech.status.infractions;
    assert_eq!(log, *world.infractions());
}
