//! What a traffic car decides every think: how far ahead to keep its cursor and how fast to go.
//! Spec: `docs/specs/ai-traffic.md` (§3).

use blackbox_driver::ramp;
use blackbox_roads::{
    Body, Occlusion, RandomSource, RoadNav, RoadNetwork, trail_curvature, update_occluded_position, zone,
};
use glam::Vec3;

use super::stop::StopLine;

/// The traffic AI thinks at 10 Hz: every this many physics steps.
pub const THINK_STEPS: u32 = 10;
/// Seconds between thinks.
pub const THINK_PERIOD: f32 = THINK_STEPS as f32 / 60.0;

/// Posted speeds in metres per second: 35 mph on streets, 55 mph where there are four or more traffic
/// lanes.
pub const STREET_SPEED: f32 = 35.0 * MPH;
pub const HIGHWAY_SPEED: f32 = 55.0 * MPH;
/// Metres per second in one mile per hour.
pub const MPH: f32 = 0.447_04;
/// Traffic lane zones (both directions) that make a road a highway.
const HIGHWAY_LANES: usize = 4;
/// Lateral acceleration (in g) traffic and cops take bends at.
const TRAFFIC_LATERAL_G: f32 = 0.6;
const COP_LATERAL_G: f32 = 1.6;
const GRAVITY: f32 = 9.8;
/// The speed a car on a free road may always ask for, so that a stopped car can pull away again. The
/// original has no such floor and a car that has come to a halt stays on the brake.
const CREEP_SPEED: f32 = 1.0;
/// A traffic car never asks for more than this much above its current speed per second.
const ACCELERATION_CAP: f32 = 2.0;

/// The distance ahead of the car the cursor is kept at: 10 m when slow, 30 m when fast, plus a think
/// of travel and the car's radius.
pub fn look_ahead(speed: f32, think: f32, radius: f32) -> f32 {
    let base = 10.0 + 20.0 * ramp(speed, 0.0, 25.0);
    base + speed * think + radius
}

/// How a car cruises: its speed on streets and on roads with four or more traffic lanes, and how hard it
/// takes bends. Traffic gets the numbers of the pattern it spawned in, patrol cops the search-mode speeds.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Cruise {
    pub street: f32,
    pub highway: f32,
    pub lateral_g: f32,
    /// A cop may accelerate as hard as it likes; traffic never asks for much more than it has.
    pub limit_acceleration: bool,
}

impl Cruise {
    pub fn traffic(street: f32, highway: f32) -> Self {
        Self { street, highway, lateral_g: TRAFFIC_LATERAL_G, limit_acceleration: true }
    }

    pub fn patrol(street: f32, highway: f32) -> Self {
        Self { street, highway, lateral_g: COP_LATERAL_G, limit_acceleration: false }
    }
}

impl Default for Cruise {
    fn default() -> Self {
        Self::traffic(STREET_SPEED, HIGHWAY_SPEED)
    }
}

/// The speed limit posted on the segment the cursor is on.
pub fn posted_speed(net: &RoadNetwork, nav: &RoadNav, cruise: &Cruise) -> f32 {
    let seg = net.segment(nav.segment);
    let profile = net.profile_at(nav.segment, seg.nodes[0]);
    let lanes = profile.zones.iter().filter(|z| z.kind == zone::TRAFFIC).count();
    match lanes >= HIGHWAY_LANES {
        true => cruise.highway,
        false => cruise.street,
    }
}

/// Signed distance of the cursor ahead of the car along the cursor's forward vector.
pub fn cursor_ahead(nav: &RoadNav, car: Vec3) -> f32 {
    nav.forward.dot(nav.position - car)
}

/// What a traffic car knows of itself when it thinks.
#[derive(Debug, Clone, Copy)]
pub struct CarState {
    pub body: Body,
    /// Signed speed along the car, m/s.
    pub forward_speed: f32,
    /// Bounding radius, metres.
    pub radius: f32,
    pub mass: f32,
    /// The next stop line of a signalled junction ahead, when lights are on and the car obeys them.
    pub stop: Option<StopLine>,
}

/// The result of a think.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Think {
    /// The point to steer at (physics space).
    pub target: Vec3,
    pub speed: f32,
    /// Another car blocks the view along the trail.
    pub blocked: bool,
}

/// The stopping distance behind a car ahead: 3 m when slow up to 50 m at the top of the table.
pub(super) fn stopping_distance(speed: f32, mass: f32) -> f32 {
    3.0 + 47.0 * (speed * (mass * 0.0005).max(1.0) / 80.0).clamp(0.0, 1.0)
}

