//! The pull-over goal: the cops of a collapse ring drive to their ring slots around a slow target and stop there. It is the
//! ram action with the goal name `AIGoalPullOver` (spec section 7). Spec: `docs/specs/ai-pursuit-goals.md` section 7;
//! `docs/specs/ai-pursuit-tactics.md` section 5.1.
//!
//! The ring slots come from `formation::collapse::ring_slots`. The vehicle separation vector (the driver spec's
//! `VehicleSeperation`) is computed by the caller with the scales from [`separation_scales`]; it is not built here.

use super::approach::{
    AiVehicle, CopState, GoalCommand, KMH, Step, TargetState, combine, drive, lead_seek, limit_step,
};
use glam::Vec2;

/// The opposing-speed scale of the pull-over's separation: 55 km/h, both as the full value and as the span.
pub const SEPARATION_SCALE_KMH: f32 = 55.0;
/// The static avoidance at aggression 0: `4 (1 - a)`. The dynamic radius adds [`DYNAMIC_EXTRA`] on top.
const STATIC_AVOID: f32 = 4.0;
const DYNAMIC_EXTRA: f32 = 1.5;
/// Arrival: `0.2 km/h x (1 - a) + 0.5 km/h` on the seek length.
const ARRIVAL_PER_CALM_KMH: f32 = 0.2;
const ARRIVAL_BASE_KMH: f32 = 0.5;
/// The detour's clearance from the target: `min(|slot - target|, |cop - target|) - 0.2` m.
const CLEARANCE_OFFSET: f32 = 0.2;
/// The detour is used only when its clearance is above this (m).
const DETOUR_MIN_CLEARANCE: f32 = 0.1;

/// The aggression the pull-over uses: the target's `CollapseAggression`, or 1.0 when the target is a jerk pursuit.
pub fn aggression(collapse_aggression: f32, jerk: bool) -> f32 {
    if jerk {
        return 1.0;
    }
    collapse_aggression
}

/// The separation's `(static, dynamic)` scales for an aggression: `(4 (1 - a), 4 (1 - a) + 1.5)`. The caller evaluates the
/// separation vector with them.
pub fn separation_scales(aggression: f32) -> (f32, f32) {
    let static_avoid = (1.0 - aggression) * STATIC_AVOID;
    (static_avoid, static_avoid + DYNAMIC_EXTRA)
}

/// The seek length below which a cop has arrived, in m/s: `(0.2 (1 - a) + 0.5) km/h`.
///
/// Spec: the in-m/s form is `0.0556 (1 - a) + 0.1389`. At heat 5 (a = 0.8) it is 0.54 km/h; at heat 1 (a = 0.4), 0.62 km/h.
pub fn arrival_threshold(aggression: f32) -> f32 {
    ((1.0 - aggression) * ARRIVAL_PER_CALM_KMH + ARRIVAL_BASE_KMH) * KMH
}

/// Whether a cop has arrived: the seek (before separation) is strictly under the threshold.
pub fn has_arrived(seek: Vec2, aggression: f32) -> bool {
    seek.length() < arrival_threshold(aggression)
}

/// The detour around the target (spec 7.3). The straight line from the cop to its slot is replaced by a tangent to the
/// circle of radius `r = min(|slot - target|, |cop - target|) - 0.2` around the target when that line passes within `r`
/// of the target and `r > 0.1`. Returns the tangent point nearer to the slot, or `None` for the straight line.
///
/// Readings, both [unconfirmed]:
/// - The spec gives a tangent length of `2.1 x |slot - target|` (tactics 5.1; goals Q3 says the code's factor is not
///   seen). A tangent from a point at distance `d` from a circle has length at most `d`, so `2.1 d` cannot be a tangent
///   length from the slot. This takes the exact tangent from the cop's own position instead.
/// - "The straight line" is read as the segment from the cop to the slot.
pub fn detour(cop: Vec2, slot: Vec2, target: Vec2) -> Option<Vec2> {
    let clearance = (slot - target).length().min((cop - target).length()) - CLEARANCE_OFFSET;
    if clearance <= DETOUR_MIN_CLEARANCE {
        return None;
    }
    if segment_distance(target, cop, slot) >= clearance {
        return None;
    }
    let (a, b) = tangent_points(cop, target, clearance);
    if (a - slot).length() <= (b - slot).length() {
        return Some(a);
    }
    Some(b)
}

/// The distance from `point` to the segment `a` to `b`.
fn segment_distance(point: Vec2, a: Vec2, b: Vec2) -> f32 {
    let ab = b - a;
    let len_sq = ab.length_squared();
    if len_sq <= 0.0 {
        return (point - a).length();
    }
    let t = ((point - a).dot(ab) / len_sq).clamp(0.0, 1.0);
    (point - (a + ab * t)).length()
}

