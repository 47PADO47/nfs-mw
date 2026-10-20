//! The helicopter's controller: from a goal (where to go, how fast, how to face) to a desired velocity. It leads the
//! position by the velocity, brakes early when closing on the goal, and rises or sinks by its height rules. The height
//! sheet floors the destination and steers the move around rising surfaces (`CheckHeliSheet`). The game gives the heli
//! sheet, the tests a plane. Spec: `ai-helicopter.md` §4 and §4.2.
//!
//! Evidence tags on the constants: [decomp] the decompiled rule; [unconfirmed] a value or reading the spec does not
//! confirm. A pool address is the float word in `speed.exe` (PC v1.3) that holds the value; "cluster-only" means the word
//! sits near the heli constants but no use site was found.

use glam::{Vec2, Vec3};

use super::camera::{self, Camera};
use super::gains::Gains;

/// The lowest height the helicopter may fly at above `x, z`, as a trait so the game can supply the heli sheet.
pub trait Heights {
    /// The floor elevation under `(x, z)`, or `None` where no sheet covers the point (no floor there).
    fn elevation(&self, x: f32, z: f32) -> Option<f32>;

    /// The floor under `(x, z)` with the normal of the surface that covers it. A source without normals keeps this
    /// default: a zero normal, which gives the steering no side push (see [`side_push`]).
    fn floor(&self, x: f32, z: f32) -> Option<Floor> {
        self.elevation(x, z).map(|height| Floor { height, normal: Vec3::ZERO })
    }
}

/// A floor point: its height and the normal of the surface under it (zero when the source does not give one).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Floor {
    pub height: f32,
    pub normal: Vec3,
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

/// The velocity the flight model should reach, whether the helicopter may brake at its maximum, and a velocity the body
/// takes before the flight step (the lead-point rule's hard steer), if any.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Desired {
    pub velocity: Vec3,
    pub max_decel: bool,
    pub body_velocity: Option<Vec3>,
}

/// Seconds of velocity the work point (where the helicopter will be) and the destination lead by (`LeadPositionTime`).
/// [decomp] 0.45, pool.
const WORK_LEAD: f32 = 0.45;
/// Seconds of velocity the look-ahead point leads by (`FarLeadPositionTime`). [decomp] 0.65, pool (cluster-only).
const AHEAD_LEAD: f32 = 0.65;
/// Side-push gain of the rising-roof rule (the look-ahead point). [decomp] 0.4, pool (also;
/// [unconfirmed] which of the two words is this gain.
const SHEET_STEER_AHEAD: f32 = 0.4;
/// Side-push gain of the lead-point rule. [decomp] 0.3, pool (cluster-only); [unconfirmed] its use.
const SHEET_STEER_LEAD: f32 = 0.3;
/// Closing speed (m/s) above which the approach may brake hard. [decomp] 3, pool and.
const CLOSING_BRAKE: f32 = 3.0;
/// Horizontal distance (m) under which a body moving against the goal brakes at its maximum. [decomp] 15, pool.
const REVERSE_BRAKE_RANGE: f32 = 15.0;
/// Horizontal move (m) under which the speed is not applied to the move. [decomp] 2 (spec pseudocode; no pool word found).
const SPEED_SCALE_MIN: f32 = 2.0;
/// Steering shorter than this (per unit) is ignored. [decomp] 0.1, pool.
const STEER_MIN: f32 = 0.1;
/// Share of a steering direction blended into the move. [decomp] 0.3 (spec pseudocode; no pool word found).
const STEER_BLEND: f32 = 0.3;
/// Climb band: a height error from 0 up to this (m) climbs at `CLIMB_GAIN`. [decomp] 7, pool.
const CLIMB_BAND: f32 = 7.0;
/// Climb gain (vertical speed per metre of error). [decomp] 5, pool.
const CLIMB_GAIN: f32 = 5.0;
/// Sink band: a height error from this (m) up to 0 sinks at `SINK_GAIN`. [decomp] -5, pool.
const SINK_BAND: f32 = -5.0;
/// Sink gain (vertical speed per metre of error). [decomp] 3, pool.
const SINK_GAIN: f32 = 3.0;
/// Extra climb per m/s the target climbs at (`2 * Vd.y`). [unconfirmed] the factor 2 (pool holds 2, also the
/// response ratio); its use here is spec pseudocode.
const TARGET_CLIMB_GAIN: f32 = 2.0;

