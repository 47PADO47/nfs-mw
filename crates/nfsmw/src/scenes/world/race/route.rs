//! The race's route for the opponents: the gates of every lap in the order a racer meets them, which leg a racer is on,
//! and building the route when the race starts (`ai/race_route.rs`). Spec: `docs/specs/ai-racers-route.md` section 2.

use std::sync::Arc;

use blackbox_race::{Course, Gate, Progress};
use blackbox_roads::{Goal, Route};
use glam::Vec3;
use nfsmw_data::gameplay::RaceType;

use crate::scenes::world::road::Spawn;
use crate::scenes::world::{WorldScene, space};

/// A racer that moves farther than this in one tick (metres) was put somewhere (a reset, a teleport): the move counts
/// no gate (`blackbox_race::RaceConfig::max_move`).
pub(in crate::scenes::world) const MAX_MOVE: f32 = 60.0;

/// The gates in the order a racer crosses them: each lap's checkpoints, then its finish line.
pub(in crate::scenes::world) fn gates_of(course: &Course) -> Vec<Gate> {
    let lap = course.checkpoints.iter().chain(course.finish.iter()).copied();
    (0..course.laps.max(1)).flat_map(|_| lap.clone()).collect()
}

/// The leg a racer with `progress` is on: the index in [`gates_of`] of the gate it drives to. `None` once it finished.
pub(in crate::scenes::world) fn leg_of(course: &Course, progress: &Progress) -> Option<usize> {
    if progress.finished {
        return None;
    }
    let per_lap = course.checkpoints.len() + usize::from(course.finish.is_some());
    Some(progress.laps as usize * per_lap + progress.reached)
}

impl WorldScene {
    /// Builds the route of a race whose player starts at `spawn` (render space), with the race's walls up. `None`
    /// without a road network or a gate.
    pub(super) fn build_race_route(&mut self, course: &Course, spawn: Spawn) -> Option<Arc<Route>> {
        let traffic = self.traffic.as_mut()?;
        let gates = gates_of(course);
        if gates.is_empty() {
            return None;
        }
        let lines = self.scenery_groups.active_barriers(traffic.track_barriers());
        let heading = Vec3::new(spawn.heading.cos(), spawn.heading.sin(), 0.0);
        let start = Goal { position: course.start, forward: Vec3::from(space::to_physics(heading)) };
        Some(Arc::new(traffic.build_race_route(start, &gates, &lines)))
    }

    /// One lap of the race's `route` for the Custom map, closed for a lap race (circuit, knockout). `None` without a
    /// road network or a route to draw. Built once, when the race starts.
    pub(super) fn map_race_route(
        &self,
        route: &Route,
        course: &Course,
        kind: RaceType,
    ) -> Option<crate::hud::RaceRouteHandle> {
        let (net, _) = self.traffic.as_ref()?.road_graph();
        let per_lap = course.checkpoints.len() + usize::from(course.finish.is_some());
        let closed = matches!(kind, RaceType::Circuit | RaceType::Knockout);
        let lap = crate::hud::RaceRoute::build(net, route, (per_lap, closed), |p| {
            let at = space::to_render(p.to_array());
            [at.x, at.y]
        })?;
        Some(crate::hud::RaceRouteHandle::new(lap))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gate(z: f32) -> Gate {
        Gate::new(Vec3::new(0.0, 0.0, z), Vec3::Z, Vec3::Y, [2.0, 40.0, 10.0])
    }

    #[test]
    fn every_lap_lists_its_checkpoints_then_its_finish() {
        let course = Course::new(Vec3::ZERO, vec![gate(10.0), gate(20.0)], Some(gate(30.0)), 2);
        let order: Vec<f32> = gates_of(&course).iter().map(|g| g.centre.z).collect();
        assert_eq!(order, vec![10.0, 20.0, 30.0, 10.0, 20.0, 30.0]);
        let open = Course::new(Vec3::ZERO, vec![gate(10.0)], None, 3);
        assert_eq!(gates_of(&open).len(), 3, "without a finish the last checkpoint closes the lap");
    }

    #[test]
    fn the_leg_counts_the_laps_and_the_gates_reached() {
        let course = Course::new(Vec3::ZERO, vec![gate(10.0), gate(20.0)], Some(gate(30.0)), 2);
        let at = |laps: u32, reached: usize| Progress { laps, reached, finished: false };
        assert_eq!(leg_of(&course, &at(0, 0)), Some(0));
        assert_eq!(leg_of(&course, &at(0, 2)), Some(2), "on the way to the finish line");
        assert_eq!(leg_of(&course, &at(1, 1)), Some(4));
        assert_eq!(leg_of(&course, &Progress { finished: true, ..at(1, 0) }), None);
    }
}
