//! A race's route on the road graph: the road from the grid through every gate of every lap, built once when the race
//! starts. The race's barrier lines close the roads they cross while it is searched, and each leg falls back to a
//! looser search when the strict one finds nothing (`blackbox_roads::build_route`). A leg counts only when its centre
//! line passes through its gate. Spec: `docs/specs/ai-racers-route.md` section 2.

use std::sync::Arc;

use blackbox_race::Gate;
use blackbox_roads::{
    ActiveBarrier, Goal, LegPath, LegSource, PathType, Route, RouteBuilder, SegmentFilter, build_leg, build_route,
    closest_segment,
};

use super::TrafficWorld;

impl TrafficWorld {
    /// The route from `start` through `gates` in order (every lap's), with `lines` the race's barrier lines. The road at
    /// the start keeps no barrier flag, as in free roam. The network's barrier flags are clear afterwards.
    pub fn build_race_route(&mut self, start: Goal, gates: &[Gate], lines: &[ActiveBarrier]) -> Route {
        let goals: Vec<Goal> = gates.iter().map(|g| Goal { position: g.centre, forward: g.forward }).collect();
        let exempt: Vec<u16> =
            closest_segment(&self.network, &self.index, start.position, start.forward, 1.0, SegmentFilter::default())
                .and_then(|located| self.network.segment(located.segment).road)
                .into_iter()
                .collect();
        let through_gate = |leg: usize, points: &[glam::Vec3]| passes(&gates[leg], points);
        let (route, sources) =
            build_route(&mut self.network, &self.index, start, &goals, (lines, &exempt), through_gate);
        let count = |source: LegSource| sources.iter().filter(|&&s| s == source).count();
        log::info!(
            "race route: {} leg(s), {:.0} m: {} closed, {} with the ends opened, {} open, {} missing",
            sources.len(),
            route.length,
            count(LegSource::Closed),
            count(LegSource::OpenEnds),
            count(LegSource::Open),
            count(LegSource::Missing)
        );
        route
    }

    /// The road of a shortcut through `via`: the leg of `route` it lies beside (the one it lengthens least), driven
    /// through `via` on the way. `None` when there is no such road or it is more than [`MAX_STRETCH`] times the leg.
    pub fn route_through(&self, route: &Route, via: Goal) -> Option<(usize, LegPath)> {
        let detour = |leg: &blackbox_roads::Leg| {
            leg.from.position.distance(via.position) + via.position.distance(leg.to.position) - leg.length
        };
        let (index, leg) = route
            .legs
            .iter()
            .enumerate()
            .filter(|(_, leg)| !leg.missing())
            .min_by(|(_, a), (_, b)| detour(a).total_cmp(&detour(b)))?;
        let (net, roads) = (&self.network, &self.index);
        let first = build_leg(net, roads, leg.from, via, PathType::RaceRoute)?;
        let second = build_leg(net, roads, via, leg.to, PathType::RaceRoute)?;
        let path = first.join(&second);
        let mut builder = RouteBuilder::new(net);
        builder.push(leg.from, leg.to, Some(&path));
        (builder.finish().length <= leg.length * MAX_STRETCH).then_some((index, path))
    }

    /// `route` with the legs `replaced` names driven along their new road.
    pub fn splice_route(&self, route: &Route, replaced: &[(usize, LegPath)]) -> Route {
        route.spliced(&self.network, replaced)
    }

    /// The racer `id` follows `route` from now on (`ai/racer.rs`).
    pub fn set_racer_route(&mut self, id: usize, route: Arc<Route>) {
        if let Some(racer) = self.racer_mut(id) {
            racer.route = Some(route);
        }
    }
}

/// How much longer than its leg a shortcut's road may be.
const MAX_STRETCH: f32 = 1.25;

/// Whether the polyline `points` passes through `gate`.
pub(in crate::scenes::world) fn passes(gate: &Gate, points: &[glam::Vec3]) -> bool {
    points.windows(2).any(|pair| gate.touches(pair[0], pair[1]))
}

#[cfg(test)]
mod tests {
    use glam::Vec3;

    use super::*;

    #[test]
    fn a_line_through_the_gate_passes_and_one_beside_it_does_not() {
        let gate = Gate::new(Vec3::new(0.0, 0.0, 100.0), Vec3::Z, Vec3::Y, [2.0, 20.0, 10.0]);
        let through: Vec<Vec3> = (0..60).map(|i| Vec3::new(1.0, 0.5, i as f32 * 2.0)).collect();
        assert!(passes(&gate, &through));
        let beside: Vec<Vec3> = through.iter().map(|p| *p + Vec3::X * 30.0).collect();
        assert!(!passes(&gate, &beside));
        assert!(!passes(&gate, &through[..40]), "a line that stops short of the gate");
    }
}