/// The desired velocity for a goal, from the body's position and velocity.
pub fn desired(
    position: Vec3,
    velocity: Vec3,
    goal: &Goal,
    heights: &dyn Heights,
    gains: &Gains,
    camera: Option<Camera>,
) -> Desired {
    let mut velocity = velocity;
    let mut work = position + velocity * WORK_LEAD;
    let dest_velocity_h = Vec3::new(goal.dest_velocity.x, 0.0, goal.dest_velocity.z);
    let mut dest = goal.drive + dest_velocity_h * WORK_LEAD;
    // Keep out of the camera's sphere before anything else moves the destination.
    let kicked = camera.and_then(|camera| camera::avoid(position, velocity, work, camera));
    if let Some(avoided) = kicked {
        dest = avoided.dest;
        velocity = avoided.velocity;
    }
    floor_point(&mut dest, heights);
    let roof_steer = ahead_roof(position, ahead_point(position, velocity), heights);
    let body_velocity = lead_roof(position, velocity, &mut work, &mut dest, heights).or(kicked.map(|a| a.velocity));

    let to = dest - work;
    let len = to.length();
    let mut yd = to.y;
    let mut move_h = Vec3::new(to.x, 0.0, to.z);
    let dir = move_h.normalize_or_zero();
    let speed_h = Vec2::new(velocity.x, velocity.z).length();
    let closing = (velocity - dest_velocity_h).dot(dir);
    let (vmax, max_decel) = brake(goal.speed, closing, len, speed_h, velocity, dir, gains);
    if len > SPEED_SCALE_MIN {
        move_h *= vmax / len;
    }
    yd = vertical(yd, goal.dest_velocity.y);
    let move_v = Vec3::new(move_h.x, yd, move_h.z);
    let velocity = blend_steer(move_v, roof_steer, vmax);
    Desired { velocity, max_decel, body_velocity }
}

/// The point the helicopter is at in `AHEAD_LEAD` seconds on its present velocity.
pub fn ahead_point(position: Vec3, velocity: Vec3) -> Vec3 {
    position + velocity * AHEAD_LEAD
}

/// Raises a point to the floor under it, if the heights cover it.
fn floor_point(point: &mut Vec3, heights: &dyn Heights) {
    let Some(floor) = heights.elevation(point.x, point.z) else { return };
    point.y = point.y.max(floor);
}

/// The rising-roof rule (spec §4.2, the look-ahead point). If the floor under the look-ahead point is above it, the point
/// is lifted to that floor and the steering direction is returned: the direction to the lifted point plus a side push
/// from the surface normal. Zero when nothing rises. [unconfirmed] the side push's exact form (see [`side_push`]).
fn ahead_roof(position: Vec3, ahead: Vec3, heights: &dyn Heights) -> Vec3 {
    let Some(floor) = heights.floor(ahead.x, ahead.z) else { return Vec3::ZERO };
    if floor.height <= ahead.y {
        return Vec3::ZERO;
    }
    let raised = Vec3::new(ahead.x, floor.height, ahead.z);
    let dir = (raised - position).normalize_or_zero();
    dir + side_push(floor.normal, dir, SHEET_STEER_AHEAD)
}

