use super::*;
use crate::scenes::world::ai::PlayerView;

fn patrol(world: &mut TrafficWorld, renderer: &mut NullBackend, at: Vec3) {
    let model = cop_model(renderer, "copx", player());
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

fn see(world: &mut TrafficWorld, seen: bool) {
    world.seen_by_cops = seen;
    world.update_police(
        0.25,
        &PlayerView { position: Vec3::ZERO, forward: Vec3::X, speed: 10.0, nitrous: 0.0, throttle: 0.0 },
        &|_, _| true,
    );
}

fn renderer() -> NullBackend {
    NullBackend::new(Capabilities::baseline("null", GraphicsApi::Vulkan))
}

#[test]
fn a_nearby_patrol_joins_in_place_with_its_identity_speed_and_damage() {
    let mut world = TrafficWorld::new(cop_road());
    patrol(&mut world, &mut renderer(), Vec3::new(2.0, 0.0, 20.0));
    let identity = world.cars[0].audio_emitter().id;
    let position = world.cars[0].physics_position();
    let speed = world.cars[0].speed();
    world.player_position = Some(Vec3::ZERO);
    world.start_pursuit(1, vec![wave()], player());
    assert!(world.cars[0].is_cop());
    assert!(!world.cars[0].is_patrol());
    assert_eq!(world.cars[0].audio_emitter().id, identity);
    assert_eq!(world.cars[0].physics_position(), position);
    assert_eq!(world.cars[0].speed(), speed);
    assert_eq!(world.pursuit.as_ref().unwrap().involved, 1);
    assert!(world.cars[0].emergency_lights());
    assert!((world.cars[0].physics_forward() - Vec3::Z).length() < 1e-5, "physics direction is not converted twice");
}

#[test]
fn tracked_sight_keeps_the_grace_then_search_remembers_and_reacquisition_updates() {
    let mut world = TrafficWorld::new(cop_road());
    world.start_pursuit(1, vec![wave()], player());
    let first = focus().body.unwrap();
    see(&mut world, true);
    world.pursuit_target(Some(first));
    see(&mut world, false);
    let hidden = Body { position: Vec3::new(30.0, 0.0, 50.0), velocity: Vec3::Z * 25.0, ..first };
    assert_eq!(world.pursuit_target(Some(hidden)).unwrap().position, hidden.position, "tracked grace remains a chase");
    for _ in 0..32 {
        see(&mut world, false);
    }
    let target = world.pursuit_target(Some(hidden)).unwrap();
    assert_eq!(target.position, first.position);
    assert_eq!(target.velocity, Vec3::ZERO);
    let mut aims = Vec::new();
    world.cop_aims(&target, &mut aims);
    assert!(aims.iter().all(Option::is_none), "search has no chase formation or stationary-point target");
    see(&mut world, true);
    assert_eq!(world.pursuit_target(Some(hidden)).unwrap().position, hidden.position);
}

#[test]
fn search_replacements_spawn_on_lanes_in_search_mode() {
    let mut world = TrafficWorld::new(straight_road(1000.0));
    world.add_cop_model(cop_model(&mut renderer(), "copx", player()));
    let mut row = wave();
    row.patrol_cars = 1;
    row.cops[0].count = 3;
    world.start_pursuit(1, vec![row], player());
    see(&mut world, true);
    for _ in 0..32 {
        see(&mut world, false);
    }
    assert!(!world.police.is_perp_in_sight());
    for _ in 0..120 {
        world.update_pursuit(1.0, focus(), &floor(), &SurfaceTable::default());
    }
    assert_eq!(cop_count(&world), 1, "cooldown replacements stop at NumPatrolCars instead of restoring the full wave");
    assert!(world.cars.iter().all(AiCar::is_searching));
    assert!(world.cars.iter().all(|car| {
        let distance = car.physics_position().length();
        (149.0..401.0).contains(&distance)
    }));
}

#[test]
fn police_switch_to_road_search_and_back_without_replacing_or_teleporting() {
    let mut world = TrafficWorld::new(cop_road());
    patrol(&mut world, &mut renderer(), Vec3::new(2.0, 0.0, 20.0));
    world.player_position = Some(Vec3::ZERO);
    world.start_pursuit(1, vec![wave()], player());
    let car = &mut world.cars[0];
    let before = (car.audio_emitter().id, car.physics_position(), car.body().velocity);
    let cruise = Cruise::patrol(10.0, 20.0);
    car.search_mode(true, cruise, &world.network, &world.index, &mut world.rng);
    assert!(car.is_searching());
    assert_eq!((car.audio_emitter().id, car.physics_position(), car.body().velocity), before);
    car.search_mode(false, cruise, &world.network, &world.index, &mut world.rng);
    assert!(!car.is_searching());
    assert_eq!((car.audio_emitter().id, car.physics_position(), car.body().velocity), before);
}

#[test]
fn patrol_work_and_model_requests_continue_with_civilian_traffic_disabled() {
    let mut world = TrafficWorld::new(cop_road());
    world.configure(0, 5);
    let mut plan = super::super::super::PatrolPlan::new(vec!["copx".into()], 50.0, 71.0);
    let mut row = wave();
    row.patrol_cars = 3;
    plan.levels = vec![row];
    world.set_patrol(plan);
    assert!(world.has_work());
    world.manage(0.1, focus(), &floor(), &SurfaceTable::default());
    assert_eq!(world.take_model_requests(), vec![super::super::super::ModelRequest::Patrol("copx".into())]);
    world.configure(0, 0);
    assert!(!world.has_work(), "zero remains the explicit patrol-off switch");
}

#[test]
fn patrol_population_follows_the_heat_row_even_with_zero_civilian_slots() {
    let mut world = TrafficWorld::new(straight_road(1000.0));
    // Put the production spawn band over the synthetic floor, with room for three stationary units.
    let focus = Focus {
        position: [-200.0, 0.0],
        body: Some(Body { position: -Vec3::Z * 200.0, ..focus().body.unwrap() }),
        ..focus()
    };
    world.add_cop_model(cop_model(&mut renderer(), "copx", player()));
    let mut row = wave();
    row.patrol_cars = 3;
    let mut plan = super::super::super::PatrolPlan::new(vec!["copx".into()], 50.0, 71.0);
    plan.levels = vec![row];
    world.set_patrol(plan);
    world.configure(0, 5);
    for _ in 0..1000 {
        world.manage(0.1, focus, &floor(), &SurfaceTable::default());
    }
    assert_eq!(world.cars.iter().filter(|car| car.is_patrol()).count(), 3);
    assert!(world.cars.iter().all(AiCar::is_patrol));
}

#[test]
fn radar_warns_about_a_patrol_before_the_chase_and_returns_during_search() {
    let mut world = TrafficWorld::new(cop_road());
    patrol(&mut world, &mut renderer(), Vec3::new(2.0, 0.0, 20.0));
    let forward = space::to_render(Vec3::Z.to_array());
    assert!(!world.cars[0].emergency_lights(), "patrol light bars stay off");
    let radar = world.radar(Vec3::ZERO, forward).unwrap();
    assert!(radar.distance > 19.0 && radar.distance < 22.0);
    assert!(radar.angle.abs() < 0.2);
    world.player_position = Some(Vec3::ZERO);
    world.start_pursuit(1, vec![wave()], player());
    assert!(world.radar(Vec3::ZERO, forward).is_none());
    see(&mut world, true);
    see(&mut world, false);
    assert!(world.radar(Vec3::ZERO, forward).is_none(), "a missed sight test is still within the chase grace");
    for _ in 0..32 {
        see(&mut world, false);
    }
    assert!(world.radar(Vec3::ZERO, forward).is_some());
    let above = space::to_render([0.0, 20.0, 0.0]);
    assert!(world.radar(above, forward).is_some(), "an elevated cop remains a target inside the native 3D cone");
}

#[test]
fn ending_the_chase_removes_static_roadblock_units_too() {
    let mut world = TrafficWorld::new(cop_road());
    patrol(&mut world, &mut renderer(), Vec3::new(2.0, 0.0, 20.0));
    world.cars[0].set_roadblock();
    world.start_pursuit(1, vec![wave()], player());
    world.stop_pursuit();
    assert!(world.cars.is_empty());
}

#[test]
fn a_wrecked_patrol_cannot_report_sight_or_appear_on_radar() {
    use crate::scenes::world::ai::collide::DriverClass;
    let mut world = TrafficWorld::new(cop_road());
    patrol(&mut world, &mut renderer(), Vec3::new(2.0, 0.0, 20.0));
    world.player_position = Some(Vec3::ZERO);
    world.update_sight(0.25, &|_, _| None);
    assert!(world.sighting(1).seen);
    let at = world.cars[0].physics_position();
    world.cars[0].on_hit(1_000_000.0, true, DriverClass::Human, 50.0, at, 0.0);
    assert!(world.cars[0].is_wrecked());
    world.update_sight(0.25, &|_, _| None);
    assert!(!world.sighting(1).seen);
    assert!(world.radar(Vec3::ZERO, Vec3::X).is_none());
}

#[test]
fn patrol_rear_sight_uses_fifty_metres_and_an_open_slope_remains_visible() {
    let mut world = TrafficWorld::new(cop_road());
    patrol(&mut world, &mut renderer(), Vec3::new(2.0, 0.0, 20.0));
    world.player_position = Some(Vec3::new(2.0, 0.0, -80.0));
    world.update_sight(0.25, &|_, _| None);
    assert!(!world.sighting(1).seen, "a patrol cannot see one hundred metres directly behind");
    world.player_position = Some(Vec3::new(2.0, 10.0, 50.0));
    world.update_sight(0.25, &|_, _| None);
    assert!(world.sighting(1).seen, "an unobstructed uphill target has no arbitrary height cutoff");
    world.update_sight(0.25, &|from, to| {
        Some(crate::scenes::world::drive::walls::Hit {
            point: from.lerp(to, 0.5),
            normal: Vec3::Y,
            prop: None,
            info: None,
        })
    });
    assert!(!world.sighting(1).seen, "the actual intervening deck or building still blocks sight");
}

#[test]
fn hiding_entry_keeps_watchers_visible_and_lost_sight_is_latched_per_unit() {
    let mut world = TrafficWorld::new(cop_road());
    patrol(&mut world, &mut renderer(), Vec3::new(2.0, 0.0, 20.0));
    patrol(&mut world, &mut renderer(), Vec3::new(-2.0, 0.0, 40.0));
    assert!(world.cars[0].report_sight(true, true), "entering under observation does not disappear");
    assert!(!world.cars[0].report_sight(true, false));
    assert!(!world.cars[0].report_sight(true, true), "this unit already lost the hidden target");
    assert!(world.cars[1].report_sight(true, true), "another watching unit retains its own contact");
    assert!(!world.cars[0].report_sight(false, false), "leaving the zone clears the latch");
    assert!(world.cars[0].report_sight(true, true));
}

#[test]
fn air_support_waits_for_a_model_instead_of_tracking_from_an_invisible_unit() {
    let mut world = TrafficWorld::new(cop_road());
    world.start_pursuit(4, vec![wave()], player());
    for _ in 0..48 {
        see(&mut world, true);
    }
    assert!(!world.pursuit_feed().unwrap().heli_present);
}

#[test]
fn roadblock_cars_keep_the_layout_offsets_instead_of_snapping_into_lanes() {
    let mut world = TrafficWorld::new(straight_road(1000.0));
    world.add_cop_model(cop_model(&mut renderer(), "copx", player()));
    world.start_pursuit(1, vec![wave()], player());
    let view = PlayerView {
        position: space::to_render([0.0, 0.0, -150.0]),
        forward: space::to_render(Vec3::Z.to_array()),
        speed: 20.0,
        nitrous: 0.0,
        throttle: 0.5,
    };
    let placed = world.place_roadblock(view, &floor(), &SurfaceTable::default()).unwrap();
    assert!(placed >= 2);
    let positions: Vec<_> = world.cars.iter().map(AiCar::physics_position).collect();
    assert!((positions[0].x - positions[1].x).abs() > 3.0, "lateral separation is preserved: {positions:?}");
    assert!(world.cars.iter().all(|car| car.is_live_police() && car.emergency_lights()));
}
