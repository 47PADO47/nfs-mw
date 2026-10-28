//! The engine, the gearbox and the clutch of a bike, turned into one torque at the rear wheel.
//!
//! The clutch is a torque limiter: below the engine's idle speed it slips, and the engine runs at a launch speed that
//! follows the throttle; above it, the engine speed is the wheel's speed times the gear. The gears are changed
//! automatically.

use std::f32::consts::TAU;

use super::spec::{EngineSpec, TransmissionSpec};

/// A gear change takes this long, and the drive is cut for the first part of it (s).
const SHIFT_TIME: f32 = 0.15;
/// The clutch slips at the launch speed (a share of the rev limit) until the wheel catches up with it.
const LAUNCH_RPM_SHARE: f32 = 0.7;

/// The drive side of the bike.
#[derive(Clone, Debug)]
pub struct Powertrain {
    gear: usize,
    rpm: f32,
    shift_timer: f32,
}

/// The value of a `(x, y)` curve at `x`, held flat outside its ends.
pub fn curve(points: &[(f32, f32)], x: f32) -> f32 {
    let Some(&(first_x, first_y)) = points.first() else { return 0.0 };
    if x <= first_x {
        return first_y;
    }
    for pair in points.windows(2) {
        let ((x0, y0), (x1, y1)) = (pair[0], pair[1]);
        if x <= x1 {
            return y0 + (y1 - y0) * ((x - x0) / (x1 - x0).max(1e-6));
        }
    }
    points.last().map_or(0.0, |p| p.1)
}

impl Powertrain {
    pub fn new(engine: &EngineSpec) -> Self {
        Self { gear: 0, rpm: engine.idle_rpm, shift_timer: 0.0 }
    }

    /// The gear in use, 1 for first.
    pub fn gear(&self) -> usize {
        self.gear + 1
    }

    pub fn rpm(&self) -> f32 {
        self.rpm
    }

    /// Whether a gear change is under way.
    pub fn shifting(&self) -> bool {
        self.shift_timer > 0.0
    }

    /// Match the engine and the gear to a road speed (the bike starts already moving).
    pub fn match_speed(&mut self, engine: &EngineSpec, transmission: &TransmissionSpec, wheel_spin: f32) {
        self.gear = 0;
        for index in 0..transmission.gears.len() {
            self.gear = index;
            if wheel_rpm(transmission, index, wheel_spin) < transmission.up_shift * engine.max_rpm {
                break;
            }
        }
        self.rpm = wheel_rpm(transmission, self.gear, wheel_spin).max(engine.idle_rpm);
    }

    /// Advance by `dt` with the rear wheel turning at `wheel_spin` (rad/s) under `throttle`; returns the torque the drive
    /// puts on the wheel (N m, negative for engine braking).
    pub fn step(
        &mut self,
        engine: &EngineSpec,
        transmission: &TransmissionSpec,
        throttle: f32,
        wheel_spin: f32,
        dt: f32,
    ) -> f32 {
        self.shift_timer = (self.shift_timer - dt).max(0.0);
        let ratio = transmission.gears.get(self.gear).copied().unwrap_or(1.0) * transmission.final_ratio;
        let coupled = wheel_rpm(transmission, self.gear, wheel_spin.max(0.0));
        let launching = self.gear == 0 && throttle > 0.0 && coupled < engine.idle_rpm;
        let launch = engine.idle_rpm + throttle * (LAUNCH_RPM_SHARE * engine.max_rpm - engine.idle_rpm);
        self.rpm = if launching { launch.max(coupled) } else { coupled.max(engine.idle_rpm) };
        self.change_gear(engine, transmission, throttle, coupled);
        if self.shifting() && self.shift_timer > SHIFT_TIME * 0.5 {
            return 0.0;
        }
        let engaged = coupled >= engine.idle_rpm * 0.9 || launching;
        if !engaged {
            return 0.0;
        }
        let limited = self.rpm >= engine.max_rpm;
        let drive = engine.peak_torque * curve(&engine.torque_curve, self.rpm) * throttle * f32::from(!limited);
        let drag = engine.friction_torque * (self.rpm / engine.max_rpm) * (1.0 - throttle);
        (drive - drag) * ratio * transmission.efficiency
    }

