//! Going round a car that blocks the lane: a traffic car held up for a while moves to the next traffic lane of the same
//! direction when the gap is clear. A rewrite extension behind the `traffic_avoidance` setting; the original has no
//! lane change for traffic. Spec: `docs/specs/ai-traffic-obstacles.md` §10.1. The rules are in
//! `blackbox_roads::{Blocked, LaneChange, lane_clear}`; this file feeds them the car's neighbours and moves the navigator.

use blackbox_roads::{
    Blocked, Body, Command, Crossing, LaneChange, Other, Phase, RoadNav, RoadNetwork, SegmentIndex, Situation,
    adjacent_lanes, lane_clear, right_of, travel_profile,
};
use glam::Vec3;

use super::super::stop::{Lights, StopLine, yield_to_oncoming};
use super::super::traffic::THINK_PERIOD;
use super::AiCar;

/// Seconds the horn of a blocked car sounds.
const HONK_SECONDS: f32 = 1.5;
/// Metres from the player within which a blocked car sounds its horn: the horn is for the player to hear.
const HONK_RANGE: f32 = 20.0;

/// The avoidance state of one traffic car.
#[derive(Debug, Default)]
pub(super) struct Avoid {
    /// The `traffic_avoidance` setting: when off, the car never changes lane.
    enabled: bool,
    think_delta: Option<f32>,
    blocked: Blocked,
    change: LaneChange,
    /// How long the car has waited for an oncoming car at its green line (`stop::yield_to_oncoming`).
    crossing: Crossing,
}

impl Avoid {
    pub(super) fn set_think_delta(&mut self, dt: f32) {
        self.think_delta = Some(dt);
    }

    fn delta(&self) -> f32 {
        self.think_delta.unwrap_or(THINK_PERIOD)
    }
    pub(super) fn changing_lanes(&self) -> Option<(usize, usize)> {
        match self.change.phase {
            Phase::Moving { from, to, .. } => Some((from, to)),
            _ => None,
        }
    }

    pub(super) fn set_enabled(&mut self, on: bool) {
        self.enabled = on;
    }

    /// The blocked clock, when the setting is on. Off, the clock never runs and nothing reads it.
    pub(super) fn blocked_clock(&self) -> Option<Blocked> {
        self.enabled.then_some(self.blocked)
    }

    /// The horn this car sounds now, as its duration in seconds, or `None`. It sounds once per blocked episode
    /// (`Blocked::take_honk`), only with the setting on and the player within [`HONK_RANGE`] metres (`player_distance`;
    /// infinite when there is no player).
    pub(super) fn honk(&mut self, player_distance: f32) -> Option<f32> {
        if !self.enabled || player_distance > HONK_RANGE {
            return None;
        }
        self.blocked.take_honk().then_some(HONK_SECONDS)
    }

    /// The oncoming-gap rule for the car's stop line: a green line may be held like a red one while an oncoming car
    /// nears the junction. `approach` is the signalled approach the line is on (the car's `stop_memory`, which
    /// `stop::find` sets with the line). Off with the setting, and the line is returned unchanged. The
    /// think in `car.rs` calls it between the `before_think` and the `traffic::think` lines.
    pub(super) fn yield_at(
        &mut self,
        net: &RoadNetwork,
        lights: Option<Lights<'_>>,
        stop: Option<StopLine>,
        approach: Option<usize>,
        others: impl IntoIterator<Item = Body>,
    ) -> Option<StopLine> {
        if !self.enabled {
            return stop;
        }
        let (Some(lights), Some(stop), Some(approach)) = (lights, stop, approach) else {
            self.crossing = Crossing::default();
            return stop;
        };
        Some(yield_to_oncoming(
            &lights,
            net,
            stop,
            approach,
            &mut self.crossing,
            self.think_delta.unwrap_or(THINK_PERIOD),
            others,
        ))
    }

