//! The reset key during a race: the car goes back onto the race's route, at the nearest point of the leg the player is
//! on (or of a shortcut of that leg), in a lane, facing along the route, instead of onto the nearest road (which could
//! be a bridge over the route, or a road the route never takes). The race engine is told of the move, so it cannot count
//! a gate. **[ours]**

use blackbox_roads::{
    LaneType, LegPath, NavKind, RoadNav, RoadNetwork, Route, RouteBuilder, RoutePoint, SegmentFilter,
};
use glam::Vec3;

use super::Stage;
use super::route::leg_of;
use crate::scenes::world::road::Spawn;
use crate::scenes::world::{WorldScene, space};

/// Height difference to the route that costs nothing, and the score per metre beyond it (as `Route::locate` scores).
const HEIGHT_SLACK: f32 = 3.0;
const HEIGHT_WEIGHT: f32 = 3.0;

/// Where a car near `near` (physics space) goes back onto `route` on leg `leg`: the nearest point of that leg or of one
/// of its `variants` (the roads of its shortcuts), moved into the drivable lane nearest the car, with the direction of
/// travel there. `None` when the leg has no road.
pub(in crate::scenes::world) fn reset_spot(
    net: &RoadNetwork,
    route: &Route,
    leg: usize,
    variants: &[LegPath],
    near: Vec3,
) -> Option<(Vec3, Vec3)> {
    let span = route.legs.get(leg)?;
    let own = route.locate(net, near, leg..leg + 1).map(|p| (p, route.steps[p.step]));
    let others = variants.iter().filter_map(|path| {
        let mut builder = RouteBuilder::new(net);
        builder.push(span.from, span.to, Some(path));
        let variant = builder.finish();
        variant.locate(net, near, 0..1).map(|p| (p, variant.steps[p.step]))
    });
    let score = |p: &RoutePoint| p.gap + HEIGHT_WEIGHT * (p.rise.abs() - HEIGHT_SLACK).max(0.0);
    let (point, step) = own.into_iter().chain(others).min_by(|(a, _), (b, _)| score(a).total_cmp(&score(b)))?;
    let along = (point.distance - step.start) / net.segment(step.segment).length.max(1e-3) + step.from_t;
    let stored = if step.node_ind == 1 { along } else { 1.0 - along };
    let mut nav = RoadNav::new(NavKind::Direction, LaneType::Racing, SegmentFilter::default());
    nav.init_at_segment(net, step.segment, stored.clamp(0.0, 1.0), near, point.forward, false);
    Some((nav.position, nav.forward))
}

impl WorldScene {
    /// While a race runs, a car waiting to be put back on the road after a reset (or a fall) is put on the race's route
    /// instead (see the module). Exact placements (the grid, a known spot) are left alone.
    pub(in crate::scenes::world) fn reset_onto_race_route(&mut self) {
        let Some(request) = self.players.lead().and_then(|d| d.waiting_for_road()) else { return };
        if request.exact.is_some() {
            return;
        }
        let (Some(session), Some(traffic)) = (self.race.as_mut(), self.traffic.as_ref()) else { return };
        let (Some(route), Some(engine)) = (session.route.as_ref(), session.engine.as_mut()) else { return };
        if !matches!(session.stage, Stage::Running) {
            return;
        }
        let Some(leg) = engine.racers().first().and_then(|player| leg_of(engine.course(), &player.progress)) else {
            return;
        };
        let height = self.players.lead().map_or(0.0, |d| d.position().z);
        let near = Vec3::from(space::to_physics(Vec3::new(request.near[0], request.near[1], height)));
        let variants: Vec<LegPath> =
            session.shortcuts.iter().filter(|s| s.legs.contains(&leg)).map(|s| s.path.clone()).collect();
        let Some((at, forward)) = traffic.race_reset_spot(route, leg, &variants, near) else { return };
        let facing = space::to_render(forward.to_array());
        let spawn = Spawn { position: space::to_render(at.to_array()), heading: facing.y.atan2(facing.x) };
        engine.place_racer(0, at);
        let Some(drive) = self.players.lead_mut() else { return };
        log::info!(
            "race: reset onto the route, leg {leg}, at ({:.0}, {:.0}, {:.1})",
            spawn.position.x,
            spawn.position.y,
            spawn.position.z
        );
        drive.place_exactly(spawn, 0.0);
    }
}

#[cfg(test)]
mod tests {
    use blackbox_race::{Course, Gate, Race, RaceConfig};
    use blackbox_roads::{PathType, SegmentIndex, build_leg};

    use super::*;
    use crate::scenes::world::race::test_roads::{goal, network, route};

    #[test]
    fn the_spot_is_on_the_players_leg_only() {
        let net = network();
        let index = SegmentIndex::build(&net);
        let route = route(&net, &index);
        // Near z = 170, on the second leg: the first leg's nearest point is its end, at its goal (z = 150).
        let (at, _) = reset_spot(&net, &route, 0, &[], Vec3::new(1.0, 0.0, 170.0)).unwrap();
        assert!((at.z - 150.0).abs() < 1.0, "{at:?}");
        let (at, _) = reset_spot(&net, &route, 1, &[], Vec3::new(1.0, 0.0, 170.0)).unwrap();
        assert!((at.z - 170.0).abs() < 1.0, "{at:?}");
        assert!(reset_spot(&net, &route, 7, &[], Vec3::ZERO).is_none());
    }

    #[test]
    fn the_spot_faces_along_the_route_in_a_lane() {
        let net = network();
        let index = SegmentIndex::build(&net);
        let route = route(&net, &index);
        // A car in the side street, facing away: it goes back to the main road, facing +z.
        let (at, forward) = reset_spot(&net, &route, 0, &[], Vec3::new(20.0, 0.0, 95.0)).unwrap();
        assert!(forward.z > 0.99, "{forward:?}");
        assert!(at.x.abs() > 0.5 && at.x.abs() < 8.0, "in a lane, not on the centre line: {at:?}");
    }

    #[test]
    fn a_shortcut_of_the_leg_is_a_place_to_go_back_to() {
        let net = network();
        let index = SegmentIndex::build(&net);
        let route = route(&net, &index);
        // A "shortcut" of the first leg that runs down the side street and back: the nearest point is on it.
        let out = build_leg(
            &net,
            &index,
            goal(5.0),
            blackbox_roads::Goal { position: Vec3::new(80.0, 0.0, 100.0), forward: Vec3::X },
            PathType::Racer,
        )
        .unwrap();
        let near = Vec3::new(60.0, 0.0, 101.0);
        let (plain, _) = reset_spot(&net, &route, 0, &[], near).unwrap();
        let (cut, _) = reset_spot(&net, &route, 0, &[out], near).unwrap();
        assert!(plain.x.abs() < 8.0 && cut.x > 50.0, "{plain:?} {cut:?}");
    }

    #[test]
    fn a_reset_past_a_gate_counts_no_gate() {
        let gate = Gate::new(Vec3::new(1.0, 0.0, 150.0), Vec3::Z, Vec3::Y, [2.0, 20.0, 10.0]);
        let course = Course::new(Vec3::ZERO, vec![gate], None, 1);
        let mut race = Race::new(course, RaceConfig::default(), &[(true, Vec3::new(1.0, 0.0, 140.0))]);
        race.start_now();
        race.place_racer(0, Vec3::new(1.0, 0.0, 170.0));
        assert!(race.move_racer(0, Vec3::new(1.0, 0.0, 171.0)).is_empty());
        assert_eq!(race.racers()[0].progress.reached, 0);
    }
}
