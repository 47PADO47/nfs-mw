//! Racers far from the player. The world streams around the player, so a racer that is far ahead or behind has no
//! ground under it. Such a racer is not simulated: the race moves it along its route at the speed it would drive at,
//! and puts its car back on the road when the player comes near. Spec: `docs/specs/ai-racers-route.md` section 6.

use blackbox_race::{Course, Phase, RaceEvent};
use glam::Vec3;

use super::RaceSession;
use super::flow::next_gate;
use crate::scenes::world::WorldScene;

/// A racer farther than this from the player (metres, on the ground) becomes emulated.
const AWAY: f32 = 450.0;
/// An emulated racer nearer than this to the player goes back on the road. Less than [`AWAY`], so it does not flip.
const NEAR: f32 = 300.0;

/// The distance on the ground (physics space: x and z).
fn flat_distance(a: Vec3, b: Vec3) -> f32 {
    (a.x - b.x).hypot(a.z - b.z)
}

/// Moves `from` towards `to` by at most `step` metres.
pub(super) fn advance(from: Vec3, to: Vec3, step: f32) -> Vec3 {
    let offset = to - from;
    let length = offset.length();
    if length <= step || length < 1e-4 {
        return to;
    }
    from + offset / length * step
}

/// Whether the racer should be emulated, from the distance to the player and what it is now. Hysteresis keeps a racer
/// near the limit from flipping every frame.
pub(super) fn wants_emulation(emulated: bool, distance: f32) -> bool {
    match emulated {
        true => distance >= NEAR,
        false => distance > AWAY,
    }
}

impl WorldScene {
    /// Feeds the engine the places of the AI racers this frame: a simulated racer's car, an emulated racer's place on
    /// its route. Returns the events the moves raised.
    pub(super) fn step_racers(&mut self, session: &mut RaceSession, player: Option<Vec3>, dt: f32) -> Vec<RaceEvent> {
        let mut events = Vec::new();
        let (Some(engine), Some(traffic)) = (session.engine.as_mut(), self.traffic.as_mut()) else { return events };
        let racing = engine.phase() == Phase::Racing;
        let cars: Vec<(usize, Vec3, f32)> = traffic.racers().collect();
        for (id, car_at, car_speed) in cars {
            if !engine.racers()[id].racing() {
                continue;
            }
            let Some(player) = player.filter(|_| racing) else {
                events.extend(engine.move_racer(id, car_at));
                continue;
            };
            let at = session.emu[id].unwrap_or(car_at);
            // A person's car is never put on rails: it is always the one being watched.
            let person = session.plan.get(id).is_some_and(|plan| plan.person.is_some());
            let far = !person && wants_emulation(session.emu[id].is_some(), flat_distance(at, player));
            engine.set_speed(id, car_speed);
            if !far {
                if session.emu[id].is_some() {
                    let forward = heading_to(engine.course(), engine.racers()[id].progress.reached, at);
                    if !traffic.relocate_racer(id, at, forward, self.residency.collision(), &self.physics.surfaces) {
                        continue;
                    }
                    log::debug!("race: racer {id} is back on the road at ({:.0}, {:.0})", at.x, at.z);
                    session.emu[id] = None;
                }
                events.extend(engine.move_racer(id, car_at));
                continue;
            }
            let Some(state) = traffic.racer_mut(id) else { continue };
            state.emulated = true;
            let speed = state.emulated_speed();
            engine.set_speed(id, speed);
            let reached = engine.racers()[id].progress.reached;
            let moved = match next_gate(engine.course(), reached) {
                Some(centre) => advance(at, centre, speed * dt),
                None => at,
            };
            session.emu[id] = Some(moved);
            events.extend(engine.move_racer(id, moved));
        }
        events
    }
}

/// The unit direction an emulated racer faces: towards the centre of its next gate.
fn heading_to(course: &Course, reached: usize, at: Vec3) -> Vec3 {
    let centre = next_gate(course, reached).unwrap_or(at + Vec3::Z);
    Vec3::new(centre.x - at.x, 0.0, centre.z - at.z).try_normalize().unwrap_or(Vec3::Z)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_racer_moves_towards_its_aim_and_stops_there() {
        let from = Vec3::ZERO;
        let to = Vec3::new(0.0, 0.0, 100.0);
        assert_eq!(advance(from, to, 30.0), Vec3::new(0.0, 0.0, 30.0));
        assert_eq!(advance(Vec3::new(0.0, 0.0, 90.0), to, 30.0), to, "no overshoot");
        assert_eq!(advance(to, to, 5.0), to);
    }

    #[test]
    fn emulation_has_a_dead_band() {
        assert!(!wants_emulation(false, 449.0));
        assert!(wants_emulation(false, 451.0));
        assert!(wants_emulation(true, 400.0), "still emulated between the two limits");
        assert!(!wants_emulation(true, 299.0), "back on the road when near");
    }

    #[test]
    fn an_emulated_racer_faces_its_gate() {
        let gate = blackbox_race::Gate::new(Vec3::new(0.0, 0.0, 500.0), Vec3::Z, Vec3::Y, [2.0, 40.0, 10.0]);
        let course = Course::new(Vec3::ZERO, vec![gate], None, 1);
        let facing = heading_to(&course, 0, Vec3::new(0.0, 0.0, 100.0));
        assert!((facing - Vec3::Z).length() < 1e-5);
    }
}
