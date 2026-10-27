//! The handling score of a car, from its stats. Top speed and acceleration come from the longitudinal estimate the
//! host already has (`docs/specs/garage-performance-bars.md`); handling has no such estimate for a car without a rating
//! of its own, so this scores the grip a car can use, on a fixed scale.

use super::CarStats;

/// The lateral acceleration, in g, that scores 0 and 1. A road tyre on a family car is near the low end and the best
/// road-legal tyres are near the high end.
pub const HANDLING_RANGE: (f32, f32) = (1.0, 1.9);
/// Share of the score that is cornering grip; the rest is braking.
const CORNERING_SHARE: f32 = 0.75;
/// Gravity, m/s².
const GRAVITY: f32 = 9.81;
/// `cg height / track` below which a car loses nothing to rolling, and the range over which it loses the most.
const ROLL_FREE: f32 = 0.2;
const ROLL_SPAN: f32 = 0.3;
/// The most a tall car loses of its grip to weight transfer.
const ROLL_LOSS: f32 = 0.25;

/// Cornering grip in g: the mean friction coefficient of the tyres, less what a high centre of gravity on a narrow
/// track gives up to weight transfer.
fn cornering_g(stats: &CarStats) -> f32 {
    let grip = 0.5 * (stats.tire_grip[0] + stats.tire_grip[1]);
    let track = 0.5 * (stats.track_m[0] + stats.track_m[1]);
    let tall = ((stats.cg_height_m / track.max(0.1) - ROLL_FREE) / ROLL_SPAN).clamp(0.0, 1.0);
    grip * (1.0 - ROLL_LOSS * tall)
}

/// Braking in g: the brakes' torque at the wheels over the weight, up to what the tyres hold.
fn braking_g(stats: &CarStats) -> f32 {
    let radius = 0.5 * (stats.tire_radius_m[0] + stats.tire_radius_m[1]);
    let torque = 2.0 * (stats.brake_torque_nm[0] + stats.brake_torque_nm[1]);
    let brakes = torque / radius.max(0.1) / (stats.mass_kg.max(1.0) * GRAVITY);
    brakes.min(0.5 * (stats.tire_grip[0] + stats.tire_grip[1]))
}

/// The handling score of the car, 0 to 1: three quarters cornering grip and a quarter braking, on [`HANDLING_RANGE`].
pub fn handling_index(stats: &CarStats) -> f32 {
    let g = CORNERING_SHARE * cornering_g(stats) + (1.0 - CORNERING_SHARE) * braking_g(stats);
    ((g - HANDLING_RANGE.0) / (HANDLING_RANGE.1 - HANDLING_RANGE.0)).clamp(0.0, 1.0)
}