/// The lead-point rule (spec §4.2, the work point). If the floor under the work point is above it, the work point is
/// lifted to that floor, shortened to the destination's distance when it reaches past it, the destination is lifted to
/// the work point, and the body velocity to set is returned: the direction to the work point plus a side push, at the
/// body's present speed. [unconfirmed] the shortening: the spec's "scale to |dest - pos|" is read as "only when longer
/// than the destination and over 0.2 m". Scaling it up always would lift the lead point far past the goal.
fn lead_roof(position: Vec3, velocity: Vec3, work: &mut Vec3, dest: &mut Vec3, heights: &dyn Heights) -> Option<Vec3> {
    let floor = heights.floor(work.x, work.z)?;
    if floor.height <= work.y {
        return None;
    }
    work.y = floor.height;
    let to_dest = (*dest - position).length();
    let offset = *work - position;
    let to_work = offset.length();
    if to_work > 0.2 && to_work > to_dest {
        *work = position + offset * (to_dest / to_work);
    }
    dest.y = dest.y.max(work.y);
    let dir = (*work - position).normalize_or_zero();
    let dir = dir + side_push(floor.normal, dir, SHEET_STEER_LEAD);
    Some(dir * velocity.length())
}

/// The side push of a steering rule: `n2 = (-n.y, 0, n.x) * gain * |n2 x dir|`, taken as the spec writes it, with `n2`
/// before scaling. Zero for a zero normal, so a source without normals gets no push. [unconfirmed] this reading: the
/// spec gives it in one line and the pool has no use site for it.
fn side_push(normal: Vec3, dir: Vec3, gain: f32) -> Vec3 {
    let p = Vec3::new(-normal.y, 0.0, normal.x);
    p * (gain * p.cross(dir).length())
}

/// Whether the approach brakes, and the speed it may use: braking hard at the goal cuts the speed, and a body moving
/// against a near goal brakes at its maximum. Returns the speed and the max-deceleration flag.
fn brake(speed: f32, closing: f32, len: f32, speed_h: f32, velocity: Vec3, dir: Vec3, gains: &Gains) -> (f32, bool) {
    if closing > CLOSING_BRAKE && len <= braking_distance(closing, speed_h, gains) {
        return (speed * gains.brake[1], true);
    }
    if closing <= CLOSING_BRAKE && velocity.dot(dir) < -1.0 && len < REVERSE_BRAKE_RANGE {
        return (speed, true);
    }
    (speed, false)
}

/// The distance needed to stop from `closing` m/s, with the acceleration the flight model allows.
fn braking_distance(closing: f32, speed_h: f32, gains: &Gains) -> f32 {
    let avail = (speed_h * 0.2 + gains.acceleration[1]).min(gains.acceleration[0]);
    let used = (closing * 2.0).min(avail);
    closing * closing / (gains.brake[0] * used)
}

/// Blends a steering direction into the move: a longer steer than `STEER_MIN` takes `STEER_BLEND` of the speed.
fn blend_steer(move_v: Vec3, steer: Vec3, vmax: f32) -> Vec3 {
    if steer.length() <= STEER_MIN {
        return move_v;
    }
    move_v * (1.0 - STEER_BLEND) + steer.normalize() * (vmax * STEER_BLEND)
}

