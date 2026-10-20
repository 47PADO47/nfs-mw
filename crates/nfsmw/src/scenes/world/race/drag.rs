//! The drag race: the player's car steers itself down the strip, and the left and right inputs change lane
//! (`docs/specs/race-mode.md` section 8, `docs/specs/ai-driver-control.md` section 6).

use blackbox_race::{Course, Phase};
use glam::Vec3;
use nfsmw_data::gameplay::RaceType;

use super::RaceSession;
use crate::scenes::world::{WorldScene, space};

/// Metres between two lanes of the strip **[unconfirmed]**.
const LANE_WIDTH: f32 = 3.5;
/// How far ahead of the car the assist aims, metres. The original's human drag look-ahead is 8 to 40 m
/// (`docs/specs/ai-driver-speed-skill.md` section 9); the upper end is used so the car does not weave.
const LOOK_AHEAD: f32 = 40.0;
/// A steering input past this changes lane; it must fall back under [`RELEASE`] before it counts again.
const PRESS: f32 = 0.6;
const RELEASE: f32 = 0.3;
/// The outermost lanes either side of the start lane.
const MAX_LANE: i32 = 1;

/// The state of the lane choice: the lane (0 is the start lane, positive to the right) and whether the steering has
/// come back to centre since the last change.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::scenes::world) struct Lane {
    pub index: i32,
    armed: bool,
}

impl Default for Lane {
    fn default() -> Self {
        Self { index: 0, armed: true }
    }
}

impl Lane {
    /// Reads the player's steering: a firm press right or left moves one lane, once per press.
    pub fn update(&mut self, steer: f32) {
        if steer.abs() < RELEASE {
            self.armed = true;
            return;
        }
        if !self.armed || steer.abs() < PRESS {
            return;
        }
        self.armed = false;
        self.index = (self.index + steer.signum() as i32).clamp(-MAX_LANE, MAX_LANE);
    }
}

/// The point the car steers at: `LOOK_AHEAD` metres ahead of the car along the strip (the line from the start to the
/// finish, on the ground), shifted sideways to the lane.
pub(super) fn aim_point(start: Vec3, finish: Vec3, car: Vec3, lane: i32) -> Vec3 {
    let along = Vec3::new(finish.x - start.x, 0.0, finish.z - start.z).try_normalize().unwrap_or(Vec3::Z);
    let right = Vec3::Y.cross(along);
    let travelled = (car - start).dot(along);
    start + along * (travelled + LOOK_AHEAD) + right * (lane as f32 * LANE_WIDTH) + Vec3::Y * (car.y - start.y)
}

/// The end of the strip: the finish line, or the last checkpoint.
fn strip_end(course: &Course) -> Option<Vec3> {
    course.finish.as_ref().map(|g| g.centre).or_else(|| course.checkpoints.last().map(|g| g.centre))
}

impl WorldScene {
    /// Each frame of a drag race, once it has started: the lane and the steering aim.
    pub(super) fn drag_assist(&mut self, session: &mut RaceSession) {
        if session.kind != RaceType::Drag {
            return;
        }
        let (Some(engine), Some(drive)) = (session.engine.as_ref(), self.drive.as_mut()) else { return };
        let racing = engine.phase() == Phase::Racing && engine.racers().first().is_some_and(|r| r.racing());
        if !racing {
            drive.set_steer_assist(None);
            return;
        }
        session.lane.update(drive.steer_request());
        let Some(end) = strip_end(engine.course()) else { return };
        let car = Vec3::from(space::to_physics(drive.position()));
        drive.set_steer_assist(Some(aim_point(engine.course().start, end, car, session.lane.index)));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_firm_press_changes_one_lane_once() {
        let mut lane = Lane::default();
        lane.update(0.9);
        assert_eq!(lane.index, 1);
        lane.update(0.9);
        assert_eq!(lane.index, 1, "held: no second change");
        lane.update(0.0);
        lane.update(-0.9);
        assert_eq!(lane.index, 0);
    }

    #[test]
    fn a_light_touch_does_nothing_and_the_lanes_are_bounded() {
        let mut lane = Lane::default();
        lane.update(0.4);
        assert_eq!(lane.index, 0);
        for _ in 0..4 {
            lane.update(0.0);
            lane.update(1.0);
        }
        assert_eq!(lane.index, MAX_LANE);
    }

    #[test]
    fn the_aim_is_ahead_of_the_car_along_the_strip_in_its_lane() {
        let start = Vec3::new(0.0, 5.0, 0.0);
        let finish = Vec3::new(0.0, 5.0, 400.0);
        let aim = aim_point(start, finish, Vec3::new(1.0, 5.0, 100.0), 0);
        assert_eq!((aim.x, aim.z), (0.0, 100.0 + LOOK_AHEAD));
        let right = aim_point(start, finish, Vec3::new(1.0, 5.0, 100.0), 1);
        assert!((right.x - LANE_WIDTH).abs() < 1e-4, "right of a strip that runs along +z is +x: {right}");
    }
}