/// The traffic think: keep the cursor ahead of the car, find the steering point past other cars and pick
/// a speed. `was_blocked` is the result of the previous think (a blocked car looks further ahead).
pub fn think(
    net: &RoadNetwork,
    nav: &mut RoadNav,
    car: &CarState,
    others: &[Body],
    was_blocked: bool,
    rng: &mut impl RandomSource,
    cruise: &Cruise,
) -> Think {
    let speed = car.forward_speed.abs();
    let look = match was_blocked {
        true => 30.0 + speed * THINK_PERIOD + car.radius,
        false => look_ahead(speed, THINK_PERIOD, car.radius),
    };
    let ahead = cursor_ahead(nav, car.body.position);
    if !nav.dead_end && ahead < look {
        nav.advance_with_lookahead(net, look - ahead, Vec3::ZERO, look, rng);
    }
    // The other cars within the avoidable radius.
    let reach = look.max(2.0 * speed);
    let near: Vec<Body> = others.iter().filter(|o| o.position.distance(car.body.position) < reach).copied().collect();
    let occlusion = nav
        .trail()
        .and_then(|trail| update_occluded_position(trail, &car.body, nav.position, &near, true, nav.half_width));
    let target = occlusion.as_ref().map_or(nav.position, |o| o.position);
    let blocked = occlusion.as_ref().is_some_and(|o| o.avoidable != 0);
    Think { target, speed: compute_speed(net, nav, car, occlusion.as_ref(), cruise), blocked }
}

/// The wanted speed: the posted one, limited by the bends ahead, by the car in front and by a light that
/// is red (or amber and still stoppable), nothing at a dead end, and never far above the current speed.
fn compute_speed(
    net: &RoadNetwork,
    nav: &RoadNav,
    car: &CarState,
    occlusion: Option<&Occlusion>,
    cruise: &Cruise,
) -> f32 {
    if nav.dead_end {
        return 0.0;
    }
    let posted = posted_speed(net, nav, cruise);
    let current = car.forward_speed;
    let mut desired = posted;
    if let (Some(o), Some(trail)) = (occlusion, nav.trail()) {
        let k = trail_curvature(trail.cookies(), o.current_index, car.body.position, o, nav.position, current);
        let a = cruise.lateral_g * GRAVITY;
        desired = desired.min((a / (a / (posted * posted)).max(k.abs())).sqrt());
        if o.avoidable != 0 && !o.from_behind {
            let length = 2.0 * car.radius;
            let distance = (o.apex.distance(car.body.position) - length).max(0.0);
            let stop = stopping_distance(current, car.mass);
            if distance < stop {
                desired = (o.trail_speed * distance / stop).clamp(0.0, desired).min(current);
            }
        }
    }
    // A light that holds the car is an obstacle that stands at the stop line.
    let light = car.stop.and_then(|stop| stop.speed_limit(current.max(0.0), car.mass));
    desired = desired.min(light.unwrap_or(desired));
    if !cruise.limit_acceleration {
        return desired;
    }
    // The cap on how far the target may exceed the current speed would pin a stopped car to the brake (the
    // pedals treat anything under 0.5 m/s as "stop"), so a free road always allows a walking pace. Not
    // beyond what the light allows: at the line the car has to stay on the brake.
    let floor = CREEP_SPEED.min(light.unwrap_or(CREEP_SPEED));
    desired.min((current.max(0.0) + ACCELERATION_CAP * THINK_PERIOD).max(floor))
}

