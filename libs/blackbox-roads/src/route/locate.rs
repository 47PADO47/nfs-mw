//! The point of a route nearest a position.

use std::ops::Range;

use glam::Vec3;

use super::Route;
use crate::{RoadNetwork, centre_line};

/// Height difference to the route that costs nothing (suspension, kerbs, ramps), and the score per metre beyond it,
/// as in the closest-segment search: a bridge over the route is not on it.
const HEIGHT_SLACK: f32 = 3.0;
const HEIGHT_WEIGHT: f32 = 3.0;

/// A point on the route.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RoutePoint {
    /// The piece of [`Route::steps`] it is on, and its leg.
    pub step: usize,
    pub leg: usize,
    /// Metres along the route.
    pub distance: f32,
    pub position: Vec3,
    /// Unit direction of travel there.
    pub forward: Vec3,
    /// How far the position looked for is from it on the ground, and how far above it (negative below), metres.
    pub gap: f32,
    pub rise: f32,
}

impl Route {
    /// The point of legs `legs` nearest to `point`: the nearest on the ground, a road far above or below the point
    /// scoring worse. `None` when those legs have no road.
    pub fn locate(&self, net: &RoadNetwork, point: Vec3, legs: Range<usize>) -> Option<RoutePoint> {
        let mut best: Option<(f32, RoutePoint)> = None;
        for leg in legs.filter_map(|l| self.legs.get(l)) {
            for index in leg.steps.clone() {
                let step = self.steps[index];
                let length = net.segment(step.segment).length;
                let curve = centre_line(net, step.segment, step.node_ind);
                let t = curve.closest_t(point, length).clamp(step.from_t, step.to_t);
                let position = curve.position(t);
                let gap = (point.x - position.x).hypot(point.z - position.z);
                let rise = point.y - position.y;
                let score = gap + HEIGHT_WEIGHT * (rise.abs() - HEIGHT_SLACK).max(0.0);
                if best.is_some_and(|(s, _)| s <= score) {
                    continue;
                }
                let forward = curve.tangent(t).try_normalize().unwrap_or(leg.to.forward);
                let distance = step.start + (t - step.from_t) * length;
                let found = RoutePoint { step: index, leg: step.leg, distance, position, forward, gap, rise };
                best = Some((score, found));
            }
        }
        best.map(|(_, found)| found)
    }
}
