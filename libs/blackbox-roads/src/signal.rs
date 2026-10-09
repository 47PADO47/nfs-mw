//! Traffic signals for the junctions of a road network.
//!
//! The data has no signal records: a junction is a group of nodes joined by decision segments and the original
//! game's cars never slow down for one. This module is an extension: it builds the junction groups, gives every
//! approach a stop line and a phase, and runs a fixed-time controller that says which approach has a green light
//! when. Nothing here reads the file; the same network always gives the same signals.
//! Format: `docs/formats/road-network.md` ("Junctions").

use std::collections::HashMap;

use glam::Vec3;

use crate::{Bezier, NodeInd, RoadNetwork, centre_line, travel_profile, zone};

/// Metres before the junction node, measured along the approaching segment, where a stopping car waits.
pub const STOP_LINE_DISTANCE: f32 = 8.0;
/// A junction gets signals when at least this many roads can be entered from it (T and four-way junctions).
pub const MIN_SIGNAL_APPROACHES: usize = 3;
/// Seconds a phase shows green.
pub const GREEN_TIME: f32 = 14.0;
/// Seconds of amber after the green.
pub const AMBER_TIME: f32 = 3.0;
/// Seconds with red in both directions before the other phase turns green.
pub const ALL_RED_TIME: f32 = 2.0;
/// Seconds the cycle of one junction is shifted per junction index, so that the lights of a city do not
/// all change at once.
pub const OFFSET_PER_JUNCTION: f32 = 5.3;
/// Two approaches whose headings are at most this far apart (the absolute dot product of the unit headings is at
/// least this) from the first approach of the junction, in either direction, share its phase: cos 45 degrees.
pub const PHASE_ALIGNMENT: f32 = 0.707;
/// The phases a junction cycles through.
pub const PHASES: usize = 2;
/// The distance (metres) from the centre line to the kerb assumed for a road without a traffic lane on its right.
pub const DEFAULT_KERB: f32 = 4.0;
/// Pieces the approach curve is cut into to find the stop line.
const STOP_LINE_SAMPLES: usize = 32;

/// What a signal shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Light {
    Green,
    Amber,
    Red,
}

/// How long each stage of a phase lasts, in seconds.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Timing {
    pub green: f32,
    pub amber: f32,
    pub all_red: f32,
}

impl Default for Timing {
    fn default() -> Self {
        Self { green: GREEN_TIME, amber: AMBER_TIME, all_red: ALL_RED_TIME }
    }
}

impl Timing {
    /// Seconds one phase takes: green, amber and the all-red gap.
    pub fn phase_length(&self) -> f32 {
        self.green + self.amber + self.all_red
    }

    /// Seconds all phases take together.
    pub fn cycle(&self) -> f32 {
        self.phase_length() * PHASES as f32
    }
}

/// A road that enters a junction.
#[derive(Debug, Clone, PartialEq)]
pub struct Approach {
    /// The junction (index into [`SignalController::junctions`]) it belongs to.
    pub junction: usize,
    /// The plain segment cars drive along to reach the junction.
    pub segment: u16,
    /// The node of the junction the segment ends at.
    pub node: u16,
    /// Which end of the segment that node is (`1` is the stored direction).
    pub node_ind: NodeInd,
    /// Where cars wait: on the centre line, [`STOP_LINE_DISTANCE`] metres before the node along the segment.
    pub stop_position: Vec3,
    /// Unit direction of travel at the stop line, in the horizontal plane.
    pub heading: Vec3,
    /// Which phase (0 or 1) has the green light.
    pub phase: usize,
    /// Metres from the centre line to the outer edge of the right-most traffic lane in the direction of travel:
    /// where a signal post stands.
    pub kerb: f32,
}

/// A group of nodes joined by decision segments.
#[derive(Debug, Clone, PartialEq)]
pub struct Junction {
    pub nodes: Vec<u16>,
    /// Indices into [`SignalController::approaches`]; empty when the junction has no signals.
    pub approaches: Vec<usize>,
    /// Seconds the junction's cycle is shifted.
    pub offset: f32,
}

impl Junction {
    pub fn has_signals(&self) -> bool {
        !self.approaches.is_empty()
    }
}

/// The signals of a road network and the clock they run on.
#[derive(Debug, Clone, PartialEq)]
pub struct SignalController {
    timing: Timing,
    junctions: Vec<Junction>,
    approaches: Vec<Approach>,
    by_node: HashMap<u16, usize>,
}

impl SignalController {
    /// The signals of `net` with the default timing.
    pub fn new(net: &RoadNetwork) -> Self {
        Self::with_timing(net, Timing::default())
    }

    pub fn with_timing(net: &RoadNetwork, timing: Timing) -> Self {
        let mut controller = Self { timing, junctions: Vec::new(), approaches: Vec::new(), by_node: HashMap::new() };
        for nodes in junction_groups(net) {
            controller.add_junction(net, nodes);
        }
        controller
    }

    pub fn timing(&self) -> Timing {
        self.timing
    }

    /// Every junction group, signalled or not.
    pub fn junctions(&self) -> &[Junction] {
        &self.junctions
    }

    /// The approaches of the signalled junctions.
    pub fn approaches(&self) -> &[Approach] {
        &self.approaches
    }

    /// The signalled approach that enters the junction at `node`, as an index into [`Self::approaches`].
    pub fn approach_at(&self, node: u16) -> Option<usize> {
        self.by_node.get(&node).copied()
    }

