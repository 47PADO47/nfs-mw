//! The drawn front-wheel steering angle and the drawn wheel roll: presentation values kept apart from the physical
//! wheel angle and speed (spec `docs/specs/car-wheel-animation.md`). The drawn angle turns towards the physical one at
//! the car's `SteerSpeed` and stops at its `MaxTireSteer`; the roll advances at most 17 degrees a step, or less than half
//! a spoke pitch, and stays within one turn. Both are updated once per simulation step and interpolated between steps.

use std::f32::consts::{PI, TAU};

use nfsmw_data::car::WheelSetup;

use super::{CarSim, PHYSICS_WHEEL};

/// The largest roll a wheel is drawn to make in one update without a spoke count, radians (17 degrees).
const DEFAULT_ROLL_CAP: f32 = 17.0 * PI / 180.0;
/// Beyond this distance from the view (metres) the drawn steering is zero.
pub const STEER_DRAW_DISTANCE: f32 = 100.0;

/// A car's drawn-wheel limits.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WheelVisuals {
    /// Radians per second; `None` follows the physical angle at once.
    steer_speed: Option<f32>,
    /// Radians; `None` is unlimited.
    max_steer: Option<f32>,
    /// Radians per update.
    roll_cap: f32,
}

impl Default for WheelVisuals {
    fn default() -> Self {
        Self { steer_speed: None, max_steer: None, roll_cap: DEFAULT_ROLL_CAP }
    }
}

impl WheelVisuals {
    /// The limits of a car with the wheel set-up `setup` (its `ecar` record); without one, only the default roll cap.
    pub fn from_setup(setup: Option<&WheelSetup>) -> Self {
        let Some(setup) = setup else { return Self::default() };
        let spokes = setup.spoke_count.unsigned_abs();
        let roll_cap = match spokes {
            0 => DEFAULT_ROLL_CAP,
            n => (180.0 / n as f32 - 1.0).max(1.0).to_radians(),
        };
        Self {
            steer_speed: setup.steer_speed.map(f32::to_radians),
            max_steer: setup.max_tire_steer.map(f32::to_radians),
            roll_cap,
        }
    }

    /// The drawn angle after `dt` seconds of turning from `drawn` towards `physical` (radians).
    pub fn steer(&self, drawn: f32, physical: f32, dt: f32) -> f32 {
        let moved = match self.steer_speed {
            Some(speed) => drawn + (physical - drawn).clamp(-speed * dt, speed * dt),
            None => physical,
        };
        match self.max_steer {
            Some(max) => moved.clamp(-max, max),
            None => moved,
        }
    }

    /// The drawn roll after `dt` seconds at angular speed `rate` (radians per second), capped and kept within a turn.
    pub fn roll(&self, roll: f32, rate: f32, dt: f32) -> f32 {
        let delta = (rate * dt).clamp(-self.roll_cap, self.roll_cap);
        let next = roll + delta;
        match next.is_finite() {
            true => next % TAU,
            false => 0.0,
        }
    }
}

/// `t` of the way from roll `a` to roll `b`, the short way round: a step never rolls more than half a turn, so the short
/// way is the way the wheel turned, also across the wrap.
pub fn roll_between(a: f32, b: f32, t: f32) -> f32 {
    let delta = (b - a + PI).rem_euclid(TAU) - PI;
    a + delta * t
}

impl CarSim {
    /// The drawn-wheel limits of this car.
    pub fn set_wheel_visuals(&mut self, visuals: WheelVisuals) {
        self.wheel_visuals = visuals;
    }

    /// Whether the front wheels are drawn steering: off beyond the draw distance, where they are drawn straight.
    pub fn set_steer_visible(&mut self, visible: bool) {
        self.steer_visible = visible;
    }

    /// Advances the drawn roll and steering by one step of `dt` seconds from the physical wheels.
    pub(super) fn advance_wheel_visuals(&mut self, dt: f32) {
        let visuals = self.wheel_visuals;
        for (spin, &physics) in self.spin.iter_mut().zip(&PHYSICS_WHEEL) {
            *spin = visuals.roll(*spin, self.vehicle.wheel(physics).angular_velocity, dt);
        }
        for (i, drawn) in self.drawn_steer.iter_mut().enumerate() {
            // The library steers to the right, the model to the left.
            let physical = -self.vehicle.wheel(PHYSICS_WHEEL[i]).steer_angle;
            *drawn = match self.steer_visible {
                true => visuals.steer(*drawn, physical, dt),
                false => 0.0,
            };
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cop() -> WheelVisuals {
        WheelVisuals { steer_speed: Some(90f32.to_radians()), max_steer: Some(40f32.to_radians()), roll_cap: 0.3 }
    }

    #[test]
    fn the_drawn_angle_turns_at_the_steer_speed() {
        // 90 degrees a second for one 60 Hz step is 1.5 degrees, however far the wheel snapped.
        let drawn = cop().steer(0.0, 30f32.to_radians(), 1.0 / 60.0);
        assert!((drawn.to_degrees() - 1.5).abs() < 1e-4, "{}", drawn.to_degrees());
    }

    #[test]
    fn the_drawn_angle_stops_at_the_max_tire_steer() {
        let mut drawn = 0.0;
        for _ in 0..200 {
            drawn = cop().steer(drawn, 60f32.to_radians(), 1.0 / 60.0);
        }
        assert!((drawn.to_degrees() - 40.0).abs() < 1e-3, "{}", drawn.to_degrees());
    }

    #[test]
    fn a_noisy_physical_angle_is_drawn_steady() {
        // A lock-to-lock flip every step moves the drawn wheels by one step's turn, never by the flip.
        let mut drawn = 0.0f32;
        for step in 0..60 {
            let physical = if step % 2 == 0 { 0.6 } else { -0.6 };
            let next = cop().steer(drawn, physical, 1.0 / 60.0);
            assert!((next - drawn).abs() <= 1.5f32.to_radians() + 1e-6);
            drawn = next;
        }
    }

    #[test]
    fn without_limits_the_drawn_angle_is_the_physical_one() {
        assert_eq!(WheelVisuals::default().steer(0.1, -0.4, 1.0 / 60.0), -0.4);
    }

    #[test]
    fn the_spoke_rule_keeps_the_roll_under_half_a_spoke_pitch() {
        let setup = WheelSetup { spoke_count: -5, ..WheelSetup::from_stats(&Default::default()) };
        let visuals = WheelVisuals::from_setup(Some(&setup));
        let roll = visuals.roll(0.0, 200.0, 1.0 / 60.0);
        assert!((roll.to_degrees() - 35.0).abs() < 1e-3, "{}", roll.to_degrees());
        assert!((WheelVisuals::default().roll(0.0, -200.0, 1.0 / 60.0).to_degrees() + 17.0).abs() < 1e-3);
    }

    #[test]
    fn the_roll_stays_within_a_turn_and_interpolates_across_the_wrap() {
        let visuals = WheelVisuals::default();
        let mut roll = 0.0;
        for _ in 0..1000 {
            let next = visuals.roll(roll, 100.0, 1.0 / 60.0);
            assert!(next.abs() < TAU);
            // Halfway between two steps is always half a step on, forwards.
            let half = roll_between(roll, next, 0.5);
            let ahead = (half - roll + PI).rem_euclid(TAU) - PI;
            assert!((ahead - DEFAULT_ROLL_CAP * 0.5).abs() < 1e-4, "{ahead}");
            roll = next;
        }
    }
}
