//! The shared approach of the ram-type goals (box-in, rolling block, pit and the pull-over): the aim point in the target's
//! frame, the lead term that moves the aim with the target, the speed ceiling and the smoothed limiter, and the drive
//! command they give. Spec: `docs/specs/ai-pursuit-goals.md` sections 2.4, 5 and 6; `docs/specs/ai-pursuit-tactics.md`
//! section 5.1.

use glam::Vec2;

/// Metres per second in one km/h.
pub const KMH: f32 = 1.0 / 3.6;
/// The target counts as moving at this speed or more (5 km/h). Below it, the frame and the lead use its body heading.
pub const FRAME_SPEED: f32 = 5.0 * KMH;
/// The lead term is off when the motion vector is this short (m/s).
const LEAD_GATE: f32 = 0.01;
/// The lead coefficient in `0.7 dot(r, v) / |v|^2`.
const LEAD_COEFFICIENT: f32 = 0.7;
/// The limiter pulls its value down over this band below the cop's speed (5 km/h).
const LIMITER_BAND: f32 = 5.0 * KMH;
/// The separation's opposing speed is only counted above this (m/s).
const OPPOSING_GATE: f32 = 0.0001;

/// The cop's state at one think.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CopState {
    /// The cop's position in the map plane, metres.
    pub position: Vec2,
    /// The cop's speed along its own forward, m/s (negative in reverse).
    pub forward_speed: f32,
    /// The limiter's value from the previous think, m/s. Start a new goal at 0.
    pub limit: f32,
    /// Seconds since the previous think.
    pub dt: f32,
}

/// The target's state at one think.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TargetState {
    /// The target's position in the map plane, metres.
    pub position: Vec2,
    /// The target's velocity, m/s.
    pub velocity: Vec2,
    /// The target's body forward. Any length: it is normalised, and used when the target moves below 5 km/h.
    pub heading: Vec2,
    /// The target's own top speed, m/s.
    pub top_speed: f32,
    /// The target's acceleration at the cop's speed, m/s^2. The caller reads the target's table at
    /// [`CopState::forward_speed`]; this module does not hold the table.
    pub accel: f32,
    /// Whether the target is a jerk pursuit (`IsAJerk`), which raises the cop's multipliers.
    pub jerk: bool,
}

/// The cop's own tuning from its `aivehicle` row.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AiVehicle {
    /// `MAXIMUM_AI_SPEED`, km/h.
    pub max_speed_kmh: f32,
    /// `TopSpeedMultiplier`.
    pub top_speed_mult: f32,
    /// `AccelerationMultiplier`.
    pub accel_mult: f32,
}

/// The speed ceiling and the acceleration it allows, m/s and m/s^2.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Ceiling {
    pub max_speed: f32,
    pub max_accel: f32,
}

/// The drive command for one think, for the car driver.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GoalCommand {
    /// The point to drive to, in the map plane (`x`, `z`), metres. `None` when the goal sets no drive target (an arrived
    /// pull-over). The caller places it at the cop's height.
    pub target_point: Option<Vec2>,
    /// The speed to drive at, m/s: `min(|steer|, limit)`. Zero when the goal has arrived.
    pub desired_speed: f32,
    /// The goal is complete for this think. The caller then brakes, sets the handbrake and centres the steering.
    pub arrived: bool,
}

/// One think's result: the command and the limiter value to store for the next think.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Step {
    pub command: GoalCommand,
    pub limit: f32,
}

/// The target's motion vector: its velocity when it moves at 5 km/h or more, else its body heading normalised. The frame
/// and the lead both read it (spec 2.4 and 5.2).
pub fn motion_vector(velocity: Vec2, heading: Vec2) -> Vec2 {
    if velocity.length() >= FRAME_SPEED {
        return velocity;
    }
    heading.normalize_or_zero()
}

/// The slot point of an in-position offset (or a pursuit slot) in the target's frame: `target + side * x + forward * z`,
/// where `forward` is the motion vector normalised and `side = (forward.z, -forward.x)` (spec 2.4). The `y` of `offset`
/// is the spec's `z`. A zero motion vector puts the slot on the target.
pub fn slot_point(target: Vec2, motion: Vec2, offset: Vec2) -> Vec2 {
    let forward = motion.normalize_or_zero();
    let side = Vec2::new(forward.y, -forward.x);
    target + side * offset.x + forward * offset.y
}

