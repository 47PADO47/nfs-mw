//! Parked police vehicles keep their slots of the shared vehicle budget (spec `ai-traffic-spawning.md` section 5): a
//! chase remembers the most police cars it has had on the road at once, and those that have left still count against
//! traffic until the chase ends.

use super::free_roam::renderer;
use super::*;

fn setup() -> (TrafficWorld, Focus) {
    let mut world = TrafficWorld::new(straight_road(1000.0));
    world.add_cop_model(cop_model(&mut renderer(), "copx", player()));
    world.configure(20, 5);
    let body = Body {
        position: Vec3::new(0.0, 0.0, -200.0),
        velocity: Vec3::Z * 20.0,
        forward: Vec2::Y,
        half_width: 1.0,
        half_length: 2.0,
    };
    let focus = Focus { position: [-200.0, 0.0], heading: 0.0, speed: 20.0, body: Some(body) };
    world.player_position = Some(body.position);
    world.police.set_forced_pursuit(true);
    world.start_pursuit(5, vec![wave(); 5], player());
    (world, focus)
}

fn place_three(world: &mut TrafficWorld) {
    let (collision, surfaces) = (floor(), SurfaceTable::default());
    for z in [-100.0, 0.0, 100.0] {
        let spot = CopSpot { at: Vec3::new(2.0, 0.0, z), target: Vec3::ZERO, speed: 0.0, player: player(), band: None };
        assert!(world.spawn_cop_at("copx", spot, &collision, &surfaces));
    }
    world.note_police_fleet();
}

#[test]
fn cops_that_leave_the_road_keep_their_budget_slots() {
    let (mut world, focus) = setup();
    place_three(&mut world);
    assert_eq!(world.population_slots(focus), 4, "the player and three cops");
    world.cars.clear();
    assert_eq!(world.population_slots(focus), 4, "the three cops are parked and still count");
    assert_eq!(world.other_slots(focus), 4, "the share budget counts them too");
    assert!(world.vehicle_budget_has_room(focus), "the next cop takes a parked slot");
}

#[test]
fn the_parked_slots_go_free_when_the_chase_ends() {
    let (mut world, focus) = setup();
    place_three(&mut world);
    world.cars.clear();
    world.pursuit = None;
    assert_eq!(world.population_slots(focus), 1, "only the player is left");
}