/// The two tangent points from `from` to the circle of `radius` around `centre`. Needs `radius < |from - centre|`.
fn tangent_points(from: Vec2, centre: Vec2, radius: f32) -> (Vec2, Vec2) {
    let rel = from - centre;
    let dist = rel.length();
    let dir = rel / dist;
    // The angle at the centre between the direction to `from` and each tangent point.
    let theta = (radius / dist).clamp(-1.0, 1.0).acos();
    let (sin, cos) = theta.sin_cos();
    let plus = Vec2::new(dir.x * cos - dir.y * sin, dir.x * sin + dir.y * cos);
    let minus = Vec2::new(dir.x * cos + dir.y * sin, -dir.x * sin + dir.y * cos);
    (centre + plus * radius, centre + minus * radius)
}

/// One think of the pull-over (spec 7 and 5.1). The aim is the ring slot, or the detour point around the target. The
/// cop stops (arrived, no drive target) when the predictive seek to the aim is under [`arrival_threshold`]. Otherwise it
/// steers at the seek combined with `separation`, at the speed `min(|steer|, limit)`.
///
/// The shared ram action predicts with actual target velocity and tests the complete seek before separation.
pub fn think(
    cop: &CopState,
    target: &TargetState,
    slot: Vec2,
    aggression: f32,
    separation: Vec2,
    vehicle: AiVehicle,
) -> Step {
    let aim = detour(cop.position, slot, target.position).unwrap_or(slot);
    let seek = lead_seek(cop.position, aim, target.velocity);
    let limit = limit_step(cop, target, vehicle);
    if has_arrived(seek, aggression) {
        return Step { command: arrived_command(), limit };
    }
    let steer = combine(seek, separation, SEPARATION_SCALE_KMH, SEPARATION_SCALE_KMH);
    Step { command: drive(cop.position, steer, limit), limit }
}