#[cfg(test)]
mod tests {
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
        CarState { body, forward_speed: speed, radius: 2.6, mass: 1680.0, stop: None }
    }

    fn lane_nav(net: &RoadNetwork, z_t: f32) -> RoadNav {
        let mut nav = RoadNav::traffic();
        nav.init_in_lane(net, 0, 1, z_t);
        nav.enable_trail(net);
        nav
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
        let t = think(&net, &mut nav, &car, &[], false, &mut SplitMix(1), &Cruise::default());
        assert_eq!(posted_speed(&net, &nav, &Cruise::default()), STREET_SPEED);
        assert_eq!(t.speed, STREET_SPEED);
        assert!(!t.blocked);
        // The cursor was brought up to the look-ahead distance and the car steers at it.
        let ahead = cursor_ahead(&nav, car.body.position);
        assert!(ahead >= look_ahead(STREET_SPEED, THINK_PERIOD, 2.6) - 0.5, "{ahead}");
        assert!(t.target.distance(nav.position) < 1e-3);
    }

    #[test]
    fn a_standing_car_may_only_ask_for_a_little_more() {
        let net = straight_net();
        let mut nav = lane_nav(&net, 0.1);
        let t = think(&net, &mut nav, &car_at(40.0, 10.0), &[], false, &mut SplitMix(1), &Cruise::default());
        assert!((t.speed - (10.0 + ACCELERATION_CAP * THINK_PERIOD)).abs() < 1e-5, "{}", t.speed);
    }

    #[test]
    fn a_stopped_car_on_a_free_road_pulls_away() {
        let net = straight_net();
        let mut nav = lane_nav(&net, 0.1);
        let t = think(&net, &mut nav, &car_at(40.0, 0.0), &[], false, &mut SplitMix(1), &Cruise::default());
        assert!(t.speed >= CREEP_SPEED, "{}", t.speed);
    }

    #[test]
    fn a_dead_end_means_stop() {
        let net = straight_net();
        let mut nav = lane_nav(&net, 0.1);
        nav.advance(&net, 1000.0, Vec3::ZERO, &mut SplitMix(1));
        assert!(nav.dead_end);
        let t = think(&net, &mut nav, &car_at(399.0, 10.0), &[], false, &mut SplitMix(1), &Cruise::default());
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
            let t = think(&net, &mut nav, &me, &[stopped], false, &mut SplitMix(1), &Cruise::default());
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
        let t = think(&net, &mut nav, &car_at(40.0, 10.0), &[], false, &mut SplitMix(1), &cruise);
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
            speed = think(&net, &mut nav, &car, &[], false, &mut SplitMix(1), &Cruise::default()).speed;
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
        let t = think(&net, &mut nav, &car, &[], false, &mut SplitMix(1), &Cruise::default());
        assert_eq!(t.speed, STREET_SPEED);
        assert!(!t.blocked, "a light is not a car in the way");
    }

    #[test]
    fn a_car_inside_the_junction_has_no_stop_line() {
        // Past the line the lookup gives nothing and the car carries on at the posted speed.
        let net = straight_net();
        let mut nav = lane_nav(&net, 0.1);
        let t = think(&net, &mut nav, &car_at(70.0, STREET_SPEED), &[], false, &mut SplitMix(1), &Cruise::default());
        assert_eq!(t.speed, STREET_SPEED);
    }

    #[test]
    fn a_car_waiting_at_a_red_light_stays_on_the_brake_and_pulls_away_on_green() {
        let net = straight_net();
        let mut nav = lane_nav(&net, 0.1);
        let line = 60.0 + HALF_LENGTH;
        let red = think(
            &net,
            &mut nav,
            &at_light(60.0, 0.0, line, Light::Red),
            &[],
            false,
            &mut SplitMix(1),
            &Cruise::default(),
        );
        assert!(red.speed < 0.5, "{}", red.speed);
        let green = think(
            &net,
            &mut nav,
            &at_light(60.0, 0.0, line, Light::Green),
            &[],
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
        let red = think(
            &net,
            &mut nav,
            &at_light(40.0, 0.0, 70.0, Light::Red),
            &[],
            false,
            &mut SplitMix(1),
            &Cruise::default(),
        );
        assert!(red.speed >= CREEP_SPEED, "{}", red.speed);
    }

    #[test]
    fn an_amber_the_car_cannot_stop_for_is_driven_through() {
        let net = straight_net();
        let mut nav = lane_nav(&net, 0.1);
        let close = at_light(40.0, STREET_SPEED, 40.0 + HALF_LENGTH + 4.0, Light::Amber);
        let t = think(&net, &mut nav, &close, &[], false, &mut SplitMix(1), &Cruise::default());
        assert_eq!(t.speed, STREET_SPEED);
        let far = at_light(40.0, STREET_SPEED, 40.0 + HALF_LENGTH + 20.0, Light::Amber);
        let t = think(&net, &mut nav, &far, &[], false, &mut SplitMix(1), &Cruise::default());
        assert!(t.speed < STREET_SPEED, "{}", t.speed);
    }

    #[test]
    fn a_patrol_cop_stops_at_red_as_well() {
        let net = straight_net();
        let mut nav = lane_nav(&net, 0.1);
        let car = at_light(40.0, 10.0, 40.0 + HALF_LENGTH + 5.0, Light::Red);
        let t = think(&net, &mut nav, &car, &[], false, &mut SplitMix(1), &Cruise::patrol(22.0, 32.0));
        assert!(t.speed < 10.0, "{}", t.speed);
    }
}