    /// What the signal of `approach` shows `time` seconds after the clock started.
    pub fn state(&self, approach: usize, time: f32) -> Light {
        let approach = &self.approaches[approach];
        let junction = &self.junctions[approach.junction];
        let phase_length = self.timing.phase_length();
        let cycle = self.timing.cycle();
        let since_green = (time + junction.offset - approach.phase as f32 * phase_length).rem_euclid(cycle);
        match since_green {
            s if s < self.timing.green => Light::Green,
            s if s < self.timing.green + self.timing.amber => Light::Amber,
            _ => Light::Red,
        }
    }

    fn add_junction(&mut self, net: &RoadNetwork, nodes: Vec<u16>) {
        let index = self.junctions.len();
        let mut found: Vec<Approach> = nodes.iter().flat_map(|&node| approaches_at(net, node, index)).collect();
        let offset = (index as f32 * OFFSET_PER_JUNCTION).rem_euclid(self.timing.cycle());
        if found.len() < MIN_SIGNAL_APPROACHES {
            self.junctions.push(Junction { nodes, approaches: Vec::new(), offset });
            return;
        }
        assign_phases(&mut found);
        let first = self.approaches.len();
        for approach in found {
            self.by_node.insert(approach.node, self.approaches.len());
            self.approaches.push(approach);
        }
        let approaches = (first..self.approaches.len()).collect();
        self.junctions.push(Junction { nodes, approaches, offset });
    }
}

/// Connected groups of nodes joined by decision segments, each sorted by node index, in order of their
/// lowest node.
fn junction_groups(net: &RoadNetwork) -> Vec<Vec<u16>> {
    let mut parent: Vec<usize> = (0..net.nodes.len()).collect();
    fn root(parent: &mut [usize], mut i: usize) -> usize {
        while parent[i] != i {
            parent[i] = parent[parent[i]];
            i = parent[i];
        }
        i
    }
    let mut joined = vec![false; net.nodes.len()];
    for seg in net.segments.iter().filter(|s| s.is_decision()) {
        let [a, b] = seg.nodes.map(usize::from);
        joined[a] = true;
        joined[b] = true;
        let (ra, rb) = (root(&mut parent, a), root(&mut parent, b));
        parent[ra.max(rb)] = ra.min(rb);
    }
    let mut groups: HashMap<usize, Vec<u16>> = HashMap::new();
    for node in (0..net.nodes.len()).filter(|&n| joined[n]) {
        groups.entry(root(&mut parent, node)).or_default().push(node as u16);
    }
    let mut groups: Vec<Vec<u16>> = groups.into_values().collect();
    groups.sort_by_key(|g| g[0]);
    groups
}

/// The plain segments cars can drive into the junction through `node`.
fn approaches_at(net: &RoadNetwork, node: u16, junction: usize) -> Vec<Approach> {
    let mut found = Vec::new();
    for &s in &net.node(node).segments {
        let seg = net.segment(s);
        if seg.is_decision() || seg.nodes[0] == seg.nodes[1] {
            continue;
        }
        let node_ind: NodeInd = usize::from(seg.nodes[1] == node);
        // A one-way segment leaves the junction when its stored direction starts there.
        if seg.is_one_way() && node_ind == 0 {
            continue;
        }
        let (stop_position, heading) = stop_line(&centre_line(net, s, node_ind), STOP_LINE_DISTANCE);
        let kerb = kerb_offset(net, s, node_ind);
        found.push(Approach { junction, segment: s, node, node_ind, stop_position, heading, phase: 0, kerb });
    }
    found
}

/// Metres from the centre line to the outer edge of the right-most traffic lane at the junction end of the
/// segment, read in the direction of travel towards it.
fn kerb_offset(net: &RoadNetwork, segment: u16, node_ind: NodeInd) -> f32 {
    let profile = travel_profile(net, segment, node_ind, true);
    let outermost = profile.lanes_of(zone::TRAFFIC, true).pop();
    outermost.map_or(DEFAULT_KERB, |i| profile.signed_offset(i) + profile.zones[i].width / 2.0)
}

/// The point `metres` before the end of `curve` (measured along it) and the horizontal direction of travel
/// there. A curve shorter than that gives its start.
fn stop_line(curve: &Bezier, metres: f32) -> (Vec3, Vec3) {
    let planar = |v: Vec3| Vec3::new(v.x, 0.0, v.z).try_normalize();
    let heading_at =
        |t: f32| planar(curve.tangent(t)).or_else(|| planar(curve.end() - curve.start())).unwrap_or(Vec3::Z);
    let (mut previous, mut walked) = (curve.end(), 0.0);
    for i in (0..STOP_LINE_SAMPLES).rev() {
        let t = i as f32 / STOP_LINE_SAMPLES as f32;
        let point = curve.position(t);
        let piece = previous.distance(point);
        if walked + piece >= metres {
            // Inside this piece: back off from the point it ends at.
            let along = (metres - walked) / piece.max(f32::EPSILON);
            return (previous.lerp(point, along), heading_at(t));
        }
        walked += piece;
        previous = point;
    }
    (curve.start(), heading_at(0.0))
}

/// Splits `approaches` into two phases: the roads along the axis of the first one, and the rest. When the
/// first split leaves one phase empty, the road most across the axis moves to the other.
fn assign_phases(approaches: &mut [Approach]) {
    let axis = approaches[0].heading;
    let along = |a: &Approach| a.heading.dot(axis).abs();
    for approach in approaches.iter_mut() {
        approach.phase = usize::from(along(approach) < PHASE_ALIGNMENT);
    }
    if approaches.iter().any(|a| a.phase == 1) {
        return;
    }
    let across = approaches.iter().enumerate().min_by(|(_, a), (_, b)| along(a).total_cmp(&along(b))).map(|(i, _)| i);
    if let Some(i) = across {
        approaches[i].phase = 1;
    }
}
