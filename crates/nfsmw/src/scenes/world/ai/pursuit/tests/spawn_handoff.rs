//! Automatic spawn entry, before the car has received any pursuit driving step.

use super::free_roam::renderer;
use super::*;

fn setup(one_way: bool) -> (TrafficWorld, Focus) {
    let mut network = straight_road(1000.0);
    if one_way {
        network.segments[0].flags |= blackbox_roads::flags::ONE_WAY;
    }
    let mut world = TrafficWorld::new(network);
    world.add_cop_model(cop_model(&mut renderer(), "copx", player()));
    let body = Body {
        position: Vec3::new(0.0, 0.0, -200.0),
        velocity: Vec3::Z * 20.0,
        forward: Vec2::Y,
        half_width: 1.0,
        half_length: 2.0,
    };
    let focus = Focus { position: [-200.0, 0.0], heading: 0.0, speed: 20.0, body: Some(body) };
    world.player_position = Some(body.position);
    world.start_pursuit(1, vec![wave()], player());
    (world, focus)
}

#[test]
fn an_automatic_reinforcement_enters_at_rest_before_its_first_ai_step() {
    let (mut world, focus) = setup(false);
    let target = focus.body.unwrap().position;
    let (collision, surfaces) = (floor(), SurfaceTable::default());
    assert!(world.cars.is_empty());
    assert!((0..60).any(|_| world.spawn_cop("copx", focus, target, player(), &collision, &surfaces)));
    assert_eq!(world.cars.len(), 1);
    assert!(world.cars[0].speed().abs() < 0.01, "automatic spawn is not given an invented cruise speed");
}

#[test]
fn a_seek_walk_reversed_cursor_survives_the_production_spawn_on_a_one_way_road() {
    let (mut world, focus) = setup(true);
    let target = focus.body.unwrap().position;
    let (collision, surfaces) = (floor(), SurfaceTable::default());
    assert!(world.cars.is_empty());
    assert!(world.spawn_cop("copx", focus, target, player(), &collision, &surfaces));
    let cop = &world.cars[0];
    assert!((150.0..=400.0).contains(&cop.physics_position().distance(target)));
    assert_eq!(cop.navigation().node_ind, 0, "re-locating the accepted cursor would silently restore one-way travel");
    assert!(cop.navigation().forward.dot(-Vec3::Z) > 0.9, "the reversed road cursor is the one passed to the car");
}

#[test]
fn an_unfilled_wave_alternates_fast_joins_after_the_initial_three() {
    let (mut world, focus) = setup(true);
    let (collision, surfaces) = (floor(), SurfaceTable::default());
    let mut row = wave();
    row.cops[0].count = 4;
    row.full_engagement_cops = 5;
    row.time_between_first_four_spawns = 1.0;
    row.time_between_cop_spawns = 2.0;
    world.start_pursuit(1, vec![row], player());
    let mut delays = Vec::new();
    for _ in 0..5 {
        // Clear the previous far, unengaged unit: the next request still lacks four cars.
        world.cars.clear();
        world.pursuit.as_mut().unwrap().spawn_timer = -1.0;
        world.update_pursuit(0.25, focus, &collision, &surfaces);
        assert_eq!(world.cars.len(), 1, "the production manager placed its requested unit");
        delays.push(world.pursuit.as_ref().unwrap().spawn_timer);
    }
    assert_eq!(delays, [1.0, 1.0, 1.0, 2.0, 0.2]);
    let before = world.pursuit.as_ref().unwrap().involved;
    world.cars.clear();
    world.cop_models.clear();
    world.pursuit.as_mut().unwrap().spawn_timer = -1.0;
    world.update_pursuit(0.25, focus, &collision, &surfaces);
    let pursuit = world.pursuit.as_ref().unwrap();
    assert_eq!(pursuit.involved, before, "an unavailable model is not another join");
    assert!(pursuit.spawn_timer < 0.0, "failure does not arm a delay");
}