    /// Runs the lane rules for one think, before the car's speed is decided: the blocked clock runs, and the car may
    /// move to the next lane (the navigator is moved onto it). `blocked` is what the last think found.
    pub(super) fn before_think(
        &mut self,
        net: &RoadNetwork,
        index: &SegmentIndex,
        nav: &mut RoadNav,
        body: &Body,
        others: impl IntoIterator<Item = Body, IntoIter: Clone>,
        blocked: bool,
    ) {
        if !self.enabled {
            return;
        }
        let forward = Vec3::new(body.forward.x, 0.0, body.forward.y);
        let speed = body.velocity.dot(forward).abs();
        self.blocked.tick(self.delta(), blocked, speed);
        let profile = travel_profile(net, nav.segment, nav.node_ind, true);
        let lanes = blackbox_roads::forward_traffic_lanes(&profile);
        if !lanes.contains(&nav.lane) {
            return;
        }
        // Between changes the change machine only remembers the lane, so it follows the navigator.
        if self.change.phase == Phase::Keep {
            self.change = LaneChange::new(nav.lane);
        }
        let right = right_of(nav.forward);
        let relative = relative_to(others, body, nav.position, forward, right);
        let lateral_of = |lane: usize| lane_lateral(&profile, lane) - lane_lateral(&profile, nav.lane);
        let half_width = |lane: usize| profile.zones.get(lane).map_or(0.0, |z| z.width * 0.5) + body.half_width;
        let clear = |lane: usize| lane_clear(relative.clone(), lateral_of(lane), half_width(lane), speed);
        let situation =
            Situation { blocked, may_change: self.blocked.may_change_lane(speed), speed, metres: speed * self.delta() };
        let Command::Move(lane) = self.change.update(situation, adjacent_lanes(&lanes, nav.lane), clear) else {
            return;
        };
        // Onto the target lane at the car's own place: the lateral offset of the car from the navigator's lane line,
        // carried over to the target lane's line.
        let lateral_now = (body.position - nav.position).dot(right);
        let point = body.position + right * (lateral_of(lane) - lateral_now);
        match nav.init_at_point(net, index, point, forward, false) {
            true => nav.enable_trail(net),
            false => self.change = LaneChange::new(nav.lane),
        }
    }

    /// The sideways part of the steering target: the car's own lateral offset from the navigator's lane line, closing
    /// to zero over the blend. Zero when the car is not changing lane.
    pub(super) fn steer_offset(&self, nav: &RoadNav, car: Vec3) -> Vec3 {
        let right = right_of(nav.forward);
        right * (car - nav.position).dot(right) * self.change.remaining()
    }

    /// Adds the distance the car drove since the last physics step, for the blend.
    pub(super) fn advance(&mut self, metres: f32) {
        self.change.advance(metres);
    }
}

impl AiCar {
    /// The horn duration of this car if it sounds its horn now (see [`Avoid::honk`]); `player_distance` is the
    /// distance to the player in metres, infinite without one.
    pub(in crate::scenes::world::ai) fn take_honk(&mut self, player_distance: f32) -> Option<f32> {
        self.avoid.honk(player_distance)
    }

    /// The blocked clock of this car (`Blocked::stuck` says whether it is stuck), when the rewrite's avoidance is on.
    pub fn blocked_clock(&self) -> Option<Blocked> {
        self.avoid.blocked_clock()
    }
}

/// Every other car, placed against the car's direction and the navigator's lane line, as the iterator is read.
fn relative_to(
    others: impl IntoIterator<Item = Body, IntoIter: Clone>,
    body: &Body,
    lane_line: Vec3,
    forward: Vec3,
    right: Vec3,
) -> impl Iterator<Item = Other> + Clone {
    others.into_iter().map(move |other| Other {
        along: (other.position - body.position).dot(forward),
        lateral: (other.position - lane_line).dot(right),
        along_speed: other.velocity.dot(forward),
    })
}

/// The lateral offset of a traffic lane from the centre line, right of travel.
fn lane_lateral(profile: &blackbox_roads::RoadProfile, lane: usize) -> f32 {
    profile.signed_offset(lane.min(profile.zones.len().saturating_sub(1)))
}

#[cfg(test)]
mod tests {
    use blackbox_roads::{Road, RoadNetwork, RoadNode, RoadProfile, RoadSegment, SegmentIndex, Zone, zone};
    use glam::{Vec2, Vec3};

    use super::*;

