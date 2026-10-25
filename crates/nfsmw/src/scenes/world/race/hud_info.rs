//! The race as the Custom HUD reads it (`hud/state/race_info.rs`): the type, the gate the player drives to, the route
//! left and the gap to the nearest rival, from the race engine and the race's route.

use blackbox_race::{Course, Race};

use super::flow::next_gate;
use super::route::leg_of;
use super::{RaceSession, Stage};
use crate::hud::{RaceInfo, RivalGap, checkpoint_gap, rival_of, shows_rival_gap};
use crate::scenes::world::{WorldScene, space};

impl WorldScene {
    /// The race the player runs, shown from the countdown to the race-over message as the package's readout is
    /// (`card.rs`). `None` outside a race.
    pub(in crate::scenes::world) fn race_info(&self) -> Option<RaceInfo> {
        let session = self.race.as_ref()?;
        if matches!(session.stage, Stage::Intro { .. }) {
            return None;
        }
        if session.result.is_some() && !matches!(session.stage, Stage::Results { .. }) {
            return None;
        }
        let engine = session.engine.as_ref()?;
        let course = engine.course();
        let me = engine.racers().first()?;
        let done = me.done();
        let leg = leg_of(course, &me.progress).filter(|_| !done);
        Some(RaceInfo {
            kind: session.kind,
            next_gate: next_gate(course, me.progress.reached)
                .filter(|_| !done)
                .map(|centre| space::to_render(centre.to_array()).to_array()),
            gate: leg.map(|leg| leg as u32 + 1),
            gates: gate_count(course),
            remaining_m: leg.and_then(|leg| self.route_left(session, leg, me.position)),
            rival_gap: rival_gap(session, engine),
        })
    }

    /// Metres of the race's route left from the player's place on leg `leg` (`position` in physics space). `None`
    /// without a route or a road network, or when the leg has no road.
    fn route_left(&self, session: &RaceSession, leg: usize, position: glam::Vec3) -> Option<f32> {
        let route = session.route.as_ref()?;
        let (net, _) = self.traffic.as_ref()?.road_graph();
        let point = route.locate(net, position, leg..leg + 1)?;
        Some((route.length - point.distance).max(0.0))
    }
}

/// The gates of the whole race: each lap's checkpoints and its finish line (the count of `route::gates_of`).
fn gate_count(course: &Course) -> u32 {
    let per_lap = course.checkpoints.len() + usize::from(course.finish.is_some());
    (per_lap * course.laps.max(1) as usize) as u32
}

/// The gap between the player (racer 0) and the nearest rival at the latest checkpoint both passed. `None` for a race
/// type that shows none, without a rival, and before the first shared checkpoint (`hud/state/race_info.rs`).
fn rival_gap(session: &RaceSession, engine: &Race) -> Option<RivalGap> {
    if !shows_rival_gap(session.kind) || engine.config().rank_by_points {
        return None;
    }
    let racers = engine.racers();
    let out: Vec<bool> = racers.iter().map(|racer| racer.out_at.is_some()).collect();
    let rival = rival_of(&engine.places(), 0, &out)?;
    let (seconds, ahead) = checkpoint_gap(&racers.first()?.checkpoint_times, &racers.get(rival)?.checkpoint_times)?;
    let name = session.plan.get(rival).map_or_else(String::new, |plan| plan.name.clone());
    Some(RivalGap { seconds, ahead, name })
}

#[cfg(test)]
mod tests {
    use super::*;
    use blackbox_race::Gate;
    use glam::Vec3;

    fn gate(z: f32) -> Gate {
        Gate::new(Vec3::new(0.0, 0.0, z), Vec3::Z, Vec3::Y, [2.0, 40.0, 10.0])
    }

    #[test]
    fn the_gate_count_matches_the_gates_a_racer_crosses() {
        let course = Course::new(Vec3::ZERO, vec![gate(10.0), gate(20.0)], Some(gate(30.0)), 2);
        assert_eq!(gate_count(&course), super::super::route::gates_of(&course).len() as u32);
        let open = Course::new(Vec3::ZERO, vec![gate(10.0)], None, 3);
        assert_eq!(gate_count(&open), 3);
    }
}
