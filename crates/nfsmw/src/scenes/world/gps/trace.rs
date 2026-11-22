//! The career-flow trace's GPS lines (`docs/testing.md`, "Career-flow trace"): the route as it is set (its target and its
//! road polyline), its end, and once a second while it is set a sample of the arrow against the route: the car's place,
//! heading and speed, the point the arrow aims at, the bearing wanted and drawn, the arrow's yaw and where the drawn
//! model's point goes, and the direction of the route's road where the car is. Angles are degrees in render space (x, y
//! on the ground, counter-clockwise).

use glam::{Vec2, Vec3};
use serde_json::{Value, json};

use super::{Guide, arrow, bearing, heading_of};
use crate::scenes::world::players::LEAD;
use crate::scenes::world::{WorldScene, space};

/// Seconds between two samples of the arrow.
const SAMPLE_SECS: f32 = 1.0;
/// Metres between the points of the route's polyline in the trace.
const POLYLINE_STEP_M: f32 = 25.0;

fn deg(radians: f32) -> f32 {
    (radians.to_degrees() * 10.0).round() / 10.0
}

fn round(value: f32) -> f32 {
    (value * 10.0).round() / 10.0
}

/// The direction of `d` on the ground, degrees.
fn angle_of(d: Vec2) -> f32 {
    deg(d.y.atan2(d.x))
}

/// The route's polyline thinned to a point every [`POLYLINE_STEP_M`], its last point kept.
fn thinned(path: &[[f32; 2]]) -> Vec<[f32; 2]> {
    let mut out: Vec<[f32; 2]> = Vec::new();
    let mut since = f32::INFINITY;
    let mut last: Option<[f32; 2]> = None;
    for &point in path {
        if let Some(previous) = last {
            since += Vec2::from(point).distance(Vec2::from(previous));
        }
        last = Some(point);
        if since >= POLYLINE_STEP_M {
            out.push([round(point[0]), round(point[1])]);
            since = 0.0;
        }
    }
    if let Some(end) = last.filter(|end| out.last() != Some(&[round(end[0]), round(end[1])])) {
        out.push([round(end[0]), round(end[1])]);
    }
    out
}

/// The direction of the polyline `path` at the point nearest `at`, and the distance to it: the direction of the piece of
/// line that point lies on.
fn path_direction(path: &[[f32; 2]], at: Vec2) -> Option<(f32, f32)> {
    let mut best: Option<(f32, f32)> = None;
    for pair in path.windows(2) {
        let (a, b) = (Vec2::from(pair[0]), Vec2::from(pair[1]));
        let piece = b - a;
        if piece.length_squared() <= 1e-6 {
            continue;
        }
        let t = ((at - a).dot(piece) / piece.length_squared()).clamp(0.0, 1.0);
        let distance = (a + piece * t).distance(at);
        if best.is_none_or(|(_, d)| distance < d) {
            best = Some((angle_of(piece), distance));
        }
    }
    best
}

impl WorldScene {
    /// The trace's line for a route just set (`gps_route`): the target and the route's road polyline.
    pub(super) fn trace_gps_route(&self, target: [f32; 2]) {
        if !crate::parity::career_trace::enabled() {
            return;
        }
        let car = self.players.lead().map(|drive| drive.pose().position);
        let route = self.nav_route();
        let line = json!({
            "target": [round(target[0]), round(target[1])],
            "route_target": route.as_ref().map(|route| [round(route.target[0]), round(route.target[1])]),
            "car": car.map(|car| [round(car.x), round(car.y)]),
            "segments": self.guide.route.as_ref().map(|route| route.path().len()),
            "polyline": route.as_ref().map(|route| thinned(&route.path)),
        });
        crate::parity::career_trace::record("gps_route", line);
    }

    /// The trace's line for a route that ended or was cleared (`gps_end`): `why` is `finished`, `lost` or `cleared`.
    pub(super) fn trace_gps_end(why: &str) {
        crate::parity::career_trace::record("gps_end", json!({"why": why}));
    }

    /// Once a second while the lead's route is set: the arrow against the route (`gps_sample`).
    pub(super) fn trace_gps_sample(&mut self, dt: f32) {
        if !crate::parity::career_trace::enabled() || self.guide.route.is_none() {
            return;
        }
        self.guide.sample_clock += dt;
        if self.guide.sample_clock < SAMPLE_SECS {
            return;
        }
        self.guide.sample_clock = 0.0;
        let line = self.gps_sample_line(&self.guide);
        crate::parity::career_trace::record("gps_sample", line);
    }

    fn gps_sample_line(&self, guide: &Guide) -> Value {
        let (Some(route), Some(drive)) = (guide.route.as_ref(), self.players.lead()) else { return Value::Null };
        let pose = drive.physics_pose();
        let car = pose.position;
        let heading = heading_of(pose.rotation);
        let aim = space::to_render(route.arrow_point().to_array());
        let target = space::to_render(route.target().to_array());
        let wanted = bearing(route, pose.rotation, car);
        let yaw = heading - guide.bearing;
        // Where the drawn model's point goes: the measured point axis through the arrow's transform.
        let tip = arrow::transform(Vec3::ZERO, yaw, 1.0).transform_vector3(arrow::MODEL_POINT);
        let nav = self.nav_route();
        let road = nav.as_ref().and_then(|nav| path_direction(&nav.path, car.truncate()));
        json!({
            "car": [round(car.x), round(car.y)],
            "heading": deg(heading),
            "speed": round(drive.speed().abs()),
            "state": format!("{:?}", route.state()),
            "look_ahead": round(route.look_ahead()),
            "aim": [round(aim.x), round(aim.y)],
            "aim_dir": angle_of((aim - car).truncate()),
            "bearing_wanted": wanted.map(deg),
            "bearing_drawn": deg(guide.bearing),
            "arrow_yaw": deg(yaw),
            "arrow_tip": angle_of(tip.truncate()),
            "arrow_shown": self.route_arrow_for(LEAD).is_some(),
            "road_dir": road.map(|(angle, _)| angle),
            "off_road_m": road.map(|(_, distance)| round(distance)),
            "to_target_m": round((target - car).truncate().length()),
            "next_turn": nav.and_then(|nav| nav.next_turn).map(|turn| json!({
                "direction": format!("{:?}", turn.direction), "distance_m": round(turn.distance_m)})),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_polyline_keeps_a_point_every_step_and_its_end() {
        let path: Vec<[f32; 2]> = (0..=10).map(|i| [i as f32 * 10.0, 0.0]).collect();
        assert_eq!(thinned(&path), vec![[0.0, 0.0], [30.0, 0.0], [60.0, 0.0], [90.0, 0.0], [100.0, 0.0]]);
    }

    #[test]
    fn the_road_direction_is_that_of_the_nearest_piece() {
        let path = [[0.0, 0.0], [100.0, 0.0], [100.0, 100.0]];
        let (angle, distance) = path_direction(&path, Vec2::new(98.0, 60.0)).unwrap();
        assert_eq!((angle, distance), (90.0, 2.0));
    }
}
