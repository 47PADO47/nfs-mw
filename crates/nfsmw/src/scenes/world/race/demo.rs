//! The demo run (`race start <race> demo`): the player's car is put at each gate in turn, so a whole race can be run, and
//! its result card and career glue seen, without driving the route. For tests and screenshots; no original behaviour.

use blackbox_race::Phase;
use glam::Vec3;

use super::RaceSession;
use super::flow::next_gate;
use crate::scenes::world::road::Spawn;
use crate::scenes::world::{WorldScene, space};

/// Seconds between two jumps.
const JUMP_EVERY: f32 = 1.5;

/// The place the car is put for a gate: its centre, as a render-space spawn facing along the gate.
pub(super) fn spawn_at(centre: Vec3, heading: Vec3) -> Spawn {
    Spawn { position: space::to_render(centre.to_array()), heading: (-heading.x).atan2(heading.z) }
}

impl WorldScene {
    /// Each frame of a demo run: every `JUMP_EVERY` seconds the player's car goes to the next gate.
    pub(super) fn demo_step(&mut self, session: &mut RaceSession, dt: f32) {
        if !session.options.demo {
            return;
        }
        let (Some(engine), Some(drive)) = (session.engine.as_ref(), self.players.lead_mut()) else { return };
        let Some(me) = engine.racers().first().filter(|r| r.racing()) else { return };
        if engine.phase() != Phase::Racing {
            return;
        }
        session.demo_clock += dt;
        if session.demo_clock < JUMP_EVERY {
            return;
        }
        session.demo_clock = 0.0;
        let Some(centre) = next_gate(engine.course(), me.progress.reached) else { return };
        let gate = engine.course().checkpoints.get(me.progress.reached).or(engine.course().finish.as_ref());
        let forward = gate.map_or(Vec3::Z, |g| g.forward);
        drive.place_exactly(spawn_at(centre, forward), 20.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_spawn_is_at_the_gate_and_faces_along_it() {
        // Physics heading +z is render +x: radians 0.
        let spawn = spawn_at(Vec3::new(-100.0, 5.0, 200.0), Vec3::Z);
        assert_eq!(spawn.position, Vec3::new(200.0, 100.0, 5.0));
        assert!(spawn.heading.abs() < 1e-4, "{}", spawn.heading);
        // Physics heading -x is render +y: a quarter turn.
        let side = spawn_at(Vec3::ZERO, Vec3::NEG_X);
        assert!((side.heading - std::f32::consts::FRAC_PI_2).abs() < 1e-4, "{}", side.heading);
    }
}
