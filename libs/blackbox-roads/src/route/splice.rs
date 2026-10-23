//! Changing the road of some legs of a route: a racer that takes a shortcut drives the same route with one leg replaced.

use super::{LegPath, Route, RouteBuilder};
use crate::RoadNetwork;

impl LegPath {
    /// This road followed by `next`, which starts where this one ends (on the same segment, at its goal).
    pub fn join(&self, next: &LegPath) -> LegPath {
        let mut segments = self.segments.clone();
        let rest = match next.segments.first() == segments.last() {
            true => &next.segments[1..],
            false => &next.segments[..],
        };
        segments.extend_from_slice(rest);
        LegPath { segments, from_t: self.from_t, goal: next.goal }
    }
}

impl Route {
    /// The road of leg `leg` as it was pushed; `None` for a missing leg.
    pub fn leg_path(&self, leg: usize) -> Option<LegPath> {
        let leg = self.legs.get(leg)?;
        let goal = leg.goal?;
        let steps = &self.steps[leg.steps.clone()];
        let mut segments: Vec<u16> = steps.iter().map(|s| s.segment).collect();
        segments.dedup();
        let first = steps.first()?;
        let from_t = if first.node_ind == 1 { first.from_t } else { 1.0 - first.from_t };
        Some(LegPath { segments, from_t, goal })
    }

    /// The same route with the legs `replaced` names driven along their new road.
    pub fn spliced(&self, net: &RoadNetwork, replaced: &[(usize, LegPath)]) -> Route {
        let mut builder = RouteBuilder::new(net);
        for (index, leg) in self.legs.iter().enumerate() {
            let path =
                replaced.iter().find(|(l, _)| *l == index).map(|(_, p)| p.clone()).or_else(|| self.leg_path(index));
            builder.push(leg.from, leg.to, path.as_ref());
        }
        builder.finish()
    }
}