    fn change_gear(&mut self, engine: &EngineSpec, transmission: &TransmissionSpec, throttle: f32, coupled: f32) {
        if self.shifting() {
            return;
        }
        let last = transmission.gears.len().saturating_sub(1);
        if self.gear < last && throttle > 0.3 && coupled > transmission.up_shift * engine.max_rpm {
            self.gear += 1;
            self.shift_timer = SHIFT_TIME;
            return;
        }
        if self.gear > 0 && coupled < transmission.down_shift * engine.max_rpm {
            self.gear -= 1;
            self.shift_timer = SHIFT_TIME;
        }
    }
}

/// The engine speed (rpm) that a rear wheel turning at `spin` rad/s forces in gear index `gear`.
fn wheel_rpm(transmission: &TransmissionSpec, gear: usize, spin: f32) -> f32 {
    let ratio = transmission.gears.get(gear).copied().unwrap_or(1.0) * transmission.final_ratio;
    spin * ratio * 60.0 / TAU
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bike::BikeSpec;

    #[test]
    fn the_curve_interpolates_and_holds_its_ends() {
        let points = [(0.0, 0.0), (10.0, 100.0)];
        assert_eq!(curve(&points, -5.0), 0.0);
        assert_eq!(curve(&points, 5.0), 50.0);
        assert_eq!(curve(&points, 50.0), 100.0);
        assert_eq!(curve(&[], 1.0), 0.0);
    }

    #[test]
    fn a_standing_start_drives_the_wheel_in_first_gear() {
        let spec = BikeSpec::example();
        let mut train = Powertrain::new(&spec.engine);
        let torque = train.step(&spec.engine, &spec.transmission, 1.0, 0.0, 1.0 / 60.0);
        assert!(torque > 100.0, "{torque}");
        assert_eq!(train.gear(), 1);
        assert!(train.rpm() > spec.engine.idle_rpm);
    }

    #[test]
    fn a_closed_throttle_at_a_standstill_does_nothing() {
        let spec = BikeSpec::example();
        let mut train = Powertrain::new(&spec.engine);
        assert_eq!(train.step(&spec.engine, &spec.transmission, 0.0, 0.0, 1.0 / 60.0), 0.0);
    }

    #[test]
    fn a_closed_throttle_at_speed_brakes_with_the_engine() {
        let spec = BikeSpec::example();
        let mut train = Powertrain::new(&spec.engine);
        train.match_speed(&spec.engine, &spec.transmission, 60.0);
        assert!(train.step(&spec.engine, &spec.transmission, 0.0, 60.0, 1.0 / 60.0) < 0.0);
    }

    #[test]
    fn the_box_changes_up_as_the_wheel_speeds_up_and_cuts_the_drive_for_the_change() {
        let spec = BikeSpec::example();
        let mut train = Powertrain::new(&spec.engine);
        let mut spin = 5.0;
        let mut cut = false;
        for _ in 0..1200 {
            spin += 0.2;
            let torque = train.step(&spec.engine, &spec.transmission, 1.0, spin, 1.0 / 60.0);
            cut |= train.shifting() && torque == 0.0;
        }
        assert!(train.gear() > 1);
        assert!(cut);
    }

    #[test]
    fn the_rev_limiter_cuts_the_torque() {
        let spec = BikeSpec::example();
        let mut train = Powertrain::new(&spec.engine);
        train.match_speed(&spec.engine, &spec.transmission, 30.0);
        let top = spec.transmission.gears.len() - 1;
        let spin = spec.engine.max_rpm * TAU / 60.0 / (spec.transmission.gears[top] * spec.transmission.final_ratio);
        let mut last = 1.0;
        for _ in 0..30 {
            last = train.step(&spec.engine, &spec.transmission, 1.0, spin * 1.05, 1.0 / 60.0);
        }
        assert!(last <= 0.0 || train.shifting(), "{last}");
    }
}
