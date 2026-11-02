//! The reinforcement timer and the wave size (spec `ai-pursuit-cops.md` section 3): a new chase waits the opening
//! interval before its first reinforcement, the wave asks for its nominal count with no active-car cap, and a cop that is
//! due but not placed holds traffic back (`ai-traffic-spawning.md` section 4).

use super::free_roam::renderer;
use super::*;
use crate::settings::bartender::BartenderSettings;

fn row(count: u32) -> HeatRow {
    let mut row = wave();
    row.cops[0].count = count;
    row.full_engagement_cops = 20;
    row.time_between_first_four_spawns = 10.0;
    row.time_between_cop_spawns = 2.0;
    row
}

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
    world.start_pursuit(5, vec![row(4); 5], player());
    (world, focus)
}

#[test]
fn a_new_chase_waits_the_opening_interval_before_its_first_reinforcement() {
    let (mut world, focus) = setup();
    let (collision, surfaces) = (floor(), SurfaceTable::default());
    for _ in 0..38 {
        world.update_pursuit(0.25, focus, &collision, &surfaces);
    }
    assert!(world.cars.is_empty(), "no reinforcement within the 10 s opening interval");
    assert!((0..40).any(|_| {
        world.update_pursuit(0.25, focus, &collision, &surfaces);
        !world.cars.is_empty()
    }));
}

#[test]
fn the_wave_asks_for_its_nominal_count_above_the_active_car_limit() {
    let pursuit = Pursuit::new(1, vec![row(10)], player());
    let settings = BartenderSettings::default();
    assert_eq!(settings.max_cops, 8);
    assert_eq!(pursuit.wanted(&settings, false), 10, "the active-car limit gates each request, not the wave");
}

#[test]
fn a_cop_due_but_not_placed_holds_traffic_back() {
    let (mut world, focus) = setup();
    let (collision, surfaces) = (floor(), SurfaceTable::default());
    assert!(world.density() > 0.0, "a chase with nothing due keeps three quarters of the traffic");
    world.cop_models.clear();
    world.pursuit.as_mut().unwrap().spawn_timer = -1.0;
    world.update_pursuit(0.25, focus, &collision, &surfaces);
    assert!(world.cars.is_empty());
    assert!(world.cop_request_pending(), "the wave still needs a cop it could not place");
    assert_eq!(world.density(), 0.0);
}
