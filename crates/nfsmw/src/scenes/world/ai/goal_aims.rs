//! The cop goals of a chase as the world drives them. The formation's finisher and the collapse say which cops take a goal
//! and which slot each one drives to; the goal rules in `blackbox_pursuit::goals` give the aim, the desired speed and the
//! arrival. This file reads the world's values (the player's motion, the cop's car, the heat's tuning) into those rules.
//! Spec: `docs/specs/ai-pursuit-goals.md`. The per-cop state (the limiter and the command) is in `car/goal_drive.rs`.

use blackbox_pursuit::formation::Formation;
use blackbox_pursuit::goals::approach::{AiVehicle, CopState, Step, TargetState, aim_for, ram};
use blackbox_pursuit::goals::{GoalKind, Record, Tuning, box_in, pit, pull_over, rolling_block};
use blackbox_roads::Body;
use blackbox_vehicle::performance::Performance;
use glam::Vec2;
use nfsmw_data::pursuit::AiVehicle as CopAiVehicle;

/// The collapse aggression of each heat level (`CollapseAggression`, goals spec 7.1 and formation spec 5.1). Heats 8 to 10
/// have none.
const COLLAPSE_AGGRESSION: [f32; 10] = [0.4, 0.5, 0.6, 0.7, 0.8, 0.8, 0.5, 0.0, 0.0, 0.0];

/// A goal a cop takes this step, with the slot it drives to.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum Goal {
    /// A finisher goal: the cop fills `record` of the goal's table, and drives to its in-position point.
    Finisher { kind: GoalKind, record: Record },
    /// The pull-over: the collapse ring slot the cop drives to, in the map plane.
    Ring(Vec2),
}

impl Goal {
    pub(super) fn kind(&self) -> GoalKind {
        match self {
            Self::Finisher { kind, .. } => *kind,
            Self::Ring(_) => GoalKind::PullOver,
        }
    }
}

/// Which goal a formation gives its cops when its finisher runs: the pit, box-in or rolling block. The formations without
/// a finisher give none.
pub(super) fn goal_for_formation(formation: Formation) -> Option<GoalKind> {
    match formation {
        Formation::Pit => Some(GoalKind::Pit),
        Formation::BoxIn => Some(GoalKind::BoxIn),
        Formation::RollingBlock => Some(GoalKind::RollingBlock),
        Formation::Follow | Formation::Herd | Formation::StaggerFollow => None,
    }
}

/// The finisher goal of the cop with `slot` (the index in the formation's slot table, which matches the goal's record
/// table). `None` when the goal has no such record.
pub(super) fn finisher_goal(kind: GoalKind, slot: usize, tuning: &Tuning) -> Option<Goal> {
    let record = match kind {
        GoalKind::Pit => pit::records().get(slot).copied(),
        GoalKind::BoxIn => box_in::records(tuning.boxin_tightness).get(slot).copied(),
        GoalKind::RollingBlock => rolling_block::records(tuning.rolling_block_tightness).get(slot).copied(),
        GoalKind::PullOver => None,
    }?;
    Some(Goal::Finisher { kind, record })
}

/// Install-verified free-roam goal tuning, goals spec §5.4 and heat spec §2.7.
pub(super) fn tuning_for(level: u32) -> Tuning {
    let index = (level as usize).clamp(1, 10) - 1;
    let tightness = [0.5, 0.6, 0.8, 0.9, 1.0, 1.0, 1.0, 0.5, 0.5, 0.5][index];
    let duration = [4.0, 5.0, 6.0, 7.0, 8.0, 10.0, 10.0, 3.0, 3.0, 3.0][index];
    Tuning {
        collapse_aggression: COLLAPSE_AGGRESSION[index],
        boxin_tightness: tightness,
        rolling_block_tightness: tightness,
        boxin_duration: duration,
        rolling_block_duration: duration,
    }
}

/// The player's actual performance model at the cop's current speed.
pub(super) fn target_state(player: &Body, performance: &Performance, cop_speed: f32) -> TargetState {
    TargetState {
        position: Vec2::new(player.position.x, player.position.z),
        velocity: Vec2::new(player.velocity.x, player.velocity.z),
        heading: player.forward,
        top_speed: performance.top_speed,
        accel: performance.acceleration_at(cop_speed.abs()),
        jerk: false,
    }
}

/// The cop's own limits in the goal rules' form.
pub(super) fn vehicle(ai: CopAiVehicle) -> AiVehicle {
    AiVehicle {
        max_speed_kmh: ai.max_speed_kmh,
        top_speed_mult: ai.top_speed_multiplier,
        accel_mult: ai.acceleration_multiplier,
    }
}

