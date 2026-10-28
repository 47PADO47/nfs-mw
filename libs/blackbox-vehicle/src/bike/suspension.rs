//! The spring, damper and stops of one fork or shock.
//!
//! The suspension is quasi-static: the wheel assembly has no mass of its own in the ground solve, so the compression is
//! whatever puts the tyre on the ground (`Bike` solves it), and this module turns a compression and its speed into the
//! force along the suspension axis. The weight of the wheel is carried by the tyre load.

use super::spec::{DamperSpec, SuspensionSpec};

/// How hard the bump stop at full compression pushes back, as a multiple of the spring rate.
const BOTTOM_OUT_FACTOR: f32 = 12.0;

/// A suspension with its rest state worked out: the compression the static load puts on it.
#[derive(Clone, Debug)]
pub struct Suspension {
    spec: SuspensionSpec,
    /// Compression at rest (m): the preload.
    preload: f32,
}

impl Suspension {
    /// The suspension under the static `load` (N along its axis).
    pub fn new(spec: SuspensionSpec, load: f32) -> Self {
        let mut suspension = Self { spec, preload: 0.0 };
        // The spring rate depends on the travel through the leverage ratio: iterate to the compression it settles at.
        for _ in 0..8 {
            let rate = suspension.rate(suspension.preload);
            suspension.preload = (load / rate.max(1.0)).clamp(0.0, suspension.spec.max_travel);
        }
        suspension
    }

    /// The travel left to extend from rest (m), as a negative compression.
    pub fn rest_extension(&self) -> f32 {
        -self.preload
    }

    /// The most the suspension compresses from rest (m).
    pub fn rest_compression(&self) -> f32 {
        self.spec.max_travel - self.preload
    }

    /// The leverage ratio of wheel travel to shock travel when `total` metres of the travel are used.
    fn ratio(&self, total: f32) -> f32 {
        let points = &self.spec.leverage;
        let Some(first) = points.first() else { return 1.0 };
        let share = (total / self.spec.max_travel.max(1e-6)).clamp(0.0, 1.0);
        let mut previous = *first;
        for &(at, value) in points {
            if share <= at {
                let span = (at - previous.0).max(1e-6);
                return previous.1 + (value - previous.1) * ((share - previous.0) / span).clamp(0.0, 1.0);
            }
            previous = (at, value);
        }
        previous.1
    }

    /// The spring rate at the wheel when `total` metres of the travel are used (N/m).
    fn rate(&self, total: f32) -> f32 {
        let ratio = self.ratio(total);
        self.spec.spring / (ratio * ratio)
    }

    /// The damper force at the wheel for a compression speed `speed` (m/s, positive when compressing).
    fn damper(&self, speed: f32) -> f32 {
        let d: &DamperSpec = &self.spec.damper;
        let ratio = self.ratio(self.preload);
        let (slow, fast, threshold) = match speed >= 0.0 {
            true => (d.slow_bump, d.fast_bump, d.bump_threshold),
            false => (d.slow_rebound, d.fast_rebound, d.rebound_threshold),
        };
        let v = speed.abs();
        let force = slow * v.min(threshold) + fast * (v - threshold).max(0.0);
        force.copysign(speed) / (ratio * ratio)
    }

    /// The force along the axis (N, pushing the wheel away from the frame) when the compression is `compression` metres
    /// beyond rest and changing at `speed` m/s. Never negative: the suspension cannot pull.
    ///
    /// `effective_mass` is the mass the force moves along the axis (kg): the damper is applied implicitly against it, so a
    /// stiff damper on a light effective mass slows the motion without overshooting it at any step size.
    pub fn force(&self, compression: f32, speed: f32, dt: f32, effective_mass: f32) -> f32 {
        let total = self.preload + compression;
        let damper = self.damper(speed);
        let slope = if speed.abs() > 1e-4 { damper / speed } else { 0.0 };
        let damper = damper / (1.0 + slope * dt / effective_mass.max(1e-3));
        let mut force = self.rate(total) * total + damper;
        if total < 0.0 {
            force += self.spec.top_out_stiffness * (-total);
        }
        let over = total - self.spec.max_travel;
        if over > 0.0 {
            force += BOTTOM_OUT_FACTOR * self.spec.spring * over;
        }
        force.max(0.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bike::BikeSpec;

    const DT: f32 = 1.0 / 60.0;
    const MASS: f32 = 60.0;

    fn fork(load: f32) -> Suspension {
        Suspension::new(BikeSpec::example().front_suspension, load)
    }

    #[test]
    fn the_preload_carries_the_static_load() {
        let s = fork(600.0);
        assert!((s.force(0.0, 0.0, DT, MASS) - 600.0).abs() < 1.0);
    }

    #[test]
    fn compressing_pushes_back_harder_and_extending_less() {
        let s = fork(600.0);
        assert!(s.force(0.03, 0.0, DT, MASS) > s.force(0.0, 0.0, DT, MASS));
        assert!(s.force(-0.03, 0.0, DT, MASS) < s.force(0.0, 0.0, DT, MASS));
    }

    #[test]
    fn the_damper_resists_the_motion_in_both_directions() {
        let s = fork(600.0);
        assert!(s.force(0.0, 0.5, DT, MASS) > s.force(0.0, 0.0, DT, MASS));
        assert!(s.force(0.0, -0.5, DT, MASS) < s.force(0.0, 0.0, DT, MASS));
    }

    #[test]
    fn the_leverage_ratio_softens_the_rear_wheel_rate() {
        let spec = BikeSpec::example().rear_suspension;
        let direct = SuspensionSpec { leverage: Vec::new(), ..spec.clone() };
        let with = Suspension::new(spec, 900.0);
        let without = Suspension::new(direct, 900.0);
        assert!(with.rest_compression() < without.rest_compression());
    }

    #[test]
    fn the_bottom_stop_stiffens_past_the_travel() {
        let s = fork(600.0);
        let travel = s.rest_compression();
        assert!(s.force(travel + 0.02, 0.0, DT, MASS) - s.force(travel, 0.0, DT, MASS) > 1000.0);
    }

    #[test]
    fn a_stiff_damper_on_a_light_mass_cannot_reverse_the_motion() {
        let s = fork(600.0);
        // The damper alone would give far more than the mass can take in one step at this speed.
        let with_spring = s.force(0.0, 0.5, DT, 2.0) - s.force(0.0, 0.0, DT, 2.0);
        assert!(with_spring < 2.0 * 0.5 / DT, "{with_spring}");
        assert!(with_spring > 0.0);
    }

    #[test]
    fn the_force_never_pulls() {
        let s = fork(600.0);
        assert!(s.force(-1.0, -5.0, DT, MASS) >= 0.0);
    }
}