    /// One straight road with two traffic lanes in each direction: lane 1 is the car's, 2 m right of the centre; lane
    /// 2 is the next one, 6 m right. Lane 0 is the lane on the other side.
    fn two_lanes() -> RoadNetwork {
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
            profiles: vec![RoadProfile { middle: 1, zones: vec![lane(2.0), lane(2.0), lane(6.0)] }],
            roads: vec![Road { scale: 1.0, speech_id: 0 }],
        }
    }

    fn car_at(z: f32, speed: f32) -> Body {
        Body {
            position: Vec3::new(2.0, 0.0, z),
            velocity: Vec3::new(0.0, 0.0, speed),
            forward: Vec2::Y,
            half_width: 0.9,
            half_length: 2.2,
        }
    }

    fn stopped_at(z: f32) -> Body {
        Body { velocity: Vec3::ZERO, ..car_at(z, 0.0) }
    }

    fn navigator(net: &RoadNetwork) -> RoadNav {
        let mut nav = RoadNav::traffic();
        nav.init_in_lane(net, 0, 1, 0.1);
        nav.enable_trail(net);
        nav
    }

    fn enabled() -> Avoid {
        Avoid { enabled: true, ..Avoid::default() }
    }

    #[test]
    fn a_car_blocked_for_three_seconds_moves_to_the_clear_lane_beside_it() {
        let net = two_lanes();
        let index = SegmentIndex::build(&net);
        let mut nav = navigator(&net);
        let (me, blocker) = (car_at(10.0, 2.0), stopped_at(30.0));
        let mut avoid = enabled();
        for _ in 0..10 {
            avoid.before_think(&net, &index, &mut nav, &me, [blocker], true);
        }
        assert_eq!(nav.lane, 1, "not yet: about 1.7 s blocked");
        for _ in 0..10 {
            avoid.before_think(&net, &index, &mut nav, &me, [blocker], true);
        }
        assert_eq!(nav.lane, 2, "moved over after about 3 s");
        // The steering target starts at the car, not at the new lane, so there is no step.
        let offset = avoid.steer_offset(&nav, me.position);
        assert!((offset.x + 4.0).abs() < 1e-3, "{offset:?}");
        avoid.advance(100.0);
        assert!(avoid.steer_offset(&nav, me.position).length() < 1e-3, "blended in");
    }

    #[test]
    fn a_car_with_the_setting_off_stays_in_its_lane() {
        let net = two_lanes();
        let index = SegmentIndex::build(&net);
        let mut nav = navigator(&net);
        let (me, blocker) = (car_at(10.0, 2.0), stopped_at(30.0));
        let mut avoid = Avoid::default();
        for _ in 0..40 {
            avoid.before_think(&net, &index, &mut nav, &me, [blocker], true);
        }
        assert_eq!(nav.lane, 1);
        assert_eq!(avoid.steer_offset(&nav, me.position), Vec3::ZERO);
    }

    #[test]
    fn a_car_that_is_not_blocked_never_changes_lane() {
        let net = two_lanes();
        let index = SegmentIndex::build(&net);
        let mut nav = navigator(&net);
        let mut avoid = enabled();
        for _ in 0..40 {
            avoid.before_think(&net, &index, &mut nav, &car_at(10.0, 2.0), [], false);
        }
        assert_eq!(nav.lane, 1);
    }

    #[test]
    fn a_fast_car_does_not_change_lane_however_long_it_is_blocked() {
        let net = two_lanes();
        let index = SegmentIndex::build(&net);
        let mut nav = navigator(&net);
        let mut avoid = enabled();
        for _ in 0..40 {
            avoid.before_think(&net, &index, &mut nav, &car_at(10.0, 9.0), [stopped_at(40.0)], true);
        }
        assert_eq!(nav.lane, 1, "above 5 m/s it does not move over");
    }

    #[test]
    fn a_blocked_car_honks_once_per_episode_when_the_player_is_near() {
        let mut avoid = enabled();
        for _ in 0..10 {
            avoid.blocked.tick(0.5, true, 0.0);
        }
        assert_eq!(avoid.honk(10.0), Some(HONK_SECONDS));
        assert_eq!(avoid.honk(10.0), None, "already honked this episode");
    }

    #[test]
    fn a_far_player_gets_no_horn_and_the_horn_waits_for_one() {
        let mut avoid = enabled();
        avoid.blocked.tick(blackbox_roads::HONK_AFTER + 0.5, true, 0.0);
        assert_eq!(avoid.honk(HONK_RANGE + 1.0), None, "the player is out of range");
        assert_eq!(avoid.honk(HONK_RANGE), Some(HONK_SECONDS), "still due: the episode did not use it up");
    }

    #[test]
    fn with_the_setting_off_there_is_no_horn_and_no_blocked_clock() {
        let mut avoid = Avoid::default();
        avoid.blocked.tick(10.0, true, 0.0);
        assert_eq!(avoid.honk(0.0), None);
        assert_eq!(avoid.blocked_clock(), None);
        assert_eq!(enabled().blocked_clock(), Some(Blocked::default()));
    }

    #[test]
    fn with_the_setting_off_a_green_line_is_not_held() {
        let net = two_lanes();
        let mut avoid = Avoid::default();
        let line = StopLine { distance: 5.0, light: blackbox_roads::Light::Green };
        let controller = blackbox_roads::SignalController::new(&net);
        let lights = Lights { controller: &controller, time: 0.0 };
        assert_eq!(avoid.yield_at(&net, Some(lights), Some(line), Some(0), []), Some(line));
        assert_eq!(avoid.yield_at(&net, None, None, None, []), None);
    }

    #[test]
    fn a_car_in_the_next_lane_keeps_the_car_in_its_lane() {
        let net = two_lanes();
        let index = SegmentIndex::build(&net);
        let mut nav = navigator(&net);
        let (me, blocker) = (car_at(10.0, 2.0), stopped_at(30.0));
        let beside = Body { position: Vec3::new(6.0, 0.0, 14.0), ..stopped_at(14.0) };
        let mut avoid = enabled();
        for _ in 0..40 {
            avoid.before_think(&net, &index, &mut nav, &me, [blocker, beside], true);
        }
        assert_eq!(nav.lane, 1, "the lane next to it is not clear");
    }
}
