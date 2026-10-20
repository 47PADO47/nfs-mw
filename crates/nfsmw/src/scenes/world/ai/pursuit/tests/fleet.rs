//! The cop manager's remaining spawn rules: the window measured from the camera, the lane spacing, the walk from the
//! seek-ahead point, the active-car budget with its queued requests and priority removal, and each cop's own sight
//! (`docs/specs/ai-pursuit-cops.md` sections 1, 3 and 5, `docs/provenance/ai-pursuit-cop-fleet.md`).

use super::free_roam::{patrol, renderer};
use super::*;
use crate::scenes::world::ai::PatrolPlan;
use crate::scenes::world::ai::cop_manager::requests::Role as RequestRole;
use crate::scenes::world::ai::spawn::cop_lane_clear;

/// A 2 km road along z, a patrol plan and a cop model `copx`, the player 200 m down it so the floor covers the window.
fn world() -> TrafficWorld {
    let mut world = TrafficWorld::new(straight_road(1000.0));
    world.add_cop_model(cop_model(&mut renderer(), "copx", player()));
    let mut row = wave();
    row.patrol_cars = 2;
    let mut plan = PatrolPlan::new(vec!["copx".into()], 50.0, 71.0);
    plan.levels = vec![row];
    world.set_patrol(plan);
    world.configure(0, 5);
    world
}

fn player_at(z: f32, speed: f32) -> Focus {
    let body = Body {
        position: Vec3::new(0.0, 0.0, z),
        velocity: Vec3::new(0.0, 0.0, speed),
        forward: Vec2::Y,
        half_width: 1.0,
        half_length: 2.0,
    };
    Focus { position: [z, 0.0], heading: 0.0, speed, body: Some(body) }
}

fn horizontal(a: Vec3, b: Vec3) -> f32 {
    (a.x - b.x).hypot(a.z - b.z)
}

fn patrols(world: &mut TrafficWorld, zs: impl IntoIterator<Item = f32>) {
    for z in zs {
        patrol(world, &mut renderer(), Vec3::new(2.0, 0.0, z));
    }
}

#[test]
fn the_chase_spawn_points_are_a_walk_from_the_seek_ahead_point_and_a_disc_ahead() {
    let mut world = world();
    let focus = player_at(-200.0, 30.0);
    let body = focus.body.unwrap();
    let points = world.chase_spawn_points(&body);
    assert_eq!(points.len(), 2, "a walk and the fallback");
    // On a straight road the vector back to the player turned by 60 degrees still points back along the road, so the
    // walk ends at the player: the window rejects it and the fallback is what places the cop.
    assert!(horizontal(points[0], body.position) < 30.0, "{:?}", points[0]);
    assert!(horizontal(points[1], Vec3::new(0.0, 0.0, 0.0)) <= 190.0 + 1e-3, "the disc around the point 200 m ahead");
}

#[test]
fn a_chase_cop_lands_in_the_window_through_the_fallback_when_the_walk_fails() {
    let mut world = world();
    let focus = player_at(-200.0, 30.0);
    world.player_position = Some(focus.body.unwrap().position);
    world.start_pursuit(1, vec![wave()], player());
    let (collision, surfaces) = (floor(), SurfaceTable::default());
    let placed =
        (0..60).any(|_| world.spawn_cop("copx", focus, focus.body.unwrap().position, player(), &collision, &surfaces));
    assert!(placed, "the fallback finds a spot in the window within a few tries");
    let cop = world.cars.iter().find(|car| car.is_cop()).expect("a cop stands");
    let gap = horizontal(cop.physics_position(), focus.body.unwrap().position);
    assert!((150.0..=400.0).contains(&gap), "{gap}");
}

