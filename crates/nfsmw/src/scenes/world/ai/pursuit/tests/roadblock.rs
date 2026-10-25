//! The roadblocks in a chase: built whole, kept while their cars stand, dodged, recruited from, released and asked for
//! again. Spec: `docs/specs/ai-pursuit-roadblocks.md`. The road is 2 km long along z with a 4 m lane either side (a 9 m
//! road with the metre the rule adds), and the player drives along it from z = -150, so the site is about z = 100.

use blackbox_collision::CollisionWorld;
use blackbox_pursuit::EndReason;

use super::free_roam::see;
use super::*;
use crate::scenes::world::ai::PlayerView;
use crate::scenes::world::ai::roadblock::Cause;

fn view_at(z: f32) -> PlayerView {
    PlayerView {
        nos_input: false,
        nitrous_boost: 0.0,
        velocity: (space::to_render(Vec3::Z.to_array())) * (20.0),
        position: space::to_render([0.0, 0.0, z]),
        forward: space::to_render(Vec3::Z.to_array()),
        speed: 20.0,
        nitrous: 0.0,
        throttle: 0.5,
    }
}

fn chase(heat: u32) -> TrafficWorld {
    let mut world = TrafficWorld::new(straight_road(1000.0));
    let mut renderer = NullBackend::new(Capabilities::baseline("null", GraphicsApi::Vulkan));
    world.add_cop_model(cop_model(&mut renderer, "copx", player()));
    world.police.set_forced_pursuit(true);
    world.start_pursuit(heat, vec![wave()], player());
    assert_eq!(world.police.level(), Some(heat));
    world
}

fn build(world: &mut TrafficWorld, cause: Cause) -> Result<usize, String> {
    world.build_roadblock(cause, view_at(-150.0), &floor(), None, &SurfaceTable::default())
}

fn run(world: &mut TrafficWorld, dt: f32, z: f32) {
    world.update_roadblocks(dt, view_at(z), &floor(), None, &SurfaceTable::default());
}

fn group_cars(world: &TrafficWorld) -> usize {
    world.cars.iter().filter(|car| car.roadblock_group().is_some()).count()
}

#[test]
fn a_roadblock_is_built_whole_with_its_cars_in_one_group_and_its_strips() {
    let mut world = chase(9);
    let placed = build(&mut world, Cause::Console).unwrap();
    assert!(placed >= 1);
    assert_eq!(world.roadblocks.count(), 1);
    assert_eq!(group_cars(&world), placed, "every car carries the roadblock's group");
    let strips = world.roadblocks.strips();
    assert!(!strips.is_empty(), "heat 9 takes the spike layouts every time");
    assert!(strips.iter().all(|strip| strip.half[1] > 3.0), "the strip has the installed prop's 6.5 m width");
    let stats = world.roadblocks.stats();
    assert_eq!((stats.deployed, stats.dodged), (1, 0));
    assert!(stats.props_deployed >= strips.len() as u32);
    assert!(world.police.cost_to_state() >= 500 + 250 * stats.props_deployed, "the roadblock and each prop cost");
}

#[test]
fn a_second_roadblock_cannot_stand_on_the_first() {
    let mut world = chase(5);
    build(&mut world, Cause::Console).unwrap();
    let again = build(&mut world, Cause::Console);
    assert!(again.is_err_and(|e| e.contains("another vehicle")), "occupied sites are refused");
    assert_eq!(world.roadblocks.count(), 1);
}

#[test]
fn a_site_where_no_car_can_stand_builds_nothing_not_a_part() {
    let mut world = chase(5);
    let nothing = CollisionWorld::new(None);
    let built = world.build_roadblock(Cause::Console, view_at(-150.0), &nothing, None, &SurfaceTable::default());
    assert!(built.is_err());
    assert_eq!(group_cars(&world), 0, "no partial layout is left behind");
    assert_eq!(world.roadblocks.count(), 0);
}

