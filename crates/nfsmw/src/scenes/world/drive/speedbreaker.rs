//! Speedbreaker charge and activation, from docs/specs/speedbreaker.md.

/// Real seconds to deplete a full charge.
pub const DRAIN_SECONDS: f32 = 10.0;
/// Simulation seconds to refill while strictly above the recharge speed.
pub const RECHARGE_SECONDS: f32 = 25.0;
/// Minimum speedometer speed for activation/continued use (30 mph).
pub const MINIMUM_SPEED_MPS: f32 = 30.0 * 0.44704;
/// Recharge occurs strictly above 100 mph.
pub const RECHARGE_ABOVE_MPS: f32 = 100.0 * 0.44704;

/// Charge, active state and button-edge tracking. New driving sessions start full.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SpeedBreaker {
    pub meter: f32,
    active: bool,
    held: bool,
}

impl Default for SpeedBreaker {
    fn default() -> Self {
        Self { meter: 1.0, active: false, held: false }
    }
}

impl SpeedBreaker {
    pub fn active(&self) -> bool {
        self.active
    }

    /// Drift recharge uses physical speed/body slip and the continuous fixed-step drift state.
    pub fn charge_drift(&mut self, strength: f32, speed_mps: f32, slip_radians: f32, sim_dt: f32) {
        const MIN_SPEED: f32 = 15.646_05;
        const MIN_SLIP: f32 = 0.523_589_97;
        if self.active || speed_mps <= MIN_SPEED || slip_radians.abs() <= MIN_SLIP || strength <= 0.0 {
            return;
        }
        self.meter = (self.meter + sim_dt * 0.5 * strength).clamp(0.0, 1.0);
    }

    /// Toggle on a press, then update using distinct simulation and real deltas.
    /// The caller supplies speedometer speed and handles world clock scaling.
    pub fn step(&mut self, held: bool, speed_mps: f32, sim_dt: f32, real_dt: f32) -> bool {
        let pressed = held && !self.held;
        self.held = held;
        let eligible = speed_mps >= MINIMUM_SPEED_MPS;
        if !eligible {
            self.active = false;
            return false;
        }
        if pressed {
            self.active = !self.active && self.meter > 0.0;
        }
        if self.active {
            self.meter = (self.meter - real_dt / DRAIN_SECONDS).max(0.0);
            if self.meter <= 0.0 {
                self.active = false;
            }
            return self.active;
        }
        if speed_mps > RECHARGE_ABOVE_MPS {
            self.meter = (self.meter + sim_dt / RECHARGE_SECONDS).min(1.0);
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toggles_on_presses_and_remains_active_after_release() {
        let mut sb = SpeedBreaker::default();
        assert!(sb.step(true, 50.0, 0.0, 0.0));
        assert!(sb.step(true, 50.0, 0.0, 0.0));
        assert!(sb.step(false, 50.0, 0.0, 0.0));
        assert!(!sb.step(true, 50.0, 0.0, 0.0));
    }

    #[test]
    fn drains_in_real_time_and_stops_when_empty() {
        let mut sb = SpeedBreaker::default();
        assert!(sb.step(true, 50.0, 0.5, 2.0));
        assert!((sb.meter - 0.8).abs() < 1e-6);
        assert!(!sb.step(false, 50.0, 2.0, 8.0));
        assert_eq!(sb.meter, 0.0);
        assert!(!sb.active());
    }

    #[test]
    fn recharges_only_strictly_above_one_hundred_mph_in_simulation_time() {
        let mut sb = SpeedBreaker { meter: 0.0, ..SpeedBreaker::default() };
        sb.step(false, RECHARGE_ABOVE_MPS, 25.0, 100.0);
        assert_eq!(sb.meter, 0.0);
        sb.step(false, RECHARGE_ABOVE_MPS + 1.0, 5.0, 20.0);
        assert!((sb.meter - 0.2).abs() < 1e-6);
        sb.step(false, RECHARGE_ABOVE_MPS + 1.0, 25.0, 100.0);
        assert_eq!(sb.meter, 1.0);
    }

    #[test]
    fn rejects_activation_and_deactivates_below_thirty_mph() {
        let mut sb = SpeedBreaker::default();
        assert!(!sb.step(true, MINIMUM_SPEED_MPS - 1.0, 0.0, 0.0));
        sb.step(false, MINIMUM_SPEED_MPS, 0.0, 0.0);
        assert!(sb.step(true, MINIMUM_SPEED_MPS, 0.0, 0.0));
        assert!(!sb.step(false, MINIMUM_SPEED_MPS - 1.0, 0.0, 0.0));
    }

    #[test]
    fn drift_recharge_uses_strict_physical_gates_and_continuous_strength() {
        let mut sb = SpeedBreaker { meter: 0.0, ..SpeedBreaker::default() };
        for (strength, speed, slip) in [(1.0, 15.646_05, 0.6), (1.0, 20.0, 0.523_589_97), (0.0, 20.0, 0.6)] {
            sb.charge_drift(strength, speed, slip, 1.0);
            assert_eq!(sb.meter, 0.0);
        }
        sb.charge_drift(0.25, 20.0, -0.6, 2.0);
        assert_eq!(sb.meter, 0.25);
        assert!(sb.step(true, 20.0, 0.0, 0.0));
        sb.charge_drift(1.0, 20.0, 0.6, 1.0);
        assert_eq!(sb.meter, 0.25);
        sb.step(false, 20.0, 0.0, 0.0);
        sb.step(true, 20.0, 0.0, 0.0);
        for _ in 0..120 {
            sb.charge_drift(0.25, 20.0, 0.6, 1.0 / 60.0);
        }
        assert!((sb.meter - 0.5).abs() < 1e-5);
        sb.charge_drift(1.0, 20.0, 0.6, 2.0);
        assert_eq!(sb.meter, 1.0);
    }
}

#[cfg(test)]
mod input_tests;