#[test]
fn the_spawn_window_is_measured_from_the_camera_not_the_player() {
    let mut world = world();
    let (collision, surfaces) = (floor(), SurfaceTable::default());
    let target = Vec3::new(0.0, 0.0, 0.0);
    let spot = CopSpot {
        at: Vec3::new(0.0, 0.0, 100.0),
        target,
        speed: START_SPEED,
        player: player(),
        band: Some((150.0, 400.0)),
    };
    assert!(
        !world.spawn_cop_at("copx", spot, &collision, &surfaces),
        "100 m from the player and no camera: inside the window"
    );
    world.cops.set_eye(Vec3::new(0.0, 5.0, -100.0));
    assert!(world.spawn_cop_at("copx", spot, &collision, &surfaces), "200 m from a camera behind the player");
}

#[test]
fn a_patrol_keeps_the_lane_spacing_from_the_cars_on_the_road() {
    let mut world = world();
    patrols(&mut world, [100.0]);
    let candidate = |x: f32, z: f32, toward: Vec3| {
        let mut nav = RoadNav::new(NavKind::Direction, LaneType::Racing, SegmentFilter::default());
        assert!(nav.init_at_point(&world.network, &world.index, Vec3::new(x, 0.0, z), toward, false));
        nav
    };
    let same_lane = candidate(2.0, 108.0, Vec3::Z);
    assert!(!cop_lane_clear(&world.cars, &same_lane), "8 m ahead in the patrol's own lane: 15 m are needed");
    let far_same_lane = candidate(2.0, 120.0, Vec3::Z);
    assert!(cop_lane_clear(&world.cars, &far_same_lane), "20 m ahead is clear");
    let other_lane = candidate(-2.0, 108.0, -Vec3::Z);
    assert!(cop_lane_clear(&world.cars, &other_lane), "9 m away in the oncoming lane: 5 m are needed");
    let close_other_lane = candidate(-2.0, 102.0, -Vec3::Z);
    assert!(!cop_lane_clear(&world.cars, &close_other_lane), "4.5 m away in the oncoming lane");
}

#[test]
fn a_wave_cop_is_not_asked_for_while_the_active_cars_fill_the_budget() {
    let mut world = world();
    let focus = player_at(-200.0, 0.0);
    let (collision, surfaces) = (floor(), SurfaceTable::default());
    // Eight patrols 400 m and more from the player: out of sight range, so they stay patrols and fill the budget.
    patrols(&mut world, (0..8).map(|i| 190.0 - 20.0 * i as f32));
    world.player_position = Some(focus.body.unwrap().position);
    let mut row = wave();
    row.cops[0].count = 6;
    row.full_engagement_cops = 6;
    world.start_pursuit(1, vec![row], player());
    assert_eq!(world.cop_budget(), 8);
    assert_eq!(world.active_cop_cars(), 8, "the patrols stayed patrols");
    assert!(!world.budget_has_room());
    for _ in 0..120 {
        world.update_pursuit(FRAME, focus, &collision, &surfaces);
    }
    assert_eq!(world.active_cop_cars(), 8, "no cop is added past the budget");
    world.cars.pop();
    assert!(world.budget_has_room());
    for _ in 0..600 {
        world.update_pursuit(FRAME, focus, &collision, &surfaces);
    }
    assert!(world.cars.iter().any(AiCar::is_cop), "with room in the budget the wave spawns");
}

fn queue_patrol(world: &mut TrafficWorld, z: f32) {
    world.queue_cop_spawn("copx", Vec3::new(2.0, 0.0, z), Vec3::Z, RequestRole::Patrol);
}

#[test]
fn a_spawn_request_waits_out_the_lockout_and_is_placed_where_it_was_asked() {
    let mut world = world();
    let focus = player_at(0.0, 0.0);
    let (collision, surfaces) = (floor(), SurfaceTable::default());
    world.cops.lock_out();
    queue_patrol(&mut world, 120.0);
    world.service_spawn_requests(focus, &collision, &surfaces);
    assert!(world.cars.is_empty(), "no cop is placed during the lockout");
    assert_eq!(world.cop_requests_waiting(), 1);
    world.cops.unlock();
    world.service_spawn_requests(focus, &collision, &surfaces);
    assert_eq!(world.cop_requests_waiting(), 0);
    let car = &world.cars[0];
    assert!(car.is_patrol());
    assert!(horizontal(car.physics_position(), Vec3::new(2.0, 0.0, 120.0)) < 0.5);
}