#[test]
fn passing_the_roadblock_dodges_it_once() {
    let mut world = chase(9);
    build(&mut world, Cause::Console).unwrap();
    run(&mut world, 0.25, -150.0);
    assert_eq!(world.roadblocks.stats().dodged, 0);
    run(&mut world, 0.25, 160.0);
    run(&mut world, 0.25, 200.0);
    let stats = world.roadblocks.stats();
    assert_eq!(stats.dodged, 1);
    assert!(stats.spike_strips_dodged >= 1, "the strips of a dodged roadblock count as dodged");
    assert!(!world.roadblocks.forces_follow(), "a passed roadblock no longer holds the follow formation");
}

#[test]
fn a_whole_roadblock_holds_the_follow_formation() {
    let mut world = chase(5);
    build(&mut world, Cause::Console).unwrap();
    run(&mut world, 0.25, -150.0);
    assert!(world.roadblocks.forces_follow());
}

#[test]
fn a_roadblock_with_no_car_left_is_released_and_the_next_one_may_stand() {
    let mut world = chase(9);
    build(&mut world, Cause::Console).unwrap();
    world.cars.retain(|car| car.roadblock_group().is_none());
    run(&mut world, 0.6, -150.0);
    assert_eq!(world.roadblocks.count(), 0, "no cars, no roadblock");
    assert!(world.roadblocks.strips().is_empty(), "its strips go with it");
    build(&mut world, Cause::Console).expect("the pursuit has no roadblock now, so it may have another");
    assert_eq!(world.roadblocks.stats().deployed, 2);
}

#[test]
fn the_lottery_builds_a_roadblock_after_the_minimum_delay_and_two_rolls() {
    let mut world = chase(9);
    for _ in 0..50 {
        see(&mut world, true);
    }
    assert!(world.police.pursuit_secs() > 10.0, "the minimum support delay of heat 9 has passed");
    let mut seconds = 0.0;
    while world.roadblocks.count() == 0 && seconds < 40.0 {
        run(&mut world, 0.5, -150.0);
        seconds += 0.5;
    }
    assert_eq!(world.roadblocks.count(), 1, "built within two rolls (8 to 12 s each) at 100 percent");
    assert!(seconds >= 8.0, "not before the first wait: {seconds}");
    assert!(!world.roadblocks.dispatcher_latched(), "the latch is met");
}

#[test]
fn nothing_is_asked_for_before_the_minimum_delay() {
    let mut world = chase(9);
    for _ in 0..80 {
        run(&mut world, 0.5, -150.0);
    }
    assert_eq!(world.roadblocks.count(), 0, "the pursuit has not lasted the minimum delay");
}

#[test]
fn an_accepted_ram_request_delays_the_ordinary_roadblock_lottery() {
    let mut world = chase(9);
    for _ in 0..50 {
        see(&mut world, true);
    }
    assert!(world.ask_support(support(Strategy::Ram)));
    for _ in 0..60 {
        run(&mut world, 0.25, -150.0);
        assert_eq!(world.roadblocks.count(), 0, "ram selection must postpone the ordinary lottery for 15 s");
        assert!(!world.roadblocks.dispatcher_latched());
        assert!(!world.ask_support(support(Strategy::Ram)), "an existing order refuses another request");
    }
    for _ in 0..60 {
        run(&mut world, 0.5, -150.0);
        if world.roadblocks.count() > 0 {
            break;
        }
    }
    assert_eq!(world.roadblocks.count(), 1, "rejected requests must not refresh the delay indefinitely");
}

#[test]
fn a_target_that_stays_near_the_roadblock_makes_it_recruit_one_car() {
    let mut world = chase(5);
    build(&mut world, Cause::Console).unwrap();
    let cars = group_cars(&world);
    for _ in 0..30 {
        run(&mut world, 1.0, -150.0);
    }
    assert_eq!(group_cars(&world), cars - 1, "the roadblock gave one car to the chase");
    assert_eq!(world.roadblocks.stats().recruited, 1);
    assert_eq!(cop_count(&world), 1, "and it is an ordinary pursuit cop now");
}

