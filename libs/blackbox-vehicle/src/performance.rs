//! What a car can do flat out: its top speed and how hard it accelerates at each speed. The AI limits its
//! requested speed with these numbers (`docs/specs/ai-driver-speed-skill.md` §2, open question 1).
//!
//! The original's estimator is not in the sources; this measures the car by driving it full throttle on
//! flat ground, which is what the table is meant to describe.

use crate::FIXED_STEP;
use crate::ground::FlatGround;
use crate::input::InputState;
use crate::vehicle::{Vehicle, VehicleSpec};

/// Samples of the acceleration table, evenly spaced from 0 to the top speed.
pub const TABLE_SIZE: usize = 10;
/// Gravity the grip is rated against (the car rigid body's value).
const GRAVITY: f32 = 9.8128;
/// The longest run, seconds.
const MAX_SECONDS: f32 = 90.0;
/// The run ends once the speed has gained less than this over [`SETTLE_SECONDS`].
const SETTLE_GAIN: f32 = 0.05;
const SETTLE_SECONDS: f32 = 4.0;

/// A car's measured performance.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Performance {
    /// The speed full throttle settles at, m/s.
    pub top_speed: f32,
    /// Acceleration in m/s² at `top_speed * i / (TABLE_SIZE - 1)`.
    pub acceleration: [f32; TABLE_SIZE],
    /// Tire grip (a multiple of `g`) at rest and, with the downforce, at top speed.
    pub start_grip: f32,
    pub end_grip: f32,
}

impl Performance {
    /// The acceleration at `speed` (linear between samples; 0 when the car has no top speed).
    pub fn acceleration_at(&self, speed: f32) -> f32 {
        if self.top_speed <= 0.0 {
            return 0.0;
        }
        let position = (speed.abs() / self.top_speed * (TABLE_SIZE - 1) as f32).clamp(0.0, (TABLE_SIZE - 1) as f32);
        let low = position.floor() as usize;
        let high = (low + 1).min(TABLE_SIZE - 1);
        let t = position - low as f32;
        self.acceleration[low] * (1.0 - t) + self.acceleration[high] * t
    }
}

/// Drives `spec` flat out on flat ground and reads off its top speed and acceleration table.
pub fn measure(spec: &VehicleSpec) -> Performance {
    let ground = FlatGround::new(0.0);
    let mut car = Vehicle::new(spec.clone());
    car.place_on_ground(&ground, 0.0, 0.0, 5.0, 0.0);
    let input = InputState { throttle: 1.0, ..InputState::default() };
    let steps = (MAX_SECONDS / FIXED_STEP) as usize;
    let window = (SETTLE_SECONDS / FIXED_STEP) as usize;
    let mut speeds: Vec<f32> = Vec::with_capacity(steps);
    for step in 0..steps {
        car.step(FIXED_STEP, &input, &ground);
        speeds.push(car.forward_speed().max(0.0));
        if step > window && speeds[step] - speeds[step - window] < SETTLE_GAIN {
            break;
        }
    }
    let top_speed = speeds.iter().copied().fold(0.0, f32::max);
    let mut table = [0.0; TABLE_SIZE];
    let start_grip = spec.tires.static_grip[0].min(spec.tires.static_grip[1]);
    let downforce = top_speed * 2.0 * spec.aero.aero_coefficient * 1000.0;
    let end_grip = start_grip * (GRAVITY + downforce / spec.mass.max(1.0)) / GRAVITY;
    if top_speed <= 0.0 {
        return Performance { top_speed, acceleration: table, start_grip, end_grip };
    }
    // The acceleration over a second around the moment the car passes each sample speed (a gear change
    // makes single steps useless); the last sample uses what is left at the end of the run.
    let span = (1.0 / FIXED_STEP) as usize;
    for (i, slot) in table.iter_mut().enumerate() {
        let target = top_speed * i as f32 / (TABLE_SIZE - 1) as f32;
        let at = speeds.iter().position(|&s| s >= target).unwrap_or(speeds.len() - 1);
        let (from, to) = (at.saturating_sub(span / 2), (at + span / 2).min(speeds.len() - 1));
        *slot = match to > from {
            true => ((speeds[to] - speeds[from]) / ((to - from) as f32 * FIXED_STEP)).max(0.0),
            false => 0.0,
        };
    }
    Performance { top_speed, acceleration: table, start_grip, end_grip }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_example_car_has_a_sensible_top_speed_and_a_falling_acceleration() {
        let p = measure(&VehicleSpec::example());
        assert!((30.0..130.0).contains(&p.top_speed), "{}", p.top_speed);
        assert!(p.acceleration[0] > 2.0, "{:?}", p.acceleration);
        assert!(p.acceleration[0] > p.acceleration[TABLE_SIZE - 1] + 1.0, "{:?}", p.acceleration);
        assert!(p.acceleration.iter().all(|a| a.is_finite() && *a >= 0.0));
        assert!(p.start_grip > 0.5 && p.end_grip >= p.start_grip, "{} {}", p.start_grip, p.end_grip);
    }

    #[test]
    fn the_table_is_read_linearly_and_clamped() {
        let p = Performance {
            top_speed: 90.0,
            acceleration: [9.0, 8.0, 7.0, 6.0, 5.0, 4.0, 3.0, 2.0, 1.0, 0.0],
            start_grip: 1.0,
            end_grip: 1.0,
        };
        assert_eq!(p.acceleration_at(0.0), 9.0);
        assert!((p.acceleration_at(15.0) - 7.5).abs() < 1e-5);
        assert_eq!(p.acceleration_at(500.0), 0.0);
        assert_eq!(Performance { top_speed: 0.0, ..p }.acceleration_at(10.0), 0.0);
    }
}
