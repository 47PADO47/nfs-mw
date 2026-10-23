//! Searching the legs of a route and putting them together.

use super::{Goal, Leg, Route, RouteStep};
use crate::closest_segment;
use crate::{LaneType, NavKind, NodeInd, PathState, PathType, RoadNav, RoadNetwork, SegmentFilter, SegmentIndex};

/// The road of one leg: its segments in order, where on the first one it starts and where on the last one its goal is
/// (stored-direction parameters).
#[derive(Debug, Clone, PartialEq)]
pub struct LegPath {
    pub segments: Vec<u16>,
    pub from_t: f32,
    pub goal: (u16, f32),
}

/// Searches the road from `from` to `to` for `path_type`, starting the way `from` faces and arriving the way `to`
/// faces. `None` without a full path (a search that runs out of nodes is not a route).
pub fn build_leg(
    net: &RoadNetwork,
    index: &SegmentIndex,
    from: Goal,
    to: Goal,
    path_type: PathType,
) -> Option<LegPath> {
    // The plain distance, as for a path's goal: a goal is on its road, and the edge distance of a weighted search does
    // not tell the segment ending short of the point from the one under it.
    let start = closest_segment(net, index, from.position, from.forward, 0.0, SegmentFilter::default())?;
    let mut nav = RoadNav::new(NavKind::Direction, LaneType::Racing, SegmentFilter::default());
    nav.init_at_segment(net, start.segment, start.t, from.position, from.forward, true);
    if nav.find_path_to(net, index, to.position, Some(to.forward), path_type) != PathState::Full {
        return None;
    }
    let goal = nav.goal?;
    Some(LegPath { segments: nav.path, from_t: start.t, goal })
}

/// Puts a route together leg by leg.
#[derive(Debug)]
pub struct RouteBuilder<'a> {
    net: &'a RoadNetwork,
    route: Route,
}

impl<'a> RouteBuilder<'a> {
    pub fn new(net: &'a RoadNetwork) -> Self {
        Self { net, route: Route::default() }
    }

    /// Adds the leg from `from` to `to` along `path`, or as a missing leg (a straight line) with `None`.
    pub fn push(&mut self, from: Goal, to: Goal, path: Option<&LegPath>) {
        let leg = self.route.legs.len();
        let first = self.route.steps.len();
        let start = self.route.length;
        let mut length = 0.0;
        let mut goal = None;
        if let Some(path) = path.filter(|p| !p.segments.is_empty()) {
            let last = path.segments.len() - 1;
            for (i, &segment) in path.segments.iter().enumerate() {
                let node_ind = heading(self.net, &path.segments, i, path.from_t, path.goal.1);
                let travel = |stored: f32| if node_ind == 1 { stored } else { 1.0 - stored };
                let from_t = if i == 0 { travel(path.from_t) } else { 0.0 };
                let to_t = if i == last { travel(path.goal.1) } else { 1.0 };
                let step = RouteStep { segment, node_ind, from_t, to_t: to_t.max(from_t), start: start + length, leg };
                length += step.length(self.net);
                self.route.steps.push(step);
            }
            goal = Some(path.goal);
        }
        if goal.is_none() {
            length = from.position.distance(to.position);
        }
        let steps = first..self.route.steps.len();
        self.route.legs.push(Leg { from, to, steps, start, length, goal });
        self.route.length = start + length;
    }

    pub fn finish(self) -> Route {
        self.route
    }
}

/// The end of segment `i` of `segments` the route heads to: towards the segment after it, else away from the one
/// before it, else (a one-segment leg) from `from_t` towards `goal_t`.
fn heading(net: &RoadNetwork, segments: &[u16], i: usize, from_t: f32, goal_t: f32) -> NodeInd {
    let nodes = net.segment(segments[i]).nodes;
    if let Some(&next) = segments.get(i + 1) {
        return usize::from(net.segment(next).nodes.contains(&nodes[1]));
    }
    if let Some(&before) = i.checked_sub(1).and_then(|b| segments.get(b)) {
        return usize::from(net.segment(before).nodes.contains(&nodes[0]));
    }
    usize::from(goal_t >= from_t)
}