/// The aim point of a record for a target: [`slot_point`] with the target's own motion vector.
pub fn aim_for(target: &TargetState, offset: Vec2) -> Vec2 {
    slot_point(target.position, motion_vector(target.velocity, target.heading), offset)
}

/// The lead offset without the one-second term: `r + v 0.7 dot(r, v) / |v|^2` with `r = slot - cop`. This is the tactics
/// spec's `newseek - position` (tactics 5.1). The lead is only applied when `|v| > 0.01`; otherwise it is `r` alone.
pub fn lead_offset(cop: Vec2, slot: Vec2, motion: Vec2) -> Vec2 {
    let r = slot - cop;
    if motion.length() <= LEAD_GATE {
        return r;
    }
    let speed_sq = motion.length_squared();
    r + motion * (LEAD_COEFFICIENT * r.dot(motion) / speed_sq)
}

/// The steering seek of the lead term (spec 5.2): `seek = r + v (1 + 0.7 dot(r, v) / |v|^2)`, that is [`lead_offset`]
/// plus the one-second term `v`. The one-second term is only added when `|v| > 0.01`. A target ahead along `v` is led
/// further; a slot behind the target shrinks the lead.
pub fn lead_seek(cop: Vec2, slot: Vec2, motion: Vec2) -> Vec2 {
    let offset = lead_offset(cop, slot, motion);
    if motion.length() <= LEAD_GATE {
        return offset;
    }
    offset + motion
}

/// The speed-jerk factors of a jerk pursuit (cop-cars spec section 1): the ceiling `1.1`, the speed multiplier `1.2`
/// and the acceleration multiplier `1.5`. Other pursuits use 1.
struct JerkFactors {
    ceiling: f32,
    speed: f32,
    accel: f32,
}

fn jerk_factors(jerk: bool) -> JerkFactors {
    match jerk {
        true => JerkFactors { ceiling: 1.1, speed: 1.2, accel: 1.5 },
        false => JerkFactors { ceiling: 1.0, speed: 1.0, accel: 1.0 },
    }
}

/// The speed ceiling and the acceleration limit (spec 6, tactics 3.3). `max_speed = min(MAXIMUM_AI_SPEED, target top
/// speed x TopSpeedMultiplier)`; `max_accel = target accel x AccelerationMultiplier`; a jerk raises the three factors.
///
/// [unconfirmed] The goal spec does not show the nitrous factor (0.2) or the grade term (`-9.81 forward.y`) that the
/// tactics spec lists for this ceiling (goals spec Q4). They are left out; they may sit in the shared limiter.
pub fn ceiling(vehicle: AiVehicle, target: &TargetState) -> Ceiling {
    let jerk = jerk_factors(target.jerk);
    let own_max = vehicle.max_speed_kmh * KMH * jerk.ceiling;
    let target_max = target.top_speed * vehicle.top_speed_mult * jerk.speed;
    Ceiling { max_speed: own_max.min(target_max), max_accel: target.accel * vehicle.accel_mult * jerk.accel }
}

/// The smoothed speed limit (the performance limiter of tactics 3.3). Forward speeds: while the cop is more than 5 km/h
/// under the limit, the limit is pulled down (by `min(1, dt t^2)` of the gap, with `t` the gap in 5 km/h units). Then
/// the limit is `max(speed, limit) + max_accel dt`, capped at `max_speed`.
///
/// [unconfirmed] The spec's branch is `speed > 0` with a mirror for the rest. This takes the forward branch for
/// `speed >= 0`, so a cop at rest gets a forward limit. Negative speeds take the mirror image (negated limit).
pub fn limiter_step(limit: f32, speed: f32, max_speed: f32, max_accel: f32, dt: f32) -> f32 {
    if speed < 0.0 {
        return -forward_limit(-limit, -speed, max_speed, max_accel, dt);
    }
    forward_limit(limit, speed, max_speed, max_accel, dt)
}

fn forward_limit(limit: f32, speed: f32, max_speed: f32, max_accel: f32, dt: f32) -> f32 {
    let mut limit = limit;
    if speed < limit {
        let t = (limit - speed) / LIMITER_BAND;
        limit -= (dt * t * t).min(1.0) * (limit - speed);
    }
    let grown = limit.max(speed) + max_accel * dt;
    grown.min(max_speed)
}