/// The vertical speed: climb quickly from a little below, sink gently from a little above, and use the target's
/// own vertical speed when it moves the same way.
fn vertical(dy: f32, target_vy: f32) -> f32 {
    if (0.0..CLIMB_BAND).contains(&dy) {
        return dy * CLIMB_GAIN + TARGET_CLIMB_GAIN * target_vy.max(0.0);
    }
    if dy < 0.0 && dy > SINK_BAND {
        return dy * SINK_GAIN + target_vy.min(0.0);
    }
    dy
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_camera_sphere_kicks_the_body_velocity() {
        let camera = Camera { position: Vec3::ZERO, forward: Vec3::X };
        let goal = goal(Vec3::new(-100.0, 5.0, 0.0));
        let flying = Vec3::new(-40.0, 0.0, 0.0);
        let at = Vec3::new(14.0, 5.0, 0.0);
        let free = super::desired(at, flying, &goal, &Flat(0.0), &Gains::default(), None);
        assert!(free.body_velocity.is_none());
        let near = super::desired(at, flying, &goal, &Flat(0.0), &Gains::default(), Some(camera));
        assert!(near.body_velocity.is_some_and(|v| v.x > flying.x), "{:?}", near.body_velocity);
    }

    /// The desired velocity with the game's own gains.
    fn desired(position: Vec3, velocity: Vec3, goal: &Goal, heights: &dyn Heights) -> Desired {
        super::desired(position, velocity, goal, heights, &Gains::default(), None)
    }

    fn goal(drive: Vec3) -> Goal {
        Goal { drive, speed: 50.0, dest_velocity: Vec3::ZERO, look_at: drive }
    }

    /// A roof of height `top` over `x >= edge`, its surface normal `normal`.
    struct Roof {
        edge: f32,
        top: f32,
        normal: Vec3,
    }

    impl Heights for Roof {
        fn elevation(&self, x: f32, _z: f32) -> Option<f32> {
            if x < self.edge {
                return Some(0.0);
            }
            Some(self.top)
        }

        fn floor(&self, x: f32, z: f32) -> Option<Floor> {
            let height = self.elevation(x, z)?;
            Some(Floor { height, normal: self.normal })
        }
    }

    #[test]
    fn heads_for_the_goal_at_the_speed_asked() {
        let d = desired(Vec3::new(0.0, 50.0, 0.0), Vec3::ZERO, &goal(Vec3::new(200.0, 50.0, 0.0)), &Flat(0.0));
        assert!((d.velocity.x - 50.0).abs() < 1e-3, "{:?}", d.velocity);
        assert!(!d.max_decel);
        assert_eq!(d.body_velocity, None);
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
        assert!((vertical(3.0, -4.0) - 15.0).abs() < 1e-5, "a target sinking adds nothing to a climb");
    }

    #[test]
    fn the_floor_under_the_destination_lifts_it_above_the_ground() {
        // At rest at 5 m under a 30 m floor: the destination and the work point go to the floor, so the lead move is
        // level. The look-ahead point is under the floor too, and its steer (straight up, no normal) lifts the move.
        let d = desired(Vec3::new(0.0, 5.0, 0.0), Vec3::ZERO, &goal(Vec3::new(100.0, 5.0, 0.0)), &Flat(30.0));
        assert!(d.velocity.y > 0.0, "{:?}", d.velocity);
        assert_eq!(d.body_velocity, Some(Vec3::ZERO), "the work point is lifted, the body speed is zero");
    }

    #[test]
    fn a_moving_helicopter_under_a_roof_is_steered_up_and_round_it() {
        let position = Vec3::new(0.0, 5.0, 0.0);
        let velocity = Vec3::new(30.0, 0.0, 0.0);
        let d = desired(position, velocity, &goal(Vec3::new(100.0, 5.0, 0.0)), &Flat(30.0));
        let body = d.body_velocity.expect("the lead point is under the floor");
        assert!(body.y > 0.0, "the body is steered up: {body:?}");
        assert!((body.length() - 30.0).abs() < 1e-3, "at the present speed: {body:?}");
        assert!(d.velocity.y > 0.0, "the move climbs too: {:?}", d.velocity);
    }

    #[test]
    fn a_floor_below_the_lead_point_leaves_the_body_alone() {
        let d = desired(
            Vec3::new(0.0, 50.0, 0.0),
            Vec3::new(30.0, 0.0, 0.0),
            &goal(Vec3::new(200.0, 50.0, 0.0)),
            &Flat(0.0),
        );
        assert_eq!(d.body_velocity, None);
    }

    #[test]
    fn the_ahead_point_is_0_65_seconds_on_the_velocity() {
        let p = ahead_point(Vec3::new(0.0, 5.0, 0.0), Vec3::new(10.0, 0.0, -4.0));
        assert!((p - Vec3::new(6.5, 5.0, -2.6)).length() < 1e-5, "{p:?}");
    }

    #[test]
    fn a_roof_rising_ahead_steers_the_look_ahead_point_up() {
        let roof = Roof { edge: 10.0, top: 60.0, normal: Vec3::Y };
        let steer = ahead_roof(
            Vec3::new(0.0, 5.0, 0.0),
            ahead_point(Vec3::new(0.0, 5.0, 0.0), Vec3::new(30.0, 0.0, 0.0)),
            &roof,
        );
        assert!(steer.y > 0.9, "steers up at the roof: {steer:?}");
        assert!(steer.x < 0.3, "the side push turns it off the straight approach: {steer:?}");
    }

    #[test]
    fn no_roof_ahead_means_no_look_ahead_steer() {
        let steer = ahead_roof(Vec3::new(0.0, 5.0, 0.0), Vec3::new(20.0, 5.0, 0.0), &Flat(0.0));
        assert_eq!(steer, Vec3::ZERO);
    }

    #[test]
    fn the_side_push_is_horizontal_and_absent_without_a_normal() {
        assert_eq!(side_push(Vec3::ZERO, Vec3::X, 0.4), Vec3::ZERO);
        let push = side_push(Vec3::Y, Vec3::Y, 0.4);
        assert_eq!(push.y, 0.0);
        assert!((push.x + 0.4).abs() < 1e-5, "{push:?}");
    }

    #[test]
    fn the_lead_point_is_not_carried_past_the_destination() {
        let position = Vec3::new(0.0, 5.0, 0.0);
        let velocity = Vec3::new(30.0, 0.0, 0.0);
        let mut work = position + velocity * WORK_LEAD;
        let mut dest = Vec3::new(5.0, 20.0, 0.0);
        let body = lead_roof(position, velocity, &mut work, &mut dest, &Flat(20.0));
        assert!(body.is_some());
        let limit = Vec3::new(5.0, 15.0, 0.0).length();
        assert!(((work - position).length() - limit).abs() < 1e-3, "{work:?}");
    }

    #[test]
    fn the_lead_point_keeps_its_length_when_the_destination_is_farther() {
        let position = Vec3::new(0.0, 5.0, 0.0);
        let velocity = Vec3::new(30.0, 0.0, 0.0);
        let mut work = position + velocity * WORK_LEAD;
        let mut dest = Vec3::new(100.0, 20.0, 0.0);
        lead_roof(position, velocity, &mut work, &mut dest, &Flat(20.0));
        let lifted = Vec3::new(13.5, 15.0, 0.0).length();
        assert!(((work - position).length() - lifted).abs() < 1e-3, "{work:?}");
        assert_eq!(dest.y, 20.0);
    }

    #[test]
    fn braking_in_time_is_flagged_for_the_hardest_deceleration() {
        // The work point leads the body by 0.45 s of velocity (27 m at 60 m/s), so the goal sits 40 m ahead.
        let fast = Vec3::new(60.0, 0.0, 0.0);
        let d = desired(Vec3::ZERO, fast, &goal(Vec3::new(40.0, 0.0, 0.0)), &Flat(0.0));
        assert!(d.max_decel, "closing fast on a goal 13 m past the work point");
        assert!(d.velocity.x < 60.0);
    }

    #[test]
    fn reversing_towards_a_near_goal_brakes_hard() {
        // Moving backwards along -X, with the goal 9.5 m ahead of the work point: the reverse rule.
        let d = desired(
            Vec3::new(0.0, 50.0, 0.0),
            Vec3::new(-10.0, 0.0, 0.0),
            &goal(Vec3::new(5.0, 50.0, 0.0)),
            &Flat(0.0),
        );
        assert!(d.max_decel);
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
        assert_eq!(d.body_velocity, None);
    }
}