/// One step of a goal for a cop: the aim point it steers at (map plane, metres) and the goal's step (desired speed,
/// arrival and the limiter to carry).
pub(super) fn step_goal(
    goal: Goal,
    cop: &CopState,
    target: &TargetState,
    vehicle: AiVehicle,
    tuning: &Tuning,
    separation: Vec2,
) -> (Vec2, Step) {
    match goal {
        Goal::Finisher { record, .. } => {
            let aim = aim_for(target, record.in_position);
            (aim, ram(cop, target, aim, vehicle))
        }
        Goal::Ring(slot) => {
            let aggression = pull_over::aggression(tuning.collapse_aggression, target.jerk);
            let aim = pull_over::detour(cop.position, slot, target.position).unwrap_or(slot);
            (aim, pull_over::think(cop, target, slot, aggression, separation, vehicle))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use blackbox_pursuit::goals::approach::KMH;

    /// A still player at the origin facing +Y in the map plane.
    fn still_player() -> TargetState {
        TargetState {
            position: Vec2::ZERO,
            velocity: Vec2::ZERO,
            heading: Vec2::Y,
            top_speed: 40.0,
            accel: 4.0,
            jerk: false,
        }
    }

    fn cop_vehicle() -> AiVehicle {
        AiVehicle { max_speed_kmh: 360.0, top_speed_mult: 1.0, accel_mult: 1.0 }
    }

    #[test]
    fn the_pit_box_in_and_rolling_block_formations_give_their_goal() {
        assert_eq!(goal_for_formation(Formation::Pit), Some(GoalKind::Pit));
        assert_eq!(goal_for_formation(Formation::BoxIn), Some(GoalKind::BoxIn));
        assert_eq!(goal_for_formation(Formation::RollingBlock), Some(GoalKind::RollingBlock));
    }

    #[test]
    fn the_formations_without_a_finisher_give_no_goal() {
        for formation in [Formation::Follow, Formation::Herd, Formation::StaggerFollow] {
            assert_eq!(goal_for_formation(formation), None, "{formation:?}");
        }
    }

    #[test]
    fn the_collapse_aggression_follows_the_heat_level() {
        assert_eq!(tuning_for(1).collapse_aggression, 0.4);
        assert_eq!(tuning_for(5).collapse_aggression, 0.8);
        assert_eq!(tuning_for(8).collapse_aggression, 0.0, "no collapse aggression above heat 7");
    }

    #[test]
    fn a_cop_on_a_pit_goal_aims_behind_the_rear_quarter_across_the_target() {
        // Cop 0 takes the pit's first slot: 4 m beside the target's right rear quarter. Its aim is the in-position point
        // 10 m out on the target's left and 2.7 m behind the target, behind the player's rear quarter.
        let tuning = tuning_for(1);
        let goal = finisher_goal(GoalKind::Pit, 0, &tuning).expect("the pit has a slot 0");
        let cop = CopState { position: Vec2::new(4.0, -20.0), forward_speed: 0.0, limit: 0.0, dt: 1.0 / 60.0 };
        let (aim, step) = step_goal(goal, &cop, &still_player(), cop_vehicle(), &tuning, Vec2::ZERO);
        assert!((aim - Vec2::new(-10.0, -2.7)).length() < 1e-4, "aim {aim:?}");
        assert!(aim.y < 0.0, "behind the target");
        assert!(!step.command.arrived, "a finisher goal does not arrive by itself");
        assert!(step.command.desired_speed > 0.0, "the cop drives");
    }

    #[test]
    fn a_pull_over_cop_on_its_ring_slot_arrives_and_stops() {
        // The slot is 5 m ahead of a still target and the cop is on it: the seek is empty, so it has arrived.
        let tuning = tuning_for(1);
        let slot = Vec2::new(0.0, 5.0);
        let cop = CopState { position: slot, forward_speed: 0.0, limit: 0.0, dt: 1.0 / 60.0 };
        let (_, step) = step_goal(Goal::Ring(slot), &cop, &still_player(), cop_vehicle(), &tuning, Vec2::ZERO);
        assert!(step.command.arrived, "{:?}", step.command);
        assert_eq!(step.command.desired_speed, 0.0);
        assert!(step.command.target_point.is_none(), "an arrived pull-over sets no drive target");
    }

    #[test]
    fn the_pull_over_arrival_threshold_is_the_spec_speed() {
        // Heat 1 (aggression 0.4): (0.2 * 0.6 + 0.5) km/h.
        let threshold = pull_over::arrival_threshold(0.4);
        assert!((threshold - 0.62 * KMH).abs() < 1e-4, "{threshold}");
    }

    #[test]
    fn the_goal_kind_of_a_ring_goal_is_the_pull_over() {
        assert_eq!(Goal::Ring(Vec2::ZERO).kind(), GoalKind::PullOver);
    }
}
