//! Path finding: A* on the road graph.
//! Spec: `docs/specs/ai-pathfinder.md`.

use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap};

use glam::Vec3;

use crate::{RoadNetwork, flags};

/// The longest path a navigator holds.
pub const MAX_PATH_SEGMENTS: usize = 510;
/// Search nodes one search may use.
pub const MAX_SEARCH_NODES: usize = 3072;

/// Who is asking, which decides the segments a route may use.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathType {
    Cop,
    Gps,
    Racer,
    Player,
    RaceRoute,
    Chopper,
    None,
}

impl PathType {
    /// Whether the route may take `segment`, entered at `from` (the node it is left through).
    fn admits(self, net: &RoadNetwork, segment: u16, from: u16) -> bool {
        let seg = net.segment(segment);
        let one_way_against = seg.is_one_way() && seg.nodes[0] != from;
        let barrier = seg.has(flags::CROSSES_BARRIER) || seg.has(flags::CROSSES_DRIVE_THROUGH_BARRIER);
        match self {
            PathType::Cop => !one_way_against,
            PathType::Chopper | PathType::None => true,
            PathType::Gps | PathType::Racer | PathType::Player | PathType::RaceRoute => !one_way_against && !barrier,
        }
    }
}

/// How a search ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathState {
    /// The path reaches the goal.
    Full,
    /// The search ran out of nodes: the path is the best partial one.
    Half,
    /// There is no route.
    NoWay,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PathResult {
    /// Segments in order, the one the search started on first.
    pub segments: Vec<u16>,
    pub state: PathState,
}

/// What to search for.
#[derive(Debug, Clone, Copy)]
pub struct PathRequest {
    /// The segment the traveller is on and the node it heads to.
    pub segment: u16,
    pub node: u16,
    /// Also start from the node behind (the traveller may turn round at once).
    pub may_turn_round: bool,
    pub goal_segment: u16,
    /// The node the path must arrive through, when the goal has a direction.
    pub goal_node: Option<u16>,
    pub goal_position: Vec3,
    pub path_type: PathType,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct Open {
    f: f32,
    key: (u16, u16),
}

impl Eq for Open {}

impl Ord for Open {
    fn cmp(&self, other: &Self) -> Ordering {
        // A min-heap on `f`.
        other.f.total_cmp(&self.f).then_with(|| other.key.cmp(&self.key))
    }
}

impl PartialOrd for Open {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// Finds a route. A state is `(node, segment used to reach it)`; a segment costs its length.
pub fn find_path(net: &RoadNetwork, request: &PathRequest) -> PathResult {
    let heuristic = |node: u16| net.node(node).position.distance(request.goal_position);
    let mut best_g: HashMap<(u16, u16), f32> = HashMap::new();
    let mut parent: HashMap<(u16, u16), (u16, u16)> = HashMap::new();
    let mut open = BinaryHeap::new();

    let seg = net.segment(request.segment);
    let mut starts = vec![request.node];
    if request.may_turn_round
        && let Some(other) = seg.other_node(request.node)
    {
        starts.push(other);
    }
    for node in starts {
        let key = (node, request.segment);
        best_g.insert(key, 0.0);
        open.push(Open { f: heuristic(node), key });
    }

    let (mut expanded, mut best_partial) = (0usize, None::<((u16, u16), f32)>);
    while let Some(Open { f, key }) = open.pop() {
        let g = best_g[&key];
        if f > g + heuristic(key.0) + 1e-3 {
            continue; // a stale heap entry
        }
        let (node, used) = key;
        let at_goal = used == request.goal_segment && request.goal_node.is_none_or(|n| n == node);
        if at_goal {
            return PathResult { segments: unwind(&parent, key), state: PathState::Full };
        }
        expanded += 1;
        if expanded > MAX_SEARCH_NODES {
            return match (request.path_type, best_partial) {
                (PathType::RaceRoute, _) | (_, None) => PathResult { segments: Vec::new(), state: PathState::NoWay },
                (_, Some((key, _))) => PathResult { segments: unwind(&parent, key), state: PathState::Half },
            };
        }
        if best_partial.is_none_or(|(_, h)| heuristic(node) < h) {
            best_partial = Some((key, heuristic(node)));
        }
        for &e in &net.node(node).segments {
            if e == used {
                continue;
            }
            let (used_seg, next_seg) = (net.segment(used), net.segment(e));
            if used_seg.is_decision() && next_seg.is_decision() && request.path_type != PathType::Cop {
                continue;
            }
            if !request.path_type.admits(net, e, node) {
                continue;
            }
            let Some(far) = next_seg.other_node(node) else { continue };
            let (next_key, next_g) = ((far, e), g + next_seg.length);
            if best_g.get(&next_key).is_some_and(|&old| old <= next_g) {
                continue;
            }
            best_g.insert(next_key, next_g);
            parent.insert(next_key, key);
            open.push(Open { f: next_g + heuristic(far), key: next_key });
        }
    }
    PathResult { segments: Vec::new(), state: PathState::NoWay }
}

/// The segments from the start to `key`, at most [`MAX_PATH_SEGMENTS`] (the goal end is dropped beyond that).
fn unwind(parent: &HashMap<(u16, u16), (u16, u16)>, mut key: (u16, u16)) -> Vec<u16> {
    let mut segments = vec![key.1];
    while let Some(&p) = parent.get(&key) {
        segments.push(p.1);
        key = p;
    }
    segments.reverse();
    // A start state that is revisited through the parents is the same segment: collapse repeats.
    segments.dedup();
    segments.truncate(MAX_PATH_SEGMENTS);
    segments
}