/// The command of an arrived cop: no drive target, no speed. The caller brakes and sets the handbrake.
pub fn arrived_command() -> GoalCommand {
    GoalCommand { target_point: None, desired_speed: 0.0, arrived: true }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-4
    }

    fn vec_close(a: Vec2, b: Vec2) -> bool {
        (a - b).length() < 1e-3
    }

    fn vehicle() -> AiVehicle {
        AiVehicle { max_speed_kmh: 360.0, top_speed_mult: 1.0, accel_mult: 1.0 }
    }

    fn stopped_target() -> TargetState {
        TargetState {
            position: Vec2::ZERO,
            velocity: Vec2::ZERO,
            heading: Vec2::Y,
            top_speed: 20.0,
            accel: 4.0,
            jerk: false,
        }
    }

    #[test]
    fn a_jerk_target_forces_full_aggression() {
        assert_eq!(aggression(0.4, true), 1.0);
        assert_eq!(aggression(0.4, false), 0.4);
    }

    #[test]
    fn the_separation_scales_grow_as_aggression_falls() {
        let (s, d) = separation_scales(0.8);
        assert!(close(s, 0.8) && close(d, 2.3), "a = 0.8: {s} {d}");
        let (s, d) = separation_scales(0.4);
        assert!(close(s, 2.4) && close(d, 3.9), "a = 0.4: {s} {d}");
    }

    #[test]
    fn the_arrival_threshold_is_0_54_kmh_at_heat_five_and_0_62_at_heat_one() {
        assert!(close(arrival_threshold(0.8) / KMH, 0.54), "{}", arrival_threshold(0.8) / KMH);
        assert!(close(arrival_threshold(0.4) / KMH, 0.62), "{}", arrival_threshold(0.4) / KMH);
    }

    #[test]
    fn the_arrival_threshold_in_metres_per_second_matches_the_spec_formula() {
        // 0.0556 (1 - a) + 0.1389 in m/s, at a = 0.8: 0.0111 + 0.1389 = 0.15.
        assert!(close(arrival_threshold(0.8), 0.0556 * 0.2 + 0.1389));
    }

    #[test]
    fn a_seek_under_the_threshold_has_arrived_and_one_over_has_not() {
        let a = 0.8;
        let just_under = Vec2::new(0.0, 0.5 * KMH); // 0.5 km/h, under 0.54 km/h
        let just_over = Vec2::new(0.0, 0.6 * KMH); // 0.6 km/h, over 0.54 km/h
        assert!(has_arrived(just_under, a));
        assert!(!has_arrived(just_over, a));
    }

    #[test]
    fn a_cop_on_its_slot_arrives_and_gets_no_drive_target() {
        let cop = CopState { position: Vec2::new(0.0, -6.0), forward_speed: 0.0, limit: 0.0, dt: 0.5 };
        let step = think(&cop, &stopped_target(), Vec2::new(0.0, -6.0), 0.8, Vec2::ZERO, vehicle());
        assert!(step.command.arrived);
        assert_eq!(step.command.target_point, None, "no drive target once arrived");
        assert_eq!(step.command.desired_speed, 0.0);
    }

    #[test]
    fn a_cop_short_of_its_slot_steers_at_it_with_the_limiter_speed() {
        // Cop at (0, -20), ring slot (0, -10), stationary target: no predictive lead.
        // The limiter grows from 0 by 4 * 0.5 = 2, so the speed is 2 and the drive point remains the slot.
        let cop = CopState { position: Vec2::new(0.0, -20.0), forward_speed: 0.0, limit: 0.0, dt: 0.5 };
        let step = think(&cop, &stopped_target(), Vec2::new(0.0, -10.0), 0.8, Vec2::ZERO, vehicle());
        assert!(!step.command.arrived);
        assert!(close(step.command.desired_speed, 2.0), "{}", step.command.desired_speed);
        assert!(step.command.target_point.is_some_and(|p| vec_close(p, Vec2::new(0.0, -10.0))));
    }

    #[test]
    fn arrival_tests_the_complete_velocity_seek_before_separation() {
        let slot = Vec2::new(0.0, -6.0);
        let target = TargetState { velocity: Vec2::X, ..stopped_target() };
        let cop = CopState { position: slot, forward_speed: 1.0, limit: 2.0, dt: 0.1 };
        let moving = think(&cop, &target, slot, 0.8, Vec2::ZERO, vehicle());
        assert!(!moving.command.arrived, "the moving slot requires a one-second velocity lead");
        let predictive_slot = CopState { position: slot + Vec2::X / 1.7, ..cop };
        let arrived = think(&predictive_slot, &target, slot, 0.8, Vec2::Y * 5.0, vehicle());
        assert!(arrived.command.arrived, "the complete seek cancels before separation is added");
    }

    #[test]
    fn a_straight_line_that_misses_the_target_has_no_detour() {
        // Cop (0, -20) to slot (0, -10): the line is 10 m from the target; the clearance is min(10, 20) - 0.2 = 9.8.
        assert_eq!(detour(Vec2::new(0.0, -20.0), Vec2::new(0.0, -10.0), Vec2::ZERO), None);
    }

    #[test]
    fn a_line_through_the_target_takes_the_detour_nearer_the_slot() {
        // Cop (10, 0), slot (-10, 3), target at the origin. The line passes about 1.5 m from the target, inside the
        // clearance of 9.8 m. The tangent points from the cop are at (9.6, +-1.95); the one nearer the slot is the upper.
        let point = detour(Vec2::new(10.0, 0.0), Vec2::new(-10.0, 3.0), Vec2::ZERO).expect("detour");
        assert!(point.y > 0.0, "the upper tangent is nearer the slot: {point:?}");
        assert!(close(point.length(), 9.8), "the tangent point is on the 9.8 m circle: {}", point.length());
    }

    #[test]
    fn a_detour_needs_a_clearance_above_a_tenth_of_a_metre() {
        // Cop 0.25 m from the target: clearance 0.25 - 0.2 = 0.05, under the 0.1 m gate, even with the line through it.
        assert_eq!(detour(Vec2::new(0.25, 0.0), Vec2::new(-10.0, 0.0), Vec2::ZERO), None);
    }

    #[test]
    fn a_line_on_the_far_side_of_the_clearance_has_no_detour() {
        // Cop (10, 0) to slot (10, 10): the line is 10 m out; the clearance is min(14.1, 10) - 0.2 = 9.8. 10 > 9.8.
        assert_eq!(detour(Vec2::new(10.0, 0.0), Vec2::new(10.0, 10.0), Vec2::ZERO), None);
    }

    #[test]
    fn the_tangent_points_are_on_the_circle_and_perpendicular_to_the_radius() {
        let from = Vec2::new(10.0, 0.0);
        let (a, b) = tangent_points(from, Vec2::ZERO, 5.0);
        for p in [a, b] {
            assert!(close(p.length(), 5.0), "on the circle: {}", p.length());
            assert!((p - from).dot(p).abs() < 1e-3, "tangent: the line to the cop is perpendicular to the radius");
        }
    }
}
