//! Navigators that follow a found path.
//! Spec: `docs/specs/ai-road-network.md` (§4.2), `docs/specs/ai-pathfinder.md` (§5).

use glam::Vec3;

use super::next::Step;
use super::{NavKind, RoadNav};
use crate::{PathRequest, PathState, PathType, RoadNetwork, SegmentIndex, centre_line, closest_segment, find_path};

impl RoadNav {
    /// Finds a route to `goal` (a point, with an optional direction the goal is reached in) for
    /// `path_type` and starts following it. Returns how the search ended; nothing changes without a route.
    pub fn find_path_to(
        &mut self,
        net: &RoadNetwork,
        index: &SegmentIndex,
        goal: Vec3,
        goal_direction: Option<Vec3>,
        path_type: PathType,
    ) -> PathState {
        let heading = goal_direction.unwrap_or(Vec3::X);
        let Some(located) = closest_segment(net, index, goal, heading, 0.0, self.filter) else {
            return PathState::NoWay;
        };
        let goal_node = goal_direction.map(|d| {
            let forward = centre_line(net, located.segment, 1).tangent(located.t);
            net.segment(located.segment).nodes[usize::from(d.dot(forward) >= 0.0)]
        });
        let node = net.segment(self.segment).nodes[self.node_ind];
        let result = find_path(
            net,
            &PathRequest {
                segment: self.segment,
                node,
                may_turn_round: path_type != PathType::RaceRoute,
                goal_segment: located.segment,
                goal_node,
                goal_position: goal,
                path_type,
            },
        );
        if result.segments.is_empty() || (path_type == PathType::Gps && result.state != PathState::Full) {
            return result.state;
        }
        self.kind = NavKind::Path;
        self.path_type = path_type;
        self.goal = Some((located.segment, located.t));
        self.path = result.segments;
        self.turn_towards_path(net);
        result.state
    }

    /// Makes the cursor head the way the path leaves its segment.
    fn turn_towards_path(&mut self, net: &RoadNetwork) {
        let [first, second, ..] = self.path[..] else { return };
        if first != self.segment {
            return;
        }
        let seg = net.segment(first);
        let wanted = match net.segment(second).nodes {
            n if n.contains(&seg.nodes[1]) => 1,
            n if n.contains(&seg.nodes[0]) => 0,
            _ => return,
        };
        if wanted != self.node_ind {
            self.reverse(net);
        }
    }

    /// Turns the cursor round on its segment.
    pub fn reverse(&mut self, net: &RoadNetwork) {
        let zones = travel_zone_count(net, self);
        self.node_ind = 1 - self.node_ind;
        self.t = 1.0 - self.t;
        // The same physical lane has the mirrored index in the other direction's profile.
        self.lane = zones.saturating_sub(1).saturating_sub(self.lane);
        self.dead_end = false;
        self.rebuild(net);
        self.reset_trail(net);
    }

    pub(super) fn next_path(&self, net: &RoadNetwork, toward: Vec3) -> Step {
        let node = net.segment(self.segment).nodes[self.node_ind];
        let successor = self.path.iter().position(|&s| s == self.segment).and_then(|i| self.path.get(i + 1));
        let Some(&next) = successor else {
            // The end of the path: a GPS stops, others carry on in direction mode.
            return match self.path_type {
                PathType::Gps => Step::Stay,
                _ => self.next_direction(net, toward),
            };
        };
        match net.segment(next).other_node(node) {
            Some(_) => self.snap_onto(net, next, node),
            // The path turns round on this segment.
            None => Step::To { segment: self.segment, node_ind: 1 - self.node_ind, lane: self.lane },
        }
    }

    /// Metres of road left to the goal along the path.
    pub fn distance_remaining(&self, net: &RoadNetwork) -> f32 {
        let Some(at) = self.path.iter().position(|&s| s == self.segment) else { return 0.0 };
        let Some((goal, goal_t)) = self.goal.filter(|g| self.path.last() == Some(&g.0)) else {
            // No goal: the rest of the listed path.
            let rest: f32 = self.path[at + 1..].iter().map(|&s| net.segment(s).length).sum();
            return (1.0 - self.t) * net.segment(self.segment).length + rest;
        };
        let along = |entered_from: u16| match net.segment(goal).nodes[0] == entered_from {
            true => goal_t,
            false => 1.0 - goal_t,
        };
        if goal == self.segment {
            let progress = match self.node_ind {
                1 => goal_t,
                _ => 1.0 - goal_t,
            };
            return (progress - self.t).max(0.0) * net.segment(goal).length;
        }
        let between: f32 = self.path[at + 1..self.path.len() - 1].iter().map(|&s| net.segment(s).length).sum();
        let previous = self.path[self.path.len() - 2];
        // The goal segment is entered through the node it shares with the one before it.
        let entered = net.segment(goal).nodes.into_iter().find(|n| net.segment(previous).nodes.contains(n));
        let partial = entered.map_or(1.0, along);
        (1.0 - self.t) * net.segment(self.segment).length + between + partial * net.segment(goal).length
    }

    /// Whether the cursor is on its path and its successor shares the node being approached.
    pub fn on_path(&self, net: &RoadNetwork) -> bool {
        let node = net.segment(self.segment).nodes[self.node_ind];
        let Some(at) = self.path.iter().position(|&s| s == self.segment) else { return false };
        match self.path.get(at + 1) {
            Some(&next) => net.segment(next).nodes.contains(&node),
            None => true,
        }
    }
}

fn travel_zone_count(net: &RoadNetwork, nav: &RoadNav) -> usize {
    crate::travel_profile(net, nav.segment, nav.node_ind, false).zones.len()
}