#[test]
fn a_spawn_request_whose_spot_overlaps_a_car_stays_queued() {
    let mut world = world();
    let focus = player_at(0.0, 0.0);
    let (collision, surfaces) = (floor(), SurfaceTable::default());
    patrols(&mut world, [120.0]);
    queue_patrol(&mut world, 121.0);
    world.service_spawn_requests(focus, &collision, &surfaces);
    assert_eq!(world.cars.len(), 1);
    assert_eq!(world.cop_requests_waiting(), 1);
}

#[test]
fn no_new_patrol_is_asked_for_while_a_spawn_request_waits() {
    let mut world = world();
    let focus = player_at(-200.0, 0.0);
    let (collision, surfaces) = (floor(), SurfaceTable::default());
    world.queue_cop_spawn("nobody", Vec3::ZERO, Vec3::Z, RequestRole::Patrol);
    for _ in 0..600 {
        world.manage(0.1, focus, &collision, &surfaces);
    }
    assert!(world.cars.is_empty(), "a request waits, so the patrol pass wants none");
    world.cops.requests = Default::default();
    for _ in 0..1000 {
        world.manage(0.1, focus, &collision, &surfaces);
    }
    assert!(world.cars.iter().any(AiCar::is_patrol), "with the queue empty the patrols come");
}

#[test]
fn a_request_over_the_budget_takes_the_cop_behind_the_player_off_the_road_first() {
    let mut world = world();
    let focus = player_at(0.0, 0.0);
    let (collision, surfaces) = (floor(), SurfaceTable::default());
    // Seven cops ahead of the player and in view, and one behind it, out of view.
    patrols(&mut world, [-100.0, 40.0, 60.0, 80.0, 100.0, 120.0, 140.0, 160.0]);
    for car in &mut world.cars {
        car.note_view(car.physics_position().z > 0.0, 1.0);
    }
    assert_eq!(world.active_cop_cars(), 8);
    queue_patrol(&mut world, 180.0);
    world.service_spawn_requests(focus, &collision, &surfaces);
    assert!(world.cars.iter().all(|car| car.physics_position().z > 0.0), "the cop behind the player went");
    assert_eq!(world.active_cop_cars(), 8, "and the requested one stands in its place");
    assert_eq!(world.cop_requests_waiting(), 0);
}

#[test]
fn a_request_under_the_budget_removes_nobody() {
    let mut world = world();
    let focus = player_at(0.0, 0.0);
    let (collision, surfaces) = (floor(), SurfaceTable::default());
    patrols(&mut world, [-100.0, 40.0]);
    queue_patrol(&mut world, 180.0);
    world.service_spawn_requests(focus, &collision, &surfaces);
    assert_eq!(world.active_cop_cars(), 3);
}

#[test]
fn the_cops_sent_away_are_the_ones_that_cannot_see_the_player() {
    let mut world = TrafficWorld::new(cop_road());
    for i in 0..4 {
        patrol(&mut world, &mut renderer(), Vec3::new(2.0, 0.0, 10.0 + 10.0 * i as f32));
    }
    let mut row = wave();
    row.cops[0].count = 4;
    row.full_engagement_cops = 5;
    world.player_position = Some(Vec3::ZERO);
    world.start_pursuit(1, vec![row], player());
    world.pursuit.as_mut().unwrap().budget.note_evaded(3);
    // The two farthest cops see the player; the two nearer ones do not.
    world.cars[2].report_sight(false, true);
    world.cars[3].report_sight(false, true);
    world.flee_surplus(focus(), false);
    let fleeing: Vec<bool> = world.cars.iter().map(AiCar::is_fleeing).collect();
    assert_eq!(fleeing, [true, true, false, false], "a cop that sees the player stays unless more than two do");
}
