use blackbox_roads::{Light, Road, RoadNode, RoadProfile, RoadSegment, SplitMix, Zone};
use glam::Vec2;

use super::*;

fn straight_net() -> RoadNetwork {
    let lane = |offset| Zone { kind: zone::TRAFFIC, width: 4.0, offset };
    let node = |z: f32| RoadNode { position: Vec3::new(0.0, 0.0, z), profile: 0, segments: vec![0] };
    RoadNetwork {
        nodes: vec![node(0.0), node(400.0)],
        segments: vec![RoadSegment {
            nodes: [0, 1],
            length: 400.0,
            road: Some(0),
            flags: 0,
            start_handle: Vec3::ZERO,
            end_handle: Vec3::ZERO,
        }],
        profiles: vec![RoadProfile { middle: 1, zones: vec![lane(2.0), lane(2.0)] }],
        roads: vec![Road { scale: 1.0, speech_id: 0 }],
    }
}

fn car_at(z: f32, speed: f32) -> CarState {
    let body = Body {
        position: Vec3::new(2.0, 0.0, z),
        velocity: Vec3::new(0.0, 0.0, speed),
        forward: Vec2::Y,
        half_width: 0.9,
        half_length: 2.2,
    };
    CarState { body, forward_speed: speed, radius: 2.6, mass: 1680.0, tractor: false, stop: None }
}

fn lane_nav(net: &RoadNetwork, z_t: f32) -> RoadNav {
    let mut nav = RoadNav::traffic();
    nav.init_in_lane(net, 0, 1, z_t);
    nav.enable_trail(net);
    nav
}