/// The limiter's new value for this think, from the cop's state and the target's ceiling.
pub fn limit_step(cop: &CopState, target: &TargetState, vehicle: AiVehicle) -> f32 {
    let ceil = ceiling(vehicle, target);
    limiter_step(cop.limit, cop.forward_speed, ceil.max_speed, ceil.max_accel, cop.dt)
}

/// Combine a push (a separation or avoidance steering) with the seek (spec 3.3 and 5.1). When the push opposes the seek,
/// the part of the seek along the push is multiplied by `clamp((full - counter) / span, 0, 1)`, where `counter` is the
/// opposing speed in m/s. `full_kmh` and `span_kmh` are in km/h. The pull-over uses 55 and 55; the close chase uses 35
/// and 25. The result is `push + seek'`.
pub fn combine(seek: Vec2, push: Vec2, full_kmh: f32, span_kmh: f32) -> Vec2 {
    let seek_len = seek.length();
    if seek_len <= 0.0 {
        return push + seek;
    }
    let counter = -push.dot(seek) / seek_len;
    if counter <= OPPOSING_GATE {
        return push + seek;
    }
    let dir = push.normalize_or_zero();
    let along = dir * seek.dot(dir);
    let weight = ((full_kmh * KMH - counter) / (span_kmh * KMH)).clamp(0.0, 1.0);
    push + (seek - along) + along * weight
}

/// The drive command for a steering vector: the point `cop + steer` and the speed `min(|steer|, limit)`.
pub fn drive(cop: Vec2, steer: Vec2, limit: f32) -> GoalCommand {
    GoalCommand { target_point: Some(cop + steer), desired_speed: steer.length().min(limit), arrived: false }
}

