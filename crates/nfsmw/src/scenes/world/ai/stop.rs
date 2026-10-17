//! Stopping for traffic lights. The original has no lights: cars never slow down for a junction. This is a
//! rewrite extension (the `traffic_lights` setting): a red light, or an amber the car can still stop for, is an
//! obstacle that stands at the stop line of the road the car is on.
//! Signals: `blackbox_roads::SignalController`.

use blackbox_roads::{
    Body, Crossing, Gate, Light, RoadNav, RoadNetwork, SignalController, crossing_point, earliest_oncoming,
};
use glam::{Vec2, Vec3};

use super::traffic::stopping_distance;

/// How hard (m/s squared) a car brakes for a light: the speed it may have at distance `d` from the line is
/// `sqrt(2 * BRAKING * d)`.
const BRAKING: f32 = 4.5;
/// Stop lines further ahead than this (metres) are not looked at.
pub const HORIZON: f32 = 80.0;
/// A car whose front is this far (metres) past the line still counts as waiting at it: braking overshoots a little.
const LINE_TOLERANCE: f32 = 1.5;
/// The car's heading and the road's must agree this much (dot product of the unit vectors) for the car to be
/// on the approach: about 45 degrees.
const HEADING_ALIGNMENT: f32 = 0.7;

/// The signals as the cars see them: the controller and what time it is on its clock.
#[derive(Debug, Clone, Copy)]
pub struct Lights<'a> {
    pub controller: &'a SignalController,
    /// Seconds of simulated time since the clock started.
    pub time: f32,
}

/// The stop line a car is heading for and what its light shows.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StopLine {
    /// Metres from the car's front to the line (negative a little: the car has just crossed it).
    pub distance: f32,
    pub light: Light,
}

impl StopLine {
    /// The light is not green: the car is waiting for it, or may have to.
    pub fn is_holding(&self) -> bool {
        self.light != Light::Green
    }

    /// The speed (m/s) the car may have so that it stands at the line, or `None` when it may carry on: the
    /// light is green, or amber and the car is already too close to stop before the line.
    pub fn speed_limit(&self, speed: f32, mass: f32) -> Option<f32> {
        let stops = match self.light {
            Light::Green => false,
            Light::Red => true,
            Light::Amber => self.distance >= stopping_distance(speed, mass),
        };
        stops.then(|| (2.0 * BRAKING * self.distance.max(0.0)).sqrt())
    }
}

/// The stop line the car is heading for, if there is a signalled one ahead. The candidates are the junctions
/// at the ends of the segment the cursor is on and `remembered`, the line found the last time (the cursor
/// runs ahead of the car and may already be past the junction). `remembered` is updated.
pub fn find(
    lights: &Lights<'_>,
    net: &RoadNetwork,
    nav: &RoadNav,
    body: &Body,
    remembered: &mut Option<usize>,
) -> Option<StopLine> {
    let controller = lights.controller;
    let ends = net.segment(nav.segment).nodes;
    let candidates = ends.iter().filter_map(|&node| controller.approach_at(node)).chain(*remembered);
    let at = Vec2::new(body.position.x, body.position.z);
    let found = candidates
        .filter_map(|i| {
            let approach = &controller.approaches()[i];
            let heading = Vec2::new(approach.heading.x, approach.heading.z);
            if heading.dot(body.forward) < HEADING_ALIGNMENT {
                return None;
            }
            let line = Vec2::new(approach.stop_position.x, approach.stop_position.z);
            let distance = heading.dot(line - at) - body.half_length;
            ((-LINE_TOLERANCE..=HORIZON).contains(&distance)).then_some((i, distance))
        })
        .min_by(|a, b| a.1.total_cmp(&b.1));
    *remembered = found.map(|(i, _)| i);
    found.map(|(i, distance)| StopLine { distance, light: controller.state(i, lights.time) })
}

