//! The goal drive of a cop car: the command of the goal it holds this step (pit, box-in, rolling block or pull-over, set by
//! `ai/goal_aims.rs`) and the limiter that goal carries from one step to the next. A cop without a ram goal keeps the
//! chase action's velocity request, including while holding a moving formation slot. Spec: ai-pursuit-goals.md.

use blackbox_driver::AiControls;
use blackbox_pursuit::goals::approach::{GoalCommand, Step};
use glam::Vec3;

use super::AiCar;

/// The goal a cop car holds this step: the command the goal gives and the limiter it carries to the next step.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct GoalDrive {
    pub kind: Option<blackbox_pursuit::goals::GoalKind>,
    pub collision_goal: Option<u32>,
    /// The goal's speed limiter (m/s). It carries from step to step, and is 0 with no goal.
    pub limit: f32,
    command: Option<GoalCommand>,
}

impl GoalDrive {
    pub(super) fn hash_into(&self, hash: &mut crate::scenes::world::StateHasher) {
        hash.bytes(format!("{:?}", self.kind).as_bytes());
        hash.bool(self.collision_goal.is_some());
        hash.u32(self.collision_goal.unwrap_or_default());
        hash.f32(self.limit);
        hash.bool(self.command.is_some());
        if let Some(command) = self.command {
            hash.f32(command.desired_speed);
            hash.bool(command.arrived);
            hash.bool(command.target_point.is_some());
            if let Some(point) = command.target_point {
                hash.f32(point.x);
                hash.f32(point.y);
            }
        }
    }
    /// The drive of one step of a goal, or none. With no goal the limiter starts again from 0.
    pub fn from_step(step: Option<Step>) -> Self {
        match step {
            Some(step) => Self { limit: step.limit, command: Some(step.command), collision_goal: None, kind: None },
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

    pub(in crate::scenes::world::ai) fn cop_think_due(&self) -> bool {
        (self.steps + self.stagger).is_multiple_of(super::cop::THINK_STEPS)
    }

    /// Sets the goal of this step from `ai/goal_aims.rs`: its step, or none for a car on the chase.
    pub fn set_goal_drive(&self, step: Option<Step>, kind: Option<blackbox_pursuit::goals::GoalKind>) {
        use blackbox_pursuit::goals::GoalKind;
        let mut drive = GoalDrive::from_step(step);
        drive.kind = kind.filter(|_| step.is_some());
        drive.collision_goal = kind.filter(|_| step.is_some()).map(|k| {
            blackbox_attrib::vlt_hash(match k {
                GoalKind::Pit => "AIGoalPit",
                GoalKind::BoxIn | GoalKind::RollingBlock => "AIGoalRam",
                GoalKind::PullOver => "AIGoalPullOver",
            })
        });
        self.goal.set(drive);
    }

    /// A goal cop that has arrived holds the full brake: brake and handbrake on, no steer and no gas.
    pub(super) fn hold_goal_brake(&mut self) {
        if matches!(&self.role, super::Role::Cop(cop) if !cop.ram_allowed) {
            return;
        }
        if !self.goal_drive().arrived() {
            return;
        }
        self.controls = AiControls { brake: 1.0, handbrake: 1.0, ..AiControls::default() };
    }

    /// Ram goals override the action's speed and steering; ordinary formation slots retain the moving chase request.
    pub(super) fn think_goal(&mut self, position: Vec3, chase_speed: f32, allowed: bool) {
        if !allowed {
            self.arrived = false;
            self.speed = chase_speed;
            return;
        }
        let Some(command) = self.goal_drive().command() else {
            self.arrived = false;
            self.speed = chase_speed;
            return;
        };
        self.arrived = false;
        self.speed = command.desired_speed;
        if let Some(point) = command.target_point {
            self.target = Vec3::new(point.x, position.y, point.y);
        }
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