#[test]
fn a_roadblock_within_sixty_metres_sees_the_player_and_a_hiding_spot_does_not_hide_it() {
    let mut world = chase(5);
    build(&mut world, Cause::Console).unwrap();
    run(&mut world, 0.25, -150.0);
    assert!(!world.roadblocks.sees_player(), "250 m away is out of its sight");
    run(&mut world, 0.25, 60.0);
    assert!(world.roadblocks.sees_player(), "within 60 m it sees the player");
}

#[test]
fn heavy_support_builds_its_roadblock_of_suvs() {
    let mut world = chase(5);
    let mut renderer = NullBackend::new(Capabilities::baseline("null", GraphicsApi::Vulkan));
    world.add_cop_model(cop_model(&mut renderer, "copsuv", player()));
    let placed = build(&mut world, Cause::Heavy { big_suv: 100.0 }).unwrap();
    assert!(placed >= 1);
    assert!(world.cars.iter().filter(|c| c.roadblock_group().is_some()).all(|c| c.name.eq_ignore_ascii_case("copsuv")));
}

#[test]
fn an_escape_leaves_the_roadblocks_standing_and_ending_by_hand_takes_them_away() {
    let mut world = chase(9);
    build(&mut world, Cause::Console).unwrap();
    let cars = group_cars(&world);
    world.end_pursuit(EndReason::Evaded);
    assert_eq!(group_cars(&world), cars, "the cars stay until they are retired like unseen traffic");
    assert!(!world.roadblocks.strips().is_empty(), "the strips are still on the road");
    world.stop_pursuit();
    assert_eq!(group_cars(&world), 0);
    assert!(world.roadblocks.strips().is_empty());
}

#[test]
fn roadblock_cars_do_not_look_for_the_player_themselves() {
    let mut world = chase(9);
    build(&mut world, Cause::Console).unwrap();
    world.player_position = Some(Vec3::new(0.0, 0.0, 99.0));
    world.update_sight(0.25, &|_, _| None);
    assert!(!world.sighting(9).seen, "a roadblock car is not a sighting cop; the roadblock's own rule sees");
}

/// A chase with a roaming patrol 330 m down the road from the player, unseen for `unseen` seconds.
fn chase_with_a_far_patrol(unseen: f32) -> TrafficWorld {
    let mut world = chase(5);
    let mut renderer = NullBackend::new(Capabilities::baseline("null", GraphicsApi::Vulkan));
    super::free_roam::patrol(&mut world, &mut renderer, Vec3::new(2.0, 0.0, 180.0));
    for _ in 0..(unseen * 60.0) as usize {
        world.cars[0].note_views(1.0 / 60.0, |_, _| false);
    }
    world
}

#[test]
fn a_lottery_roadblock_takes_a_cop_that_has_been_out_of_view_and_far_away() {
    let mut world = chase_with_a_far_patrol(6.0);
    let patrol_key = world.cars[0].key();
    let before = world.cars.len();
    let placed = build(&mut world, Cause::Lottery).unwrap();
    assert!(!world.cars.iter().any(|car| car.key() == patrol_key), "the patrol was taken for the roadblock");
    assert_eq!(world.cars.len(), before - 1 + placed, "one of the roadblock's cars is the stolen one");
}

#[test]
fn a_cop_seen_a_moment_ago_or_a_console_roadblock_takes_nothing() {
    let mut world = chase_with_a_far_patrol(2.0);
    let before = world.cars.len();
    let placed = build(&mut world, Cause::Lottery).unwrap();
    assert_eq!(world.cars.len(), before + placed, "a cop out of view for 2 s stays");
    let mut world = chase_with_a_far_patrol(6.0);
    let before = world.cars.len();
    let placed = build(&mut world, Cause::Console).unwrap();
    assert_eq!(world.cars.len(), before + placed, "the console builds new cars");
}