/// The oncoming-gap rule of a turning car (`docs/specs/ai-traffic-obstacles.md` §10.2, a rewrite extension). A car at a
/// green line waits, as at a red light, while an oncoming car will reach the junction's crossing point within
/// [`blackbox_roads::WAIT_FOR_GAP`] seconds, and goes after [`blackbox_roads::GIVE_UP_AFTER`] seconds of waiting. Only a
/// car that has not passed the line is held. `crossing` is the car's own wait; it resets whenever the rule does not
/// hold the car. `others` are the other cars and the player, as bodies; `dt` is the time since the last decision.
pub fn yield_to_oncoming(
    lights: &Lights<'_>,
    net: &RoadNetwork,
    stop: StopLine,
    approach: usize,
    crossing: &mut Crossing,
    dt: f32,
    others: &[Body],
) -> StopLine {
    let holds_green = stop.light == Light::Green && stop.distance >= 0.0;
    if !holds_green {
        *crossing = Crossing::default();
        return stop;
    }
    let Some(point) = crossing_point(net, lights.controller, approach) else {
        *crossing = Crossing::default();
        return stop;
    };
    let heading = approach_heading(lights.controller, approach);
    let gate = crossing.update(dt, earliest_oncoming(point, heading, others));
    match gate {
        Gate::Wait => StopLine { light: Light::Red, ..stop },
        Gate::Go => stop,
    }
}

/// The horizontal unit heading of a signalled approach, as the planar vector `(x, z)`.
fn approach_heading(controller: &SignalController, approach: usize) -> Vec2 {
    let heading: Vec3 = controller.approaches()[approach].heading;
    Vec2::new(heading.x, heading.z)
}

#[cfg(test)]
mod tests {
    use blackbox_roads::{Road, RoadNode, RoadProfile, RoadSegment, Zone, flags, zone};
    use glam::Vec3;

    use super::*;

    const MASS: f32 = 1680.0;
    /// Metres from the middle of the test junction to its nodes, and from those to the far ends of the arms.
    const NODE_RADIUS: f32 = 10.0;
    const ARM_LENGTH: f32 = 100.0;
    const HALF_LENGTH: f32 = 2.2;

    /// A T junction: arms from the west, the east and the south end at nodes 0, 1 and 2 (far ends 3, 4, 5);
    /// segment `i` is the arm of node `i`, driven towards the junction; segments 3 to 5 join the nodes.
    fn t_junction() -> RoadNetwork {
        let arms = [Vec3::NEG_X, Vec3::X, Vec3::NEG_Z];
        let segment = |nodes: [u16; 2], flags: u16| RoadSegment {
            nodes,
            length: 100.0,
            road: Some(0),
            flags,
            start_handle: Vec3::ZERO,
            end_handle: Vec3::ZERO,
        };
        let mut nodes: Vec<RoadNode> = arms
            .iter()
            .enumerate()
            .map(|(i, d)| RoadNode { position: *d * NODE_RADIUS, profile: 0, segments: vec![i as u16] })
            .collect();
        nodes.extend(arms.iter().enumerate().map(|(i, d)| RoadNode {
            position: *d * (NODE_RADIUS + ARM_LENGTH),
            profile: 0,
            segments: vec![i as u16],
        }));
        let mut segments: Vec<RoadSegment> = (0..3).map(|i| segment([i + 3, i], 0)).collect();
        for (a, b) in [(0, 1), (0, 2), (1, 2)] {
            segments.push(segment([a, b], flags::DECISION | flags::INTERSECTION));
            let index = segments.len() as u16 - 1;
            nodes[usize::from(a)].segments.push(index);
            nodes[usize::from(b)].segments.push(index);
        }
        let lane = |offset| Zone { kind: zone::TRAFFIC, width: 4.0, offset };
        RoadNetwork {
            nodes,
            segments,
            profiles: vec![RoadProfile { middle: 1, zones: vec![lane(2.0), lane(2.0)] }],
            roads: vec![Road { scale: 1.0, speech_id: 0 }],
        }
    }

    /// A car on the west arm, driving east, at `x`.
    fn eastbound(x: f32) -> Body {
        Body {
            position: Vec3::new(x, 0.0, 0.0),
            velocity: Vec3::new(10.0, 0.0, 0.0),
            forward: Vec2::X,
            half_width: 0.9,
            half_length: HALF_LENGTH,
        }
    }

