//! The wrong-way trigger of a race: a thin zone across the road behind the start that sends the player back to the
//! grid. The zone is a plane trigger with `ResetsPlayer` (`nfsmw_data::gameplay::PlaneTrigger`); the engine raises
//! `RaceEvent::Reset` when a human racer enters it (`libs/blackbox-race`), and the world runs the pause menu's restart.
//! Spec: `docs/specs/race-wrongway.md`.

use blackbox_race::{Course, Gate, RaceEvent};
use glam::Vec3;
use nfsmw_data::gameplay::{GameplayData, PlaneTrigger, world_heading};

/// Metres a plane trigger is thick, along its heading: the game builds it as a box 1 m deep.
const THICKNESS: f32 = 1.0;
/// Metres a plane trigger reaches above and below its position.
const HALF_HEIGHT: f32 = 50.0;

/// What one physics tick of a running race asks of the world.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Tick {
    /// The race goes on.
    Keep,
    /// The race is over and is taken down.
    End,
    /// A player entered a reset zone: the race starts again from its grid.
    Restart,
}

/// What the engine's events of a tick ask: a restart when any racer entered a reset zone, else the race goes on.
pub(super) fn tick_of(events: &[RaceEvent]) -> Tick {
    match events.iter().any(|event| matches!(event, RaceEvent::Reset { .. })) {
        true => Tick::Restart,
        false => Tick::Keep,
    }
}

/// The gate of a reset zone: the plane's own box, `Width` across, a metre deep along its heading and 50 m of height on
/// each side (`docs/specs/race-wrongway.md` section 2). A car is a point to the engine, where the original tests a
/// sphere the size of the car against the box **[unconfirmed]**.
pub(super) fn zone_gate(plane: &PlaneTrigger) -> Gate {
    let sizes = [THICKNESS, plane.width, 2.0 * HALF_HEIGHT];
    Gate::new(plane.position, world_heading(plane.heading_deg), Vec3::Y, sizes)
}

/// Gives `course` the reset zones of the race named `race` (its `wrongway` and `resetplayertrigger` children).
pub(super) fn add_zones(course: &mut Course, data: &GameplayData, race: &str) {
    course.resets = data.reset_zones(race).map(zone_gate).collect();
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plane(name: &str, at: Vec3, heading_deg: f32) -> PlaneTrigger {
        PlaneTrigger { key: 1, name: name.into(), position: at, heading_deg, width: 150.0, resets_player: true }
    }

    #[test]
    fn a_reset_event_asks_for_a_restart_and_nothing_else_does() {
        assert_eq!(tick_of(&[]), Tick::Keep);
        assert_eq!(tick_of(&[RaceEvent::Go, RaceEvent::Checkpoint { racer: 0, number: 1 }]), Tick::Keep);
        assert_eq!(tick_of(&[RaceEvent::Lap { racer: 0, lap: 1 }, RaceEvent::Reset { racer: 1 }]), Tick::Restart);
        assert_eq!(tick_of(&[RaceEvent::Over]), Tick::Keep, "the end is the engine's own business");
    }

    #[test]
    fn the_zone_is_a_slab_across_the_road() {
        // Heading 0 faces +z: 1 m deep along z, 150 m wide along x, 100 m high around the position.
        let gate = zone_gate(&plane("r/wrongway", Vec3::new(10.0, 20.0, 30.0), 0.0));
        assert!(gate.contains(Vec3::new(10.0, 20.0, 30.4)));
        assert!(!gate.contains(Vec3::new(10.0, 20.0, 30.6)), "half a metre either side");
        assert!(gate.contains(Vec3::new(84.0, 20.0, 30.0)) && !gate.contains(Vec3::new(86.0, 20.0, 30.0)));
        assert!(gate.contains(Vec3::new(10.0, 69.0, 30.0)) && !gate.contains(Vec3::new(10.0, 71.0, 30.0)));
    }

    #[test]
    fn the_zones_of_a_race_are_its_own_children_that_reset_the_player() {
        let mut other = plane("race_b/wrongway", Vec3::ZERO, 0.0);
        other.key = 2;
        let mut quiet = plane("race_a/marker", Vec3::ZERO, 0.0);
        quiet.resets_player = false;
        let data = GameplayData {
            planes: vec![plane("race_a/wrongway", Vec3::new(0.0, 0.0, -80.0), 0.0), other, quiet],
            ..Default::default()
        };
        let mut course = Course::new(Vec3::ZERO, Vec::new(), None, 1);
        add_zones(&mut course, &data, "race_a");
        assert_eq!(course.resets.len(), 1);
        assert_eq!(course.resets[0].centre.z, -80.0);
    }
}
