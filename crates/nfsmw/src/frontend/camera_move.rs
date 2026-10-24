//! The front end's camera mover: the move from one pose to the next (spec: `docs/specs/menu-backdrop-looks.md`
//! section 4.4).
//!
//! The original's mover (state 0 of its update) blends every value of the pose with one curve: with `t` the time over the
//! move's duration, `s = 1 - e^(-k t^2) * cos((n + 0.5) pi t^2)`, `k` the decay and `n` the order. It starts at 0, swings
//! past 1 `n` times and is exactly 1 at `t = 1`. Pitch, yaw, distance, roll, field of view and the target all follow it.
//! The goal's angles are first moved by whole turns to the copy nearest the start, so a move never takes the long way
//! round.

use nfsmw_data::frontend_camera::CameraAngle;

use super::camera_pose::CameraPose;

/// The pose of the camera over a move from a start pose to a goal pose.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CameraMove {
    start: CameraPose,
    goal: CameraPose,
    decay: f32,
    order: i32,
    /// Seconds the move takes; zero or less is already there.
    duration: f32,
    /// Seconds since the move began.
    time: f32,
}

impl CameraMove {
    /// The move that starts on the pose `start` and runs onto `goal`, taking the goal's duration, decay and order (the
    /// original sets the goal's values when it starts the move).
    pub fn between(start: &CameraAngle, goal: &CameraAngle) -> Self {
        Self::from_pose(CameraPose::from_angle(start), goal)
    }

    /// The move that starts on `start` and runs onto `goal`; the goal's angles are moved to the turn nearest the start.
    pub(super) fn from_pose(start: CameraPose, goal: &CameraAngle) -> Self {
        let mut goal_pose = CameraPose::from_angle(goal);
        goal_pose.pitch = nearest_turn(start.pitch, goal_pose.pitch);
        goal_pose.yaw = nearest_turn(start.yaw, goal_pose.yaw);
        goal_pose.roll = nearest_turn(start.roll, goal_pose.roll);
        Self { start, goal: goal_pose, decay: goal.decay, order: goal.order, duration: goal.duration, time: 0.0 }
    }

    /// Starts a new move from where the camera is now onto `goal`.
    pub fn retarget(&mut self, goal: &CameraAngle) {
        *self = Self::from_pose(self.pose(), goal);
    }

    /// Lets `dt` seconds go by.
    pub fn advance(&mut self, dt: f32) {
        self.time += dt;
    }

    pub(super) fn finished(&self) -> bool {
        self.duration <= 0.0 || self.time >= self.duration
    }

    /// Where the camera is now.
    pub fn pose(&self) -> CameraPose {
        let t = match self.duration > 0.0 && self.time < self.duration {
            true => self.time / self.duration,
            false => 1.0,
        };
        // The move ends on the goal itself; the curve is exactly 1 there, so this only spares the rounding.
        let s = match t < 1.0 {
            true => self.curve(t),
            false => 1.0,
        };
        let (a, b) = (self.start, self.goal);
        let mix = |from: f32, to: f32| s * to + (1.0 - s) * from;
        CameraPose {
            pitch: mix(a.pitch, b.pitch),
            yaw: wrap_turn(mix(a.yaw, b.yaw)),
            distance: mix(a.distance, b.distance),
            roll: wrap_turn(mix(a.roll, b.roll)),
            fov: mix(a.fov, b.fov),
            target: a.target + (b.target - a.target) * s,
        }
    }

    /// `1 - e^(-k t^2) cos((n + 0.5) pi t^2)`.
    fn curve(&self, t: f32) -> f32 {
        let t2 = f64::from(t) * f64::from(t);
        let swing = ((f64::from(self.order) + 0.5) * t2 * std::f64::consts::PI).cos();
        (1.0 - (-t2 * f64::from(self.decay)).exp() * swing) as f32
    }
}

/// `goal` moved by a whole turn when that puts it nearer `start`.
fn nearest_turn(start: f32, goal: f32) -> f32 {
    let (up, down) = ((start - (goal + 360.0)).abs(), (start - (goal - 360.0)).abs());
    let direct = (start - goal).abs();
    if up < direct && up < down {
        return goal + 360.0;
    }
    if down < direct && down < up {
        return goal - 360.0;
    }
    goal
}

/// An angle brought into 0..=360 by one turn, as the mover does after each step.
fn wrap_turn(angle: f32) -> f32 {
    if angle > 360.0 {
        return angle - 360.0;
    }
    if angle < 0.0 {
        return angle + 360.0;
    }
    angle
}

#[cfg(test)]
#[path = "camera_move_tests.rs"]
mod tests;