/// One think of the ram action (spec 5): steer at the lead seek to `aim` (the slot point, from [`aim_for`]), at the
/// speed `min(|steer|, limit)`. The box-in, rolling block and pit goals use this with their record's aim.
pub fn ram(cop: &CopState, target: &TargetState, aim: Vec2, vehicle: AiVehicle) -> Step {
    let seek = lead_seek(cop.position, aim, motion_vector(target.velocity, target.heading));
    let limit = limit_step(cop, target, vehicle);
    Step { command: drive(cop.position, seek, limit), limit }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-4
    }

    fn vec_close(a: Vec2, b: Vec2) -> bool {
        (a - b).length() < 1e-4
    }

    fn still_target_facing_plus_z() -> TargetState {
        TargetState {
            position: Vec2::ZERO,
            velocity: Vec2::ZERO,
            heading: Vec2::Y,
            top_speed: 20.0,
            accel: 4.0,
            jerk: false,
        }
    }

    fn vehicle(max_kmh: f32) -> AiVehicle {
        AiVehicle { max_speed_kmh: max_kmh, top_speed_mult: 1.0, accel_mult: 1.0 }
    }

    #[test]
    fn a_moving_target_sets_the_frame_by_its_velocity() {
        // Moving at 10 m/s along +Z: the frame is the velocity, the heading is ignored.
        let motion = motion_vector(Vec2::new(0.0, 10.0), Vec2::X);
        assert_eq!(motion, Vec2::new(0.0, 10.0));
        assert!(vec_close(slot_point(Vec2::ZERO, motion, Vec2::new(0.0, 14.0)), Vec2::new(0.0, 14.0)));
    }

    #[test]
    fn a_target_under_5_kmh_sets_the_frame_by_its_heading() {
        // 1 m/s is under 5 km/h (1.389 m/s): the heading (+X here, unit length) sets the frame.
        let motion = motion_vector(Vec2::new(0.0, 1.0), Vec2::X);
        assert_eq!(motion, Vec2::X);
        assert!(vec_close(slot_point(Vec2::ZERO, motion, Vec2::new(0.0, 14.0)), Vec2::new(14.0, 0.0)));
    }

    #[test]
    fn a_slot_is_the_target_plus_side_and_forward_offsets() {
        // Target at the origin facing +Z: side is +X, so x = 4 is 4 m to the target's right, z = -2.7 is behind.
        let fwd = Vec2::Y;
        assert!(vec_close(slot_point(Vec2::ZERO, fwd, Vec2::new(4.0, -2.7)), Vec2::new(4.0, -2.7)));
        assert!(vec_close(slot_point(Vec2::ZERO, fwd, Vec2::new(-3.5, 0.0)), Vec2::new(-3.5, 0.0)));
    }

    #[test]
    fn a_slot_moves_with_the_target() {
        let target = Vec2::new(100.0, 50.0);
        let slot = slot_point(target, Vec2::Y, Vec2::new(0.0, 14.0));
        assert!(vec_close(slot, Vec2::new(100.0, 64.0)));
    }

    #[test]
    fn the_lead_of_a_stopped_target_runs_along_its_heading() {
        // Cop at the origin, slot 10 m ahead, target still with heading +Z. |v| = 1 (heading), r = (0, 10), r.v = 10:
        // seek = (0, 10) + (0, 1) (1 + 0.7 * 10 / 1) = (0, 18).
        let seek = lead_seek(Vec2::ZERO, Vec2::new(0.0, 10.0), Vec2::Y);
        assert!(vec_close(seek, Vec2::new(0.0, 18.0)), "{seek:?}");
    }

    #[test]
    fn the_lead_of_a_target_moving_at_its_slot_is_1_7_times_the_slot() {
        // Velocity (0, 10), r = (0, 10): r.v / |v|^2 = 1, so the factor is 1.7 and seek = (0, 10) + (0, 17) = (0, 27).
        let seek = lead_seek(Vec2::ZERO, Vec2::new(0.0, 10.0), Vec2::new(0.0, 10.0));
        assert!(vec_close(seek, Vec2::new(0.0, 27.0)), "{seek:?}");
    }

    #[test]
    fn a_slot_behind_a_moving_target_shrinks_the_lead() {
        // r = (0, -5) against v = (0, 10): r.v / |v|^2 = -0.5, factor 0.65: seek = (0, -5) + (0, 6.5) = (0, 1.5).
        let seek = lead_seek(Vec2::ZERO, Vec2::new(0.0, -5.0), Vec2::new(0.0, 10.0));
        assert!(vec_close(seek, Vec2::new(0.0, 1.5)), "{seek:?}");
    }

    #[test]
    fn with_no_motion_the_seek_is_the_plain_offset() {
        let seek = lead_seek(Vec2::new(1.0, 1.0), Vec2::new(4.0, 5.0), Vec2::ZERO);
        assert!(vec_close(seek, Vec2::new(3.0, 4.0)));
    }

    #[test]
    fn the_arrival_offset_leaves_out_the_one_second_term() {
        // Slot on the cop: the offset is zero even though a still target's heading is unit length. The steering seek
        // keeps the one-second term, so it is the heading.
        let offset = lead_offset(Vec2::ZERO, Vec2::ZERO, Vec2::Y);
        let seek = lead_seek(Vec2::ZERO, Vec2::ZERO, Vec2::Y);
        assert!(vec_close(offset, Vec2::ZERO), "{offset:?}");
        assert!(vec_close(seek, Vec2::Y), "{seek:?}");
    }

    #[test]
    fn the_ceiling_takes_the_lower_of_the_cop_and_the_target() {
        // The cop's 360 km/h is 100 m/s; the target's 90 m/s is lower.
        let mut target = still_target_facing_plus_z();
        target.top_speed = 90.0;
        let c = ceiling(vehicle(360.0), &target);
        assert!(close(c.max_speed, 90.0), "{}", c.max_speed);
        assert!(close(c.max_accel, 4.0));
    }

    #[test]
    fn a_jerk_raises_the_ceiling_by_1_1_and_the_target_speed_by_1_2() {
        // Cop 100 m/s x 1.1 = 110; target 90 x 1.2 = 108. The lower is 108.
        let mut target = still_target_facing_plus_z();
        target.top_speed = 90.0;
        target.jerk = true;
        let c = ceiling(vehicle(360.0), &target);
        assert!(close(c.max_speed, 108.0), "{}", c.max_speed);
        assert!(close(c.max_accel, 6.0), "accel x 1.5: {}", c.max_accel);
    }

    #[test]
    fn the_acceleration_is_the_target_table_times_the_multiplier() {
        let mut target = still_target_facing_plus_z();
        target.accel = 4.0;
        let cop = AiVehicle { max_speed_kmh: 360.0, top_speed_mult: 1.0, accel_mult: 2.0 };
        assert!(close(ceiling(cop, &target).max_accel, 8.0));
    }

    #[test]
    fn the_limiter_grows_from_rest_at_the_allowed_acceleration() {
        // Limit 0, speed 0, accel 4 m/s^2, dt 0.5 s, ceiling 20: the limit grows to 2.
        let limit = limiter_step(0.0, 0.0, 20.0, 4.0, 0.5);
        assert!(close(limit, 2.0), "{limit}");
    }

    #[test]
    fn the_limiter_pulls_down_when_the_cop_is_well_under_it() {
        // Limit 20, speed 10: the gap is 10 m/s, t = 10 / 1.389 = 7.2, dt t^2 > 1, so the limit drops to 10. Then it
        // grows by 4 * 0.5 = 2 to 12.
        let limit = limiter_step(20.0, 10.0, 30.0, 4.0, 0.5);
        assert!(close(limit, 12.0), "{limit}");
    }

    #[test]
    fn the_limiter_is_capped_at_the_ceiling() {
        let limit = limiter_step(29.9, 29.9, 30.0, 4.0, 1.0);
        assert!(close(limit, 30.0), "{limit}");
    }

    #[test]
    fn the_limiter_mirrors_in_reverse() {
        // Speed -10 with limit -20 is the forward case (10, 20) negated: 12 becomes -12.
        let limit = limiter_step(-20.0, -10.0, 30.0, 4.0, 0.5);
        assert!(close(limit, -12.0), "{limit}");
    }

    #[test]
    fn a_push_away_from_the_seek_does_not_cut_it() {
        let seek = Vec2::new(0.0, 10.0);
        let combined = combine(seek, Vec2::new(4.0, 0.0), 55.0, 55.0);
        assert!(vec_close(combined, Vec2::new(4.0, 10.0)), "{combined:?}");
    }

    #[test]
    fn a_push_that_opposes_the_seek_cuts_the_seek_along_it() {
        // Push (0, -4) against seek (0, 10): counter 4 m/s. Full 55 km/h = 15.28 m/s, span 15.28 m/s:
        // weight = (15.28 - 4) / 15.28 = 0.738. Along part = (0, 10) x 0.738 = (0, 7.38). Result (0, -4) + (0, 7.38).
        let combined = combine(Vec2::new(0.0, 10.0), Vec2::new(0.0, -4.0), 55.0, 55.0);
        assert!((combined.y - 3.3818).abs() < 1e-3, "{combined:?}");
        assert!(combined.x.abs() < 1e-5);
    }

    #[test]
    fn no_push_leaves_the_seek() {
        let seek = Vec2::new(3.0, 4.0);
        assert!(vec_close(combine(seek, Vec2::ZERO, 55.0, 55.0), seek));
    }

    #[test]
    fn the_ram_steers_at_the_lead_seek_and_caps_the_speed_by_the_limiter() {
        // Cop at (0, -10) at rest, slot 14 ahead of a still target at the origin. Seek: r = (0, 24), v = (0, 1), factor
        // 1 + 0.7 * 24 = 17.8, so seek = (0, 41.8). Limit grows from 0 by 4 * 0.5 = 2, so the speed is 2.
        let cop = CopState { position: Vec2::new(0.0, -10.0), forward_speed: 0.0, limit: 0.0, dt: 0.5 };
        let step = ram(&cop, &still_target_facing_plus_z(), Vec2::new(0.0, 14.0), vehicle(360.0));
        let cmd = step.command;
        assert!(close(step.limit, 2.0), "{}", step.limit);
        assert!(close(cmd.desired_speed, 2.0), "speed {}", cmd.desired_speed);
        assert!(!cmd.arrived);
        assert!(cmd.target_point.is_some_and(|p| vec_close(p, Vec2::new(0.0, 31.8))), "{:?}", cmd.target_point);
    }

    #[test]
    fn drive_caps_the_speed_at_the_steer_length() {
        let cmd = drive(Vec2::ZERO, Vec2::new(0.0, 1.0), 20.0);
        assert!(close(cmd.desired_speed, 1.0), "min(|steer| = 1, limit = 20)");
        assert_eq!(cmd.target_point, Some(Vec2::new(0.0, 1.0)));
    }
}