#[test]
fn pending_spawn_decisions_change_the_world_hash_before_any_car_moves() {
    use crate::scenes::world::StateHasher;

    let (mut world, _) = setup(true);
    let hash = |world: &TrafficWorld| {
        let mut hasher = StateHasher::default();
        world.hash_into(&mut hasher);
        hasher.finish()
    };
    let baseline = hash(&world);
    world.pursuit.as_mut().unwrap().fast_spawn_next = true;
    assert_ne!(baseline, hash(&world), "the next successful join can arm a different deadline");
    world.pursuit.as_mut().unwrap().fast_spawn_next = false;
    world.pursuit.as_mut().unwrap().requested_missing = 3;
    assert_ne!(baseline, hash(&world), "the remembered request deficit affects future joins");
    world.pursuit.as_mut().unwrap().requested_missing = 0;
    world.pursuit.as_mut().unwrap().spawn_timer = 0.2;
    assert_ne!(baseline, hash(&world), "the next placement opportunity is part of the state");
    world.pursuit.as_mut().unwrap().spawn_timer = 0.0;
    world.pursuit.as_mut().unwrap().involved = 3;
    assert_ne!(baseline, hash(&world), "the early-join branch changes after three units");
    assert!(world.cars.is_empty(), "all compared states have the same empty fleet");
}

fn outer_lane(world: &mut TrafficWorld) -> RoadNav {
    world.network.profiles[0] = RoadProfile {
        middle: 2,
        zones: [6.0, 2.0, 2.0, 6.0].map(|offset| Zone { kind: zone::TRAFFIC, width: 4.0, offset }).to_vec(),
    };
    let mut nav = RoadNav::new(NavKind::Direction, LaneType::Cop, SegmentFilter::default());
    nav.place(&world.network, 0, 1, 3, 0.5);
    nav
}

#[test]
fn a_rolling_cop_body_and_initial_drive_target_use_the_same_prepared_road_cursor() {
    use blackbox_driver::{ControllerKind, DriveFlags, DriveRequest, Driver, VehicleView};

    let (mut world, _) = setup(false);
    let nav = outer_lane(&mut world);
    let mut prepared = nav.clone();
    prepared.half_width = world.cop_models[0].model.physics.spec.dimension.x;
    prepared.enable_trail(&world.network);
    assert!(prepared.position.distance(nav.position) > 0.1, "the fixture must exercise the outer-lane clamp");
    let spot = CopSpot { at: nav.position, target: Vec3::Z * 70.0, speed: 30.0, player: player(), band: None };
    assert!(world.spawn_cop_on_road("copx", spot, nav, &floor(), &SurfaceTable::default()));
    let car = &world.cars[0];
    let position = car.physics_position();
    let planar = |point: Vec3| Vec2::new(point.x, point.z);
    assert!(planar(position).distance(planar(prepared.position)) < 1e-4, "the body uses the prepared road point");
    assert!(car.physics_forward().dot(prepared.forward) > 0.9999, "the body uses the prepared heading");
    let (target, speed) = car.drive_request();
    let mut driver = Driver::new(ControllerKind::Pid, false);
    let controls = driver.step(
        &VehicleView {
            position,
            forward: car.physics_forward(),
            forward_speed: car.speed(),
            planar_speed: car.body().velocity.length(),
            gear_is_reverse: false,
            max_steer: std::f32::consts::FRAC_PI_4,
            in_shock: false,
            staging: false,
        },
        &DriveRequest { target, speed, flags: DriveFlags::FULL },
        crate::scenes::world::drive::STEP,
        0.0,
    );
    assert!(
        controls.steer.abs() < 1e-4,
        "before the first pursuit think, an old lane point must not force a lateral turn"
    );
}

#[test]
fn cop_spawn_exclusion_zones_test_the_prepared_trail_position() {
    let (mut world, _) = setup(false);
    let nav = outer_lane(&mut world);
    let mut prepared = nav.clone();
    prepared.half_width = world.cop_models[0].model.physics.spec.dimension.x;
    prepared.enable_trail(&world.network);
    let radius = prepared.position.x.abs() + 0.1;
    assert!(nav.position.x.abs() > radius, "only the prepared point lies inside the exclusion zone");
    world.cops.set_no_spawn_zones(vec![crate::scenes::world::ai::cop_manager::square_zone(radius)]);
    let spot = CopSpot { at: nav.position, target: Vec3::Z * 70.0, speed: 30.0, player: player(), band: None };
    assert!(!world.spawn_cop_on_road("copx", spot, nav, &floor(), &SurfaceTable::default()));
    assert!(world.cars.is_empty(), "clamping a cursor into an excluded area must not admit a vehicle");
}
