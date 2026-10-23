//! The curves: interpolation between two records and extrapolation past the newest.

use nfsmw_sim::EntityView;

use super::{InterpConfig, TICK_MS};
use crate::geom;

/// The record between `a` (at `u = 0`) and `b` (at `u = 1`), `span_s` seconds apart: Hermite position, slerped
/// orientation, linear velocities, discrete fields from `a` until `u = 0.5`.
pub fn interpolate(a: &EntityView, b: &EntityView, u: f32, span_s: f32) -> EntityView {
    let m0 = geom::scale(a.linear_velocity, span_s);
    let m1 = geom::scale(b.linear_velocity, span_s);
    let discrete = match u < 0.5 {
        true => a,
        false => b,
    };
    EntityView {
        position: geom::hermite(a.position, m0, b.position, m1, u),
        orientation: geom::slerp(a.orientation, b.orientation, u),
        linear_velocity: geom::lerp(a.linear_velocity, b.linear_velocity, u),
        angular_velocity: geom::lerp(a.angular_velocity, b.angular_velocity, u),
        ..*discrete
    }
}

/// Whether `a` and `b`, `span_s` seconds apart, are too far apart to have driven: a teleport, a respawn, a reset.
pub fn is_teleport(a: &EntityView, b: &EntityView, span_s: f32, config: &InterpConfig) -> bool {
    let speed = geom::length(a.linear_velocity).max(geom::length(b.linear_velocity));
    let reach = speed * span_s * 1.5 + config.teleport_margin_m;
    let gap = geom::distance(a.position, b.position);
    !gap.is_finite() || gap > reach
}

/// The seconds of full-velocity motion after `late_ms` past the newest record: the time itself up to the decay
/// start, then a velocity that falls linearly to zero at the limit, then nothing more.
pub fn effective_seconds(late_ms: f64, config: &InterpConfig) -> f64 {
    let full = config.extrapolate_full_ms.max(0.0);
    let limit = config.extrapolate_limit_ms.max(full);
    let t = late_ms.clamp(0.0, limit);
    if t <= full {
        return t / 1000.0;
    }
    let ramp = limit - full;
    if ramp <= 0.0 {
        return full / 1000.0;
    }
    let past = t - full;
    (full + past - past * past / (2.0 * ramp)) / 1000.0
}

/// `newest` moved on by its velocity and angular velocity for `late_ms` (see [`effective_seconds`]). The drawn
/// velocities keep the decay, so the remote car's sound and effects slow down with it.
pub fn extrapolate(newest: &EntityView, late_ms: f64, config: &InterpConfig) -> EntityView {
    let seconds = effective_seconds(late_ms, config) as f32;
    let turn = geom::from_rotation_vector(geom::scale(newest.angular_velocity, seconds));
    let remaining = velocity_factor(late_ms, config) as f32;
    EntityView {
        position: geom::add(newest.position, geom::scale(newest.linear_velocity, seconds)),
        orientation: geom::mul(turn, newest.orientation),
        linear_velocity: geom::scale(newest.linear_velocity, remaining),
        angular_velocity: geom::scale(newest.angular_velocity, remaining),
        ..*newest
    }
}

/// The share of the velocity still applied `late_ms` past the newest record.
fn velocity_factor(late_ms: f64, config: &InterpConfig) -> f64 {
    let full = config.extrapolate_full_ms;
    let limit = config.extrapolate_limit_ms;
    if late_ms <= full {
        return 1.0;
    }
    if late_ms >= limit || limit <= full {
        return 0.0;
    }
    1.0 - (late_ms - full) / (limit - full)
}

/// Seconds between two ticks.
pub fn span_seconds(from: u32, to: u32) -> f32 {
    (f64::from(to.wrapping_sub(from)) * TICK_MS / 1000.0) as f32
}
