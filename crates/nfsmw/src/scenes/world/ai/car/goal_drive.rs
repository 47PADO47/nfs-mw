//! The goal drive of a cop car: the command of the goal it holds this step (pit, box-in, rolling block or pull-over, set by
//! `ai/goal_aims.rs`) and the limiter that goal carries from one step to the next. A cop without a goal keeps the chase's
//! speed and stops at its slot when it is within `SLOT_ARRIVAL` (unconfirmed). Spec: `docs/specs/ai-pursuit-goals.md`.

use blackbox_driver::AiControls;
use blackbox_pursuit::goals::approach::{GoalCommand, Step};
use glam::{Vec2, Vec3};

use super::AiCar;

/// A cop this close (metres, horizontally) to its formation or collapse slot has arrived and brakes there (unconfirmed).
/// Only the cops without a goal use it.
const SLOT_ARRIVAL: f32 = 2.0;

/// The goal a cop car holds this step: the command the goal gives and the limiter it carries to the next step.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct GoalDrive {
    /// The goal's speed limiter (m/s). It carries from step to step, and is 0 with no goal.
    pub limit: f32,
    command: Option<GoalCommand>,
}

impl GoalDrive {
    /// The drive of one step of a goal, or none. With no goal the limiter starts again from 0.
    pub fn from_step(step: Option<Step>) -> Self {
        match step {
            Some(step) => Self { limit: step.limit, command: Some(step.command) },
            None => Self::default(),
        }
    }

    /// The command of the goal, when the cop holds one.
    pub fn command(&self) -> Option<GoalCommand> {
        self.command
    }

    /// The goal has arrived: the cop holds the full brake.
    pub fn arrived(&self) -> bool {
        self.command.is_some_and(|c| c.arrived)
    }
}

impl AiCar {
    /// The goal this cop holds this step.
    pub fn goal_drive(&self) -> GoalDrive {
        self.goal.get()
    }

    /// Sets the goal of this step from `ai/goal_aims.rs`: its step, or none for a car on the chase.
    pub fn set_goal_drive(&self, step: Option<Step>) {
        self.goal.set(GoalDrive::from_step(step));
    }

    /// A goal cop that has arrived holds the full brake: brake and handbrake on, no steer and no gas.
    pub(super) fn hold_goal_brake(&mut self) {
        if !self.goal_drive().arrived() {
            return;
        }
        self.controls = AiControls { brake: 1.0, handbrake: 1.0, ..AiControls::default() };
    }

    /// The think's speed and arrival. A goal cop takes its speed from its goal and never uses the slot test. The others
    /// stop when they are within `SLOT_ARRIVAL` of their aim, and drive at the chase's speed otherwise.
    pub(super) fn think_goal(&mut self, aim: Option<Vec3>, position: Vec3, chase_speed: f32) {
        let Some(command) = self.goal_drive().command() else {
            self.arrived = aim.is_some_and(|a| {
                let d = a - position;
                Vec2::new(d.x, d.z).length() <= SLOT_ARRIVAL
            });
            self.speed = match self.arrived {
                true => 0.0,
                false => chase_speed,
            };
            return;
        };
        self.arrived = false;
        self.speed = command.desired_speed;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn step(limit: f32, arrived: bool) -> Step {
        Step { command: GoalCommand { target_point: None, desired_speed: 0.0, arrived }, limit }
    }

    #[test]
    fn no_goal_restarts_the_limiter() {
        let drive = GoalDrive::from_step(None);
        assert_eq!(drive.limit, 0.0);
        assert!(drive.command().is_none());
        assert!(!drive.arrived());
    }

    #[test]
    fn a_goal_keeps_its_limiter_and_its_arrival() {
        let drive = GoalDrive::from_step(Some(step(3.5, true)));
        assert_eq!(drive.limit, 3.5);
        assert!(drive.arrived(), "an arrived goal holds the brake");
    }
}