    fn find_for(
        net: &RoadNetwork,
        segment: u16,
        body: &Body,
        time: f32,
        memory: &mut Option<usize>,
    ) -> Option<StopLine> {
        let controller = SignalController::new(net);
        let mut nav = RoadNav::traffic();
        nav.place(net, segment, 1, 1, 0.5);
        find(&Lights { controller: &controller, time }, net, &nav, body, memory)
    }

    #[test]
    fn a_car_on_an_approach_sees_the_stop_line_and_its_light() {
        let net = t_junction();
        let stop = find_for(&net, 0, &eastbound(-60.0), 0.0, &mut None).expect("a stop line ahead");
        // The line is 8 m before the node at x = -10.
        assert!((stop.distance - (-18.0 + 60.0 - HALF_LENGTH)).abs() < 0.5, "{}", stop.distance);
        assert_eq!(stop.light, Light::Green, "the west arm starts green");
    }

    #[test]
    fn a_car_past_the_line_has_no_stop_line() {
        let net = t_junction();
        assert_eq!(find_for(&net, 0, &eastbound(-10.0), 0.0, &mut None), None);
        assert_eq!(find_for(&net, 3, &eastbound(0.0), 0.0, &mut None), None);
    }

    #[test]
    fn a_car_far_from_the_line_has_none_yet() {
        let net = t_junction();
        assert_eq!(find_for(&net, 0, &eastbound(-18.0 - HORIZON - HALF_LENGTH - 1.0), 0.0, &mut None), None);
    }

    #[test]
    fn a_car_leaving_the_junction_is_not_held() {
        let net = t_junction();
        let mut leaving = eastbound(-40.0);
        leaving.forward = Vec2::NEG_X;
        assert_eq!(find_for(&net, 0, &leaving, 0.0, &mut None), None);
    }

    #[test]
    fn the_line_is_remembered_when_the_cursor_is_already_past_the_junction() {
        let net = t_junction();
        // The cursor is on the east arm (segment 1 joins nodes 4 and 1): without a memory the west line is not a
        // candidate.
        let body = eastbound(-25.0);
        assert_eq!(find_for(&net, 1, &body, 0.0, &mut None), None);
        let mut memory = Some(0);
        let stop = find_for(&net, 1, &body, 0.0, &mut memory).expect("remembered");
        assert!(stop.distance > 0.0);
        assert_eq!(memory, Some(0));
        // Once the car is past the line the memory is dropped.
        assert_eq!(find_for(&net, 1, &eastbound(-5.0), 0.0, &mut memory), None);
        assert_eq!(memory, None);
    }

    #[test]
    fn the_light_follows_the_clock() {
        let net = t_junction();
        let red = find_for(&net, 0, &eastbound(-60.0), 17.5, &mut None).expect("line");
        assert_eq!(red.light, Light::Red);
    }

    fn line(distance: f32, light: Light) -> StopLine {
        StopLine { distance, light }
    }

    #[test]
    fn green_lets_the_car_through() {
        assert_eq!(line(10.0, Light::Green).speed_limit(10.0, MASS), None);
        assert!(!line(10.0, Light::Green).is_holding());
    }

    #[test]
    fn red_allows_the_speed_that_stops_at_the_line() {
        let limit = line(20.0, Light::Red).speed_limit(10.0, MASS).expect("red stops");
        assert!((limit - (2.0 * BRAKING * 20.0).sqrt()).abs() < 1e-4);
        assert_eq!(line(0.0, Light::Red).speed_limit(0.0, MASS), Some(0.0));
        // A little past the line still stands at it.
        assert_eq!(line(-1.0, Light::Red).speed_limit(2.0, MASS), Some(0.0));
        assert!(line(5.0, Light::Red).is_holding());
    }