#[test]
fn a_tractor_uses_the_longer_following_distance_without_changing_its_physical_mass() {
    use crate::scenes::world::drive::CarSim;
    use blackbox_vehicle::VehicleSpec;
    use nfsmw_data::car::physics::{CarBounds, CarPhysics, WallSpec};

    let net = straight_net();
    let spec = VehicleSpec { mass: 10_000.0, ..VehicleSpec::example() };
    let bounds = CarBounds { half_dimensions: spec.dimension, pivot: Vec3::ZERO };
    let mut sim = CarSim::new(
        CarPhysics {
            effect_source: 0,
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
    sim.configure_ai();
    let physical_mass = sim.body().mass();
    assert_eq!(physical_mass, 10_000.0);
    let street = CarState { mass: physical_mass, ..car_at(40.0, 10.0) };
    let tractor = CarState { tractor: true, ..street };
    assert_eq!(stopping_distance(street.forward_speed, following_mass(&street)), 32.375);
    assert_eq!(stopping_distance(tractor.forward_speed, following_mass(&tractor)), 50.0);
    for car in [street, tractor] {
        let mut nav = lane_nav(&net, 0.1);
        for _ in 0..3 {
            think(&net, &mut nav, &car, [], false, &mut SplitMix(1), &Cruise::default());
        }
        assert_eq!(car.mass, 10_000.0);
        assert_eq!(sim.body().mass(), physical_mass);
    }
}

#[test]
fn the_look_ahead_grows_with_speed() {
    let radius = 2.5;
    assert!((look_ahead(0.0, THINK_PERIOD, radius) - 12.5).abs() < 1e-4);
    // Fast: 30 m, plus a think of travel, plus the radius.
    assert!((look_ahead(25.0, THINK_PERIOD, radius) - (30.0 + 25.0 * THINK_PERIOD + radius)).abs() < 1e-3);
    assert!(look_ahead(12.5, THINK_PERIOD, radius) > look_ahead(5.0, THINK_PERIOD, radius));
}

#[test]
fn a_free_road_means_the_posted_speed_and_the_cursor() {
    let net = straight_net();
    let mut nav = lane_nav(&net, 0.1);
    let car = car_at(40.0, STREET_SPEED);
    let t = think(&net, &mut nav, &car, [], false, &mut SplitMix(1), &Cruise::default());
    assert_eq!(posted_speed(&net, &nav, &Cruise::default()), STREET_SPEED);
    assert_eq!(t.speed, STREET_SPEED);
    assert!(!t.blocked);
    // The cursor was brought up to the look-ahead distance and the car steers at it.
    let ahead = cursor_ahead(&nav, car.body.position);
    assert!(ahead >= look_ahead(STREET_SPEED, THINK_PERIOD, 2.6) - 0.5, "{ahead}");
    assert!(t.target.distance(nav.position) < 1e-3);
}

#[test]
fn shared_tick_delta_scales_acceleration_cap() {
    let net = straight_net();
    let car = car_at(40.0, 10.0);
    for dt in [THINK_PERIOD, THINK_PERIOD * 0.25, 10.0 / 144.0] {
        let mut nav = lane_nav(&net, 0.1);
        let think = think_with_delta(dt, &net, &mut nav, &car, [], false, &mut SplitMix(1), &Cruise::default());
        assert!((think.speed - (10.0 + ACCELERATION_CAP * dt)).abs() < 1e-5);
    }
}

#[test]
fn a_standing_car_may_only_ask_for_a_little_more() {
    let net = straight_net();
    let mut nav = lane_nav(&net, 0.1);
    let t = think(&net, &mut nav, &car_at(40.0, 10.0), [], false, &mut SplitMix(1), &Cruise::default());
    assert!((t.speed - (10.0 + ACCELERATION_CAP * THINK_PERIOD)).abs() < 1e-5, "{}", t.speed);
}

#[test]
fn a_stopped_car_on_a_free_road_pulls_away() {
    let net = straight_net();
    let mut nav = lane_nav(&net, 0.1);
    let t = think(&net, &mut nav, &car_at(40.0, 0.0), [], false, &mut SplitMix(1), &Cruise::default());
    assert!(t.speed >= CREEP_SPEED, "{}", t.speed);
}

#[test]
fn a_dead_end_means_stop() {
    let net = straight_net();
    let mut nav = lane_nav(&net, 0.1);
    nav.advance(&net, 1000.0, Vec3::ZERO, &mut SplitMix(1));
    assert!(nav.dead_end);
    let t = think(&net, &mut nav, &car_at(399.0, 10.0), [], false, &mut SplitMix(1), &Cruise::default());
    assert_eq!(t.speed, 0.0);
}

#[test]
fn a_stopped_car_ahead_slows_the_car_down() {
    let net = straight_net();
    let mut nav = lane_nav(&net, 0.1);
    let me = car_at(40.0, STREET_SPEED);
    let stopped = Body {
        position: Vec3::new(2.0, 0.0, 56.0),
        velocity: Vec3::ZERO,
        forward: Vec2::Y,
        half_width: 0.9,
        half_length: 2.2,
    };
    let mut speeds = Vec::new();
    for _ in 0..3 {
        let t = think(&net, &mut nav, &me, [stopped], false, &mut SplitMix(1), &Cruise::default());
        speeds.push((t.speed, t.blocked));
    }
    let (speed, blocked) = speeds[2];
    assert!(blocked, "{speeds:?}");
    assert!(speed < STREET_SPEED - 5.0, "{speeds:?}");
}

#[test]
fn a_patrol_cop_cruises_at_its_own_speeds_and_takes_no_acceleration_limit() {
    let net = straight_net();
    let mut nav = lane_nav(&net, 0.1);
    let cruise = Cruise::patrol(22.0, 32.0);
    assert_eq!(posted_speed(&net, &nav, &cruise), 22.0);
    let t = think(&net, &mut nav, &car_at(40.0, 10.0), [], false, &mut SplitMix(1), &cruise);
    assert_eq!(t.speed, 22.0, "no cap of speed plus 2 m/s per second");
    assert!(cruise.lateral_g > Cruise::default().lateral_g, "cops take bends harder");
}

/// Half the length of the car of `car_at`: the front is this far ahead of its position.
const HALF_LENGTH: f32 = 2.2;

fn at_light(z: f32, speed: f32, line_z: f32, light: Light) -> CarState {
    let stop = StopLine { distance: line_z - (z + HALF_LENGTH), light };
    CarState { stop: Some(stop), ..car_at(z, speed) }
}

/// Drives a car that does exactly what each think asks, towards a light whose stop line is at `line_z`;
/// returns the speeds and the position of the car's front at the end.
fn approach(light: Light, start_z: f32, line_z: f32, thinks: usize) -> (Vec<f32>, f32) {
    let net = straight_net();
    let mut nav = lane_nav(&net, 0.1);
    let (mut z, mut speed) = (start_z, STREET_SPEED);
    let mut speeds = Vec::new();
    for _ in 0..thinks {
        let car = at_light(z, speed, line_z, light);
        speed = think(&net, &mut nav, &car, [], false, &mut SplitMix(1), &Cruise::default()).speed;
        z += speed * THINK_PERIOD;
        speeds.push(speed);
    }
    (speeds, z + HALF_LENGTH)
}

#[test]
fn a_red_light_slows_the_car_to_a_stop_before_the_line() {
    let line = 60.0;
    let (speeds, front) = approach(Light::Red, 40.0, line, 120);
    assert!(speeds[0] < STREET_SPEED, "{speeds:?}");
    assert!(speeds.windows(2).all(|w| w[1] <= w[0] + 1e-4), "never speeds up: {speeds:?}");
    assert!(speeds.last().is_some_and(|&s| s < 0.5), "stopped: {:?}", speeds.last());
    assert!(front <= line + 0.3, "front {front} past the line {line}");
    assert!(front > line - 3.0, "front {front} stops short of the line {line}");
}

#[test]
fn a_green_light_leaves_the_posted_speed() {
    let net = straight_net();
    let mut nav = lane_nav(&net, 0.1);
    let car = at_light(40.0, STREET_SPEED, 60.0, Light::Green);
    let t = think(&net, &mut nav, &car, [], false, &mut SplitMix(1), &Cruise::default());
    assert_eq!(t.speed, STREET_SPEED);
    assert!(!t.blocked, "a light is not a car in the way");
}

#[test]
fn a_car_inside_the_junction_has_no_stop_line() {
    // Past the line the lookup gives nothing and the car carries on at the posted speed.
    let net = straight_net();
    let mut nav = lane_nav(&net, 0.1);
    let t = think(&net, &mut nav, &car_at(70.0, STREET_SPEED), [], false, &mut SplitMix(1), &Cruise::default());
    assert_eq!(t.speed, STREET_SPEED);
}

#[test]
fn a_car_waiting_at_a_red_light_stays_on_the_brake_and_pulls_away_on_green() {
    let net = straight_net();
    let mut nav = lane_nav(&net, 0.1);
    let line = 60.0 + HALF_LENGTH;
    let red =
        think(&net, &mut nav, &at_light(60.0, 0.0, line, Light::Red), [], false, &mut SplitMix(1), &Cruise::default());
    assert!(red.speed < 0.5, "{}", red.speed);
    let green = think(
        &net,
        &mut nav,
        &at_light(60.0, 0.0, line, Light::Green),
        [],
        false,
        &mut SplitMix(1),
        &Cruise::default(),
    );
    assert!(green.speed >= CREEP_SPEED, "{}", green.speed);
}

#[test]
fn a_car_far_from_the_line_still_creeps_towards_it_on_red() {
    let net = straight_net();
    let mut nav = lane_nav(&net, 0.1);
    let red =
        think(&net, &mut nav, &at_light(40.0, 0.0, 70.0, Light::Red), [], false, &mut SplitMix(1), &Cruise::default());
    assert!(red.speed >= CREEP_SPEED, "{}", red.speed);
}

#[test]
fn an_amber_the_car_cannot_stop_for_is_driven_through() {
    let net = straight_net();
    let mut nav = lane_nav(&net, 0.1);
    let close = at_light(40.0, STREET_SPEED, 40.0 + HALF_LENGTH + 4.0, Light::Amber);
    let t = think(&net, &mut nav, &close, [], false, &mut SplitMix(1), &Cruise::default());
    assert_eq!(t.speed, STREET_SPEED);
    let far = at_light(40.0, STREET_SPEED, 40.0 + HALF_LENGTH + 20.0, Light::Amber);
    let t = think(&net, &mut nav, &far, [], false, &mut SplitMix(1), &Cruise::default());
    assert!(t.speed < STREET_SPEED, "{}", t.speed);
}

#[test]
fn a_patrol_cop_stops_at_red_as_well() {
    let net = straight_net();
    let mut nav = lane_nav(&net, 0.1);
    let car = at_light(40.0, 10.0, 40.0 + HALF_LENGTH + 5.0, Light::Red);
    let t = think(&net, &mut nav, &car, [], false, &mut SplitMix(1), &Cruise::patrol(22.0, 32.0));
    assert!(t.speed < 10.0, "{}", t.speed);
}
