//! The race's route for the Custom map: one lap of the opponents' route (`blackbox_roads::Route`) as a simplified 2D
//! polyline in render space, each point with its distance along the route, so the map can split it where the player
//! is. Built once per race, when the session starts (`scenes/world/race/route.rs`), and handed to the HUD as a
//! [`RaceRouteHandle`] in `HudState::race_route`, `None` outside a race.
//!
//! The route runs through every gate of every lap; the laps repeat the same road, so only the first lap's legs are
//! drawn. A lap race (circuit, knockout) is closed: its line runs on to its first point. A leg the route's search found
//! no road for is the straight line between its goals, as the route measures it.

use std::fmt;
use std::sync::Arc;

use blackbox_roads::{RoadNetwork, Route, centre_line};
use glam::Vec3;

use super::TOLERANCE_M;
use super::simplify::{kept, sample};

/// One lap of the race's route on the map.
#[derive(Debug, Clone, PartialEq)]
pub struct RaceRoute {
    /// Render space (x, y), metres; at least two points.
    points: Vec<[f32; 2]>,
    /// Metres along the route of each point, never falling.
    distances: Vec<f32>,
    /// The length of the lap drawn, and of the whole route (every lap), metres.
    lap_m: f32,
    total_m: f32,
}

impl RaceRoute {
    /// The first `per_lap` legs of `route` (one lap) on `net`, carried to render space by `to_render`, closed for a lap
    /// race. `None` for a route with no leg or too short to draw.
    pub fn build(
        net: &RoadNetwork,
        route: &Route,
        (per_lap, closed): (usize, bool),
        to_render: impl Fn(Vec3) -> [f32; 2],
    ) -> Option<Self> {
        let legs = route.legs.get(..per_lap.max(1).min(route.legs.len()))?;
        let last = legs.last()?;
        let mut samples: Vec<([f32; 2], f32)> = Vec::new();
        for leg in legs {
            if leg.missing() {
                push(&mut samples, to_render(leg.from.position), leg.start);
                push(&mut samples, to_render(leg.to.position), leg.start + leg.length);
                continue;
            }
            for step in route.steps.get(leg.steps.clone()).unwrap_or_default() {
                let curve = centre_line(net, step.segment, step.node_ind);
                let segment = net.segment(step.segment);
                for (t, at) in sample(&curve, (step.from_t, step.to_t), segment.is_curved(), TOLERANCE_M, &to_render) {
                    push(&mut samples, at, step.start + (t - step.from_t) * segment.length);
                }
            }
        }
        let lap_m = last.start + last.length;
        Self::from_samples(samples, lap_m, route.length, closed)
    }

    /// The route from its samples (point, distance), thinned, with its lap and total lengths; a `closed` one runs on to
    /// its first point. `None` for fewer than two distinct points.
    pub fn from_samples(samples: Vec<([f32; 2], f32)>, lap_m: f32, total_m: f32, closed: bool) -> Option<Self> {
        let mut deduped: Vec<([f32; 2], f32)> = Vec::new();
        for (at, distance) in samples {
            push(&mut deduped, at, distance);
        }
        if deduped.len() < 2 {
            return None;
        }
        let points: Vec<[f32; 2]> = deduped.iter().map(|(at, _)| *at).collect();
        let keep = kept(&points, TOLERANCE_M);
        let (mut points, mut distances): (Vec<[f32; 2]>, Vec<f32>) =
            deduped.into_iter().zip(keep).filter(|(_, kept)| *kept).map(|(sample, _)| sample).unzip();
        let first = points[0];
        let end = points[points.len() - 1];
        if closed && first != end {
            points.push(first);
            distances.push(lap_m.max(distances[distances.len() - 1]));
        }
        Some(Self { points, distances, lap_m, total_m })
    }

    pub fn points(&self) -> &[[f32; 2]] {
        &self.points
    }

    /// Metres along the drawn lap of a player with `remaining_m` of the whole route left: laps already driven are taken
    /// off. `None` without a lap length.
    pub fn along(&self, remaining_m: f32) -> Option<f32> {
        if self.lap_m <= 0.0 || !remaining_m.is_finite() {
            return None;
        }
        let driven = (self.total_m - remaining_m).max(0.0);
        if driven >= self.total_m {
            return Some(self.lap_m);
        }
        Some(driven.rem_euclid(self.lap_m))
    }

    /// The line split `along` metres into the lap: the stretch driven (start to there) and the stretch ahead (there to
    /// the end). Either may be empty.
    pub fn split(&self, along: f32) -> (Vec<[f32; 2]>, Vec<[f32; 2]>) {
        let next = self.distances.partition_point(|distance| *distance <= along);
        if next == 0 {
            return (Vec::new(), self.points.clone());
        }
        if next >= self.points.len() {
            return (self.points.clone(), Vec::new());
        }
        let (a, b) = (self.points[next - 1], self.points[next]);
        let (da, db) = (self.distances[next - 1], self.distances[next]);
        let t = match db > da {
            true => ((along - da) / (db - da)).clamp(0.0, 1.0),
            false => 0.0,
        };
        let at = [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t];
        let mut driven = self.points[..next].to_vec();
        driven.push(at);
        let mut ahead = vec![at];
        ahead.extend_from_slice(&self.points[next..]);
        (driven, ahead)
    }
}

/// Pushes a sample unless it repeats the last point.
fn push(samples: &mut Vec<([f32; 2], f32)>, at: [f32; 2], distance: f32) {
    if samples.last().is_some_and(|(last, _)| *last == at) {
        return;
    }
    samples.push((at, distance));
}

/// The race's route on the map, shared between the scene and the HUD. Two handles are equal when they share the same
/// route, as for [`MapRoadsHandle`](super::MapRoadsHandle).
#[derive(Clone)]
pub struct RaceRouteHandle(Arc<RaceRoute>);

impl RaceRouteHandle {
    pub fn new(route: RaceRoute) -> Self {
        Self(Arc::new(route))
    }

    pub fn route(&self) -> &RaceRoute {
        &self.0
    }
}

impl PartialEq for RaceRouteHandle {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

impl fmt::Debug for RaceRouteHandle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RaceRouteHandle").field("points", &self.0.points.len()).finish()
    }
}