    #[test]
    fn amber_stops_the_car_that_can_but_not_the_one_that_cannot() {
        let speed = 15.0;
        let need = stopping_distance(speed, MASS);
        assert!(line(need + 1.0, Light::Amber).speed_limit(speed, MASS).is_some());
        assert_eq!(line(need - 1.0, Light::Amber).speed_limit(speed, MASS), None);
    }
    /// An oncoming car on the east arm, driving west at 10 m/s, 25 m east of the junction's middle: it reaches the
    /// crossing point of the test junction in 2.5 s, which is inside the 3 s threshold.
    fn oncoming() -> Body {
        Body {
            position: Vec3::new(25.0, 0.0, 0.0),
            velocity: Vec3::new(-10.0, 0.0, 0.0),
            forward: Vec2::NEG_X,
            half_width: 0.9,
            half_length: HALF_LENGTH,
        }
    }

    /// The west approach's green line for a car at `x`, with the signals of `net` at `time`.
    fn green_line(net: &RoadNetwork, x: f32, time: f32) -> (SignalController, StopLine) {
        let controller = SignalController::new(net);
        let mut nav = RoadNav::traffic();
        nav.place(net, 0, 1, 1, 0.5);
        let stop = find(&Lights { controller: &controller, time }, net, &nav, &eastbound(x), &mut None)
            .expect("a stop line ahead");
        (controller, stop)
    }

    #[test]
    fn a_car_at_a_green_line_waits_for_an_oncoming_car_that_is_close() {
        let net = t_junction();
        let (controller, stop) = green_line(&net, -60.0, 0.0);
        assert_eq!(stop.light, Light::Green);
        let lights = Lights { controller: &controller, time: 0.0 };
        let mut crossing = Crossing::default();
        let held = yield_to_oncoming(&lights, &net, stop, 0, &mut crossing, 0.1, &[oncoming()]);
        assert_eq!(held.light, Light::Red, "held like a red light");
        assert!(held.is_holding());
        assert!(held.speed_limit(10.0, MASS).is_some(), "it brakes to the line");
    }

    #[test]
    fn a_car_at_a_green_line_goes_when_nothing_is_oncoming() {
        let net = t_junction();
        let (controller, stop) = green_line(&net, -60.0, 0.0);
        let lights = Lights { controller: &controller, time: 0.0 };
        let mut crossing = Crossing::default();
        let free = yield_to_oncoming(&lights, &net, stop, 0, &mut crossing, 0.1, &[]);
        assert_eq!(free, stop);
        let far = Body { position: Vec3::new(80.0, 0.0, 0.0), ..oncoming() };
        assert_eq!(
            yield_to_oncoming(&lights, &net, stop, 0, &mut crossing, 0.1, &[far]),
            stop,
            "4 s away is no threat"
        );
    }

    #[test]
    fn a_car_that_has_waited_eight_seconds_goes_anyway() {
        let net = t_junction();
        let (controller, stop) = green_line(&net, -60.0, 0.0);
        let lights = Lights { controller: &controller, time: 0.0 };
        let mut crossing = Crossing::default();
        let mut waits = 0;
        while yield_to_oncoming(&lights, &net, stop, 0, &mut crossing, 0.5, &[oncoming()]).light == Light::Red {
            waits += 1;
        }
        assert_eq!(waits, 16, "8 s at 0.5 s a think, then it goes");
    }

    #[test]
    fn a_red_light_or_a_car_past_the_line_is_not_changed_by_the_gate() {
        let net = t_junction();
        let (controller, red) = green_line(&net, -60.0, 17.5);
        assert_eq!(red.light, Light::Red);
        let lights = Lights { controller: &controller, time: 17.5 };
        let mut crossing = Crossing { waited: 3.0 };
        assert_eq!(yield_to_oncoming(&lights, &net, red, 0, &mut crossing, 0.1, &[oncoming()]), red);
        assert_eq!(crossing, Crossing::default(), "the wait resets when the gate does not hold the car");
        let past = StopLine { distance: -0.5, ..green_line(&net, -60.0, 0.0).1 };
        let lights = Lights { controller: &controller, time: 0.0 };
        assert_eq!(yield_to_oncoming(&lights, &net, past, 0, &mut crossing, 0.1, &[oncoming()]), past);
    }
}
