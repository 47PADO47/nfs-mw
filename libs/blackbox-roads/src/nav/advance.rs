//! Moving a navigator along the graph.
//! Spec: `docs/specs/ai-road-network.md` (§4.1).

use glam::Vec3;

use super::next::Step;
use super::{RandomSource, RoadNav};
use crate::{MIN_CHORD, RoadNetwork};

/// Guards against a pathological graph: segments crossed by one advance.
const MAX_CROSSINGS: usize = 64;

impl RoadNav {
    /// Moves the cursor `distance` metres along the lane (the step is `distance / chord` of the lane
    /// line, so on curves the cursor moves slightly differently from the metres asked). At the end of a
    /// segment the next one is chosen with the cursor's rules; `toward` is the direction a junction exit
    /// should point to (zero for none).
    pub fn advance(&mut self, net: &RoadNetwork, distance: f32, toward: Vec3, rng: &mut impl RandomSource) {
        let mut left = distance;
        for _ in 0..MAX_CROSSINGS {
            if self.dead_end || left <= 0.0 {
                break;
            }
            let chord = self.lane_line().start().distance(self.lane_line().end()).max(MIN_CHORD);
            let t = self.t + left / chord;
            if t <= 1.0 {
                self.t = t;
                break;
            }
            left = (t - 1.0) * chord;
            self.t = 1.0;
            self.evaluate();
            match self.next_segment(net, toward, rng) {
                Step::Stay => self.dead_end = true,
                Step::To { segment, node_ind, lane } => {
                    self.segment = segment;
                    self.node_ind = node_ind;
                    self.lane = lane;
                    self.t = 0.0;
                    self.rebuild(net);
                }
            }
        }
        self.evaluate();
    }
}
