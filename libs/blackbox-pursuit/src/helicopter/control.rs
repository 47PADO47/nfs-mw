//! The helicopter's controller: from a goal (where to go, how fast, how to face) to a desired velocity. It leads the
//! position by the velocity, brakes early when closing on the goal, and rises or sinks by its height rules. The
//! height floor comes from [`Heights`]; the game gives the heli sheet, the tests a plane. Spec: `ai-helicopter.md` §4.

use glam::{Vec2, Vec3};

/// The lowest height the helicopter may fly at above `x, z`, as a trait so the game can supply the heli sheet.
pub trait Heights {
    /// The floor elevation under `(x, z)`, or `None` where no sheet covers the point (no floor there).
    fn elevation(&self, x: f32, z: f32) -> Option<f32>;
}

/// A flat floor, for tests and for tracks without a sheet.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Flat(pub f32);

impl Heights for Flat {
    fn elevation(&self, _x: f32, _z: f32) -> Option<f32> {
        Some(self.0)
    }
}

/// What the helicopter is asked for this think.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Goal {
    /// The point to fly to.
    pub drive: Vec3,
    /// The speed to fly at horizontally, m/s (a horizontal limit only).
    pub speed: f32,
    /// The velocity of whatever it follows.
    pub dest_velocity: Vec3,
    /// The point to face.
    pub look_at: Vec3,
}

/// The velocity the flight model should reach, and whether the helicopter may brake at its maximum.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Desired {
    pub velocity: Vec3,
    pub max_decel: bool,
}

/// How far ahead along its velocity the helicopter plans, seconds (`LeadPositionTime` of the spec).
const WORK_LEAD: f32 = 0.45;

/// The desired velocity for a goal, from the body's position and velocity.
pub fn desired(position: Vec3, velocity: Vec3, goal: &Goal, heights: &dyn Heights) -> Desired {
    let work = position + velocity * WORK_LEAD;
    let dest_velocity_h = Vec3::new(goal.dest_velocity.x, 0.0, goal.dest_velocity.z);
    let mut dest = goal.drive + dest_velocity_h * WORK_LEAD;
    floor_point(&mut dest, heights);
    // The floor under the lead point also lifts the destination, so the climb comes from the difference in height.
    if let Some(floor) = heights.elevation(work.x, work.z) {
        dest.y = dest.y.max(floor);
    }

    let to = dest - work;
    let len = to.length();
    let mut yd = to.y;
    let mut move_h = Vec3::new(to.x, 0.0, to.z);
    let dir = move_h.normalize_or_zero();
    let speed_h = Vec2::new(velocity.x, velocity.z).length();
    let closing = (velocity - dest_velocity_h).dot(dir);
    let mut vmax = goal.speed;
    let mut max_decel = false;
    if closing > 3.0 {
        let avail = (speed_h * 0.2 + 30.0).min(80.0);
        let used = (closing * 2.0).min(avail);
        let braking = closing * closing / (1.55 * used);
        if len <= braking {
            vmax *= 0.4;
            max_decel = true;
        }
    } else if velocity.dot(to.normalize_or_zero()) < -1.0 && len < 15.0 {
        max_decel = true;
    }
    if len > 2.0 {
        move_h *= vmax / len;
    }
    yd = vertical(yd, goal.dest_velocity.y);
    let velocity = Vec3::new(move_h.x, yd, move_h.z);
    Desired { velocity, max_decel }
}

/// Raises a point to the floor under it, if the heights cover it.
fn floor_point(point: &mut Vec3, heights: &dyn Heights) {
    let Some(floor) = heights.elevation(point.x, point.z) else { return };
    point.y = point.y.max(floor);
}

/// The vertical speed: climb quickly from a little below, sink gently from a little above, and use the target's
/// own vertical speed when it moves the same way.
fn vertical(dy: f32, target_vy: f32) -> f32 {
    if (0.0..7.0).contains(&dy) {
        return dy * 5.0 + if target_vy > 0.0 { 2.0 * target_vy } else { 0.0 };
    }
    if dy < 0.0 && dy > -5.0 {
        return dy * 3.0 + if target_vy < 0.0 { target_vy } else { 0.0 };
    }
    dy
}

#[cfg(test)]
mod tests {
    use super::*;

    fn goal(drive: Vec3) -> Goal {
        Goal { drive, speed: 50.0, dest_velocity: Vec3::ZERO, look_at: drive }
    }

    #[test]
    fn heads_for_the_goal_at_the_speed_asked() {
        let d = desired(Vec3::new(0.0, 50.0, 0.0), Vec3::ZERO, &goal(Vec3::new(200.0, 50.0, 0.0)), &Flat(0.0));
        assert!((d.velocity.x - 50.0).abs() < 1e-3, "{:?}", d.velocity);
        assert!(!d.max_decel);
    }

    #[test]
    fn a_goal_within_two_metres_is_not_chased_at_speed() {
        let here = Vec3::new(0.0, 50.0, 0.0);
        let d = desired(here, Vec3::ZERO, &goal(here + Vec3::X), &Flat(0.0));
        assert!(d.velocity.x.abs() < 1.0 + 1e-3, "{:?}", d.velocity);
    }

    #[test]
    fn climbs_quickly_from_just_below_and_sinks_gently_from_just_above() {
        assert!((vertical(3.0, 0.0) - 15.0).abs() < 1e-5);
        assert!((vertical(-2.0, 0.0) - (-6.0)).abs() < 1e-5);
        assert!((vertical(20.0, 0.0) - 20.0).abs() < 1e-5, "far above is used as is");
        assert!((vertical(-20.0, 0.0) - (-20.0)).abs() < 1e-5, "far below is used as is");
    }

    #[test]
    fn the_target_vertical_speed_adds_when_it_moves_the_same_way() {
        assert!((vertical(3.0, 4.0) - (15.0 + 8.0)).abs() < 1e-5);
        assert!((vertical(-2.0, -4.0) - (-6.0 - 4.0)).abs() < 1e-5);
    }

    #[test]
    fn the_floor_lifts_the_destination_above_the_ground() {
        let d = desired(Vec3::new(0.0, 5.0, 0.0), Vec3::ZERO, &goal(Vec3::new(100.0, 5.0, 0.0)), &Flat(30.0));
        assert!(d.velocity.y > 0.0, "climbs to the 30 m floor: {:?}", d.velocity);
    }

    #[test]
    fn no_floor_where_no_sheet_covers() {
        struct Nothing;
        impl Heights for Nothing {
            fn elevation(&self, _: f32, _: f32) -> Option<f32> {
                None
            }
        }
        let d = desired(Vec3::new(0.0, 5.0, 0.0), Vec3::ZERO, &goal(Vec3::new(100.0, 5.0, 0.0)), &Nothing);
        assert!(d.velocity.y.abs() < 1e-3);
    }

    #[test]
    fn braking_in_time_is_flagged_for_the_hardest_deceleration() {
        // The work point leads the body by 0.45 s of velocity (27 m at 60 m/s), so the goal sits 40 m ahead.
        let fast = Vec3::new(60.0, 0.0, 0.0);
        let d = desired(Vec3::ZERO, fast, &goal(Vec3::new(40.0, 0.0, 0.0)), &Flat(0.0));
        assert!(d.max_decel, "closing fast on a goal 13 m past the work point");
        assert!(d.velocity.x < 60.0);
    }
}
