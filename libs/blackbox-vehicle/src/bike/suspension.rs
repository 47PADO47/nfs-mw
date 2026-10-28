//! The spring, hydraulic term, damper and stops of one fork or shock (`docs/specs/ride6-suspension.md`).
//!
//! Compression `x` is the wheel travel from full extension (m). The force at the wheel along the axis is
//! `(g(x_s + preload; spring) + g(x_s; hydraulic)) / LR - topout(x)` with `x_s = x / LR(x / travel)` the compression at the
//! shock and `g(x; k, r, f, p) = k (x + f sgn(x) 100^(p-1) max(|x| - r travel, 0)^p)` the progressive spring law (SI form of
//! the law that counts the compression in cm). The wheel has a mass and is integrated here when it is free; on the ground
//! the compression follows the ground (`Bike` splits the penetration between the tyre and this).

use super::choices;
use super::spec::{DamperSpec, Progressive, SuspensionSpec};

/// A suspension with its rest state worked out: the compression the static load puts on it.
#[derive(Clone, Debug)]
pub struct Suspension {
    spec: SuspensionSpec,
    /// Compression at rest (m): from full extension.
    rest: f32,
}

/// The progressive spring law `k (x + f sgn(x) 100^(p-1) max(|x| - r travel, 0)^p)`.
fn progressive(x: f32, rate: f32, term: &Progressive, travel: f32) -> f32 {
    if term.factor == 0.0 {
        return rate * x;
    }
    let over = (x.abs() - term.start * travel).max(0.0);
    rate * (x + x.signum() * term.factor * 100.0_f32.powf(term.linearity - 1.0) * over.powf(term.linearity))
}

impl Suspension {
    /// The suspension under the static `load` (N along its axis, pushing the wheel away from the frame).
    pub fn new(spec: SuspensionSpec, load: f32) -> Self {
        let mut suspension = Self { spec, rest: 0.0 };
        let travel = suspension.spec.max_travel;
        let (mut low, mut high) = (0.0, travel);
        // The force rises with the compression: bisect for the one that carries the load.
        for _ in 0..40 {
            let middle = 0.5 * (low + high);
            match suspension.spring_force(middle) < load {
                true => low = middle,
                false => high = middle,
            }
        }
        suspension.rest = 0.5 * (low + high);
        suspension
    }

    pub fn spec(&self) -> &SuspensionSpec {
        &self.spec
    }

    /// Compression from full extension (m) for a compression `beyond` rest.
    #[cfg(test)]
    pub fn total(&self, beyond: f32) -> f32 {
        self.rest + beyond
    }

    /// The travel left to extend from rest (m), as a negative compression beyond rest.
    pub fn rest_extension(&self) -> f32 {
        -self.rest
    }

    /// The most the suspension compresses from rest (m).
    pub fn rest_compression(&self) -> f32 {
        self.spec.max_travel - self.rest
    }

    /// The leverage ratio of wheel travel to shock travel when `total` metres of the travel are used (at least 1).
    pub fn ratio(&self, total: f32) -> f32 {
        let points = &self.spec.leverage;
        let Some(first) = points.first() else { return 1.0 };
        let share = (total / self.spec.max_travel.max(1e-6)).clamp(0.0, 1.0);
        let mut previous = *first;
        for &(at, value) in points {
            if share <= at {
                let span = (at - previous.0).max(1e-6);
                let ratio = previous.1 + (value - previous.1) * ((share - previous.0) / span).clamp(0.0, 1.0);
                return ratio.max(1.0);
            }
            previous = (at, value);
        }
        previous.1.max(1.0)
    }

    /// The spring, hydraulic and top-out forces at the wheel (N, positive pushes the wheel away from the frame) at the
    /// compression `total`, with no damper.
    pub fn spring_force(&self, total: f32) -> f32 {
        let spec = &self.spec;
        let ratio = self.ratio(total);
        let shock = total / ratio;
        let spring = progressive(shock + spec.rest_offset, spec.spring, &spec.progressive, spec.max_travel);
        let hydraulic = match spec.hydraulic.stiffness > 0.0 {
            true => progressive(shock, spec.hydraulic.stiffness, &spec.hydraulic.term, spec.max_travel),
            false => 0.0,
        };
        let top_out = spec.top_out_stiffness * (spec.top_out_length - total).max(0.0);
        (spring + hydraulic) / ratio - top_out
    }

    /// The damper force at the wheel (N, positive resists compression) for a compression speed `speed` (m/s).
    pub fn damper_force(&self, total: f32, speed: f32) -> f32 {
        let d: &DamperSpec = &self.spec.damper;
        let ratio = self.ratio(total);
        let speed = match self.spec.max_speed > 0.0 {
            true => speed.clamp(-self.spec.max_speed, self.spec.max_speed),
            false => speed,
        };
        // The knee is compared with the speed of the shock; the force is carried to the wheel by the leverage ratio.
        let shock_speed = speed / ratio;
        let (slow, fast, knee) = match speed >= 0.0 {
            true => (d.slow_bump, d.fast_bump, d.bump_threshold),
            false => (d.slow_rebound, d.fast_rebound, d.rebound_threshold),
        };
        let v = shock_speed.abs();
        let force = slow * v.min(knee) + fast * (v - knee).max(0.0);
        force.copysign(speed) / ratio
    }

    /// The damping slope (N s/m) at `speed`: the force over the speed, never negative.
    fn damper_slope(&self, total: f32, speed: f32) -> f32 {
        match speed.abs() > 1e-4 {
            true => (self.damper_force(total, speed) / speed).max(0.0),
            false => self.damper_force(total, 1e-4) / 1e-4,
        }
    }

    /// The force along the axis (N, pushing the wheel away from the frame) when the compression is `total` and changing at
    /// `speed` m/s; signed (the top-out spring can pull the wheel in).
    ///
    /// `effective_mass` is the mass the force moves along the axis (kg): the damper is applied implicitly against it, so a
    /// stiff damper on a light effective mass slows the motion without overshooting it at any step size.
    pub fn force(&self, total: f32, speed: f32, dt: f32, effective_mass: f32) -> f32 {
        let slope = self.damper_slope(total, speed);
        let damper = self.damper_force(total, speed) / (1.0 + slope * dt / effective_mass.max(1e-3));
        self.spring_force(total) + damper
    }

    /// The chain pull, traction and brake coupling forces (N, positive resists compression) at the compression `total`, for the
    /// drive torque at the wheel (N m), the tyre's longitudinal force (N) and the brake torque (N m) on a wheel of radius
    /// `radius`.
    pub fn coupling_force(&self, total: f32, drive_torque: f32, traction: f32, brake_torque: f32, radius: f32) -> f32 {
        let c = &self.spec.coupling;
        let share = (total / self.spec.max_travel.max(1e-6)).clamp(0.0, 1.0);
        let remap = |range: [f32; 2]| range[0] + (range[1] - range[0]) * share;
        // A drive torque of T N m with a chain factor f adds -100 f T newtons: the code's torque is in 1e-4 N m and its force
        // in 1e-2 N.
        let chain = -drive_torque * 100.0 * c.chain * remap(c.chain_range);
        let traction = -traction * c.traction * remap(c.traction_range);
        let brake =
            choices::BRAKE_COUPLING_SIGN * (brake_torque / radius.max(1e-3)) * c.brake * 0.01 * remap(c.brake_range);
        chain + traction + brake
    }

    /// The share of a ground penetration the tyre takes: the softer of the suspension's rate at the wheel and the tyre's
    /// radial stiffness takes more (`ride6-tyre.md`, section 1). 0 for a rigid tyre.
    pub fn tyre_share(&self, total: f32, tire_stiffness: f32) -> f32 {
        if tire_stiffness <= 0.0 {
            return 0.0;
        }
        ((self.spec.spring / self.ratio(total)) / tire_stiffness).min(1.0)
    }

    /// A wheel with no ground under it, free on its suspension for `dt`: returns the compression and speed after the step.
    /// `weight_along_axis` is the wheel's weight along the suspension axis (N, positive pulls the wheel away from the frame).
    pub fn free_step(&self, total: f32, speed: f32, dt: f32, wheel_mass: f32, weight_along_axis: f32) -> (f32, f32) {
        let mass = wheel_mass.max(1e-3);
        let slope = self.damper_slope(total, speed);
        let accel = (-self.spring_force(total) - weight_along_axis) / mass;
        let mut next_speed = (speed + dt * accel) / (1.0 + dt * slope / mass);
        let mut next = total + 0.5 * (speed + next_speed) * dt;
        let travel = self.spec.max_travel;
        if next < 0.0 {
            next = 0.0;
            next_speed = next_speed.max(0.0);
        }
        if next > travel {
            next = travel;
            next_speed = next_speed.min(0.0);
        }
        (next, next_speed)
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
    fn the_rest_compression_carries_the_static_load() {
        let s = fork(600.0);
        assert!((s.force(s.total(0.0), 0.0, DT, MASS) - 600.0).abs() < 1.0);
    }

    #[test]
    fn compressing_pushes_back_harder_and_extending_less() {
        let s = fork(600.0);
        let at = s.force(s.total(0.0), 0.0, DT, MASS);
        assert!(s.force(s.total(0.03), 0.0, DT, MASS) > at);
        assert!(s.force(s.total(-0.03), 0.0, DT, MASS) < at);
    }

    #[test]
    fn the_damper_resists_the_motion_in_both_directions() {
        let s = fork(600.0);
        let t = s.total(0.0);
        assert!(s.force(t, 0.5, DT, MASS) > s.force(t, 0.0, DT, MASS));
        assert!(s.force(t, -0.5, DT, MASS) < s.force(t, 0.0, DT, MASS));
    }

    #[test]
    fn the_damper_has_two_slopes() {
        let s = fork(600.0);
        let t = s.total(0.0);
        let d = BikeSpec::example().front_suspension.damper;
        // Below the knee the slow slope, above it the fast slope on the excess.
        assert!((s.damper_force(t, 0.1) - d.slow_bump * 0.1).abs() < 1e-2);
        let beyond = d.slow_bump * d.bump_threshold + d.fast_bump * 0.2;
        assert!((s.damper_force(t, 0.5) - beyond).abs() < 1e-2);
    }

    #[test]
    fn the_leverage_ratio_softens_the_rear_wheel_rate() {
        let spec = BikeSpec::example().rear_suspension;
        let direct = SuspensionSpec { leverage: Vec::new(), ..spec.clone() };
        let with = Suspension::new(spec, 900.0);
        let without = Suspension::new(direct, 900.0);
        assert!(with.rest_compression() < without.rest_compression());
        assert!(with.ratio(0.0) > 2.7 && with.ratio(0.26) < 2.0);
    }

    #[test]
    fn the_hydraulic_term_follows_the_cm_law() {
        // Front hydraulic term of the example: k 400, factor 1, linearity 2, start 0: 400 (x + 100 x^2) with x in metres.
        let mut spec = BikeSpec::example().front_suspension;
        spec.rest_offset = 0.0;
        spec.top_out_stiffness = 0.0;
        spec.spring = 0.0;
        let s = Suspension::new(spec, 0.0);
        let x = 0.05;
        let expected = 400.0 * (x + 100.0 * x * x);
        assert!((s.spring_force(x) - expected).abs() < 1e-2, "{} {expected}", s.spring_force(x));
    }

    #[test]
    fn the_top_out_spring_pushes_the_wheel_in_near_full_extension() {
        let mut spec = BikeSpec::example().front_suspension;
        let with = Suspension::new(spec.clone(), 600.0).spring_force(0.0);
        spec.top_out_stiffness = 0.0;
        let without = Suspension::new(spec, 600.0).spring_force(0.0);
        assert!(with < without);
    }

    #[test]
    fn a_stiff_damper_on_a_light_mass_cannot_reverse_the_motion() {
        let s = fork(600.0);
        let t = s.total(0.0);
        let with_damper = s.force(t, 0.5, DT, 2.0) - s.force(t, 0.0, DT, 2.0);
        assert!(with_damper < 2.0 * 0.5 / DT, "{with_damper}");
        assert!(with_damper > 0.0);
    }

    #[test]
    fn a_free_wheel_hangs_at_full_extension_and_stops_there() {
        let s = fork(600.0);
        let (mut x, mut v) = (s.total(0.0), 0.0);
        for _ in 0..600 {
            (x, v) = s.free_step(x, v, DT, 18.0, 18.0 * 9.81);
        }
        assert!(x < 0.02, "{x}");
        assert!(v.abs() < 0.05);
    }

    #[test]
    fn a_free_wheel_stops_at_the_bump_stop() {
        let s = fork(600.0);
        let (x, v) = s.free_step(s.spec().max_travel - 0.001, 3.0, DT, 18.0, 0.0);
        assert!(x <= s.spec().max_travel && v <= 0.0);
    }

    #[test]
    fn the_drive_adds_to_the_rear_suspension_force_as_anti_squat_does() {
        let s = Suspension::new(BikeSpec::example().rear_suspension, 900.0);
        let t = s.total(0.0);
        // 1.4 N per N m, remapped from 1 at no compression to 0.6 at full travel.
        let share = t / s.spec().max_travel;
        let expected = 1.4 * 100.0 * (1.0 - 0.4 * share);
        let chain = s.coupling_force(t, 100.0, 0.0, 0.0, 0.32);
        assert!((chain - expected).abs() < 0.5, "{chain} {expected}");
        assert!(chain > 100.0, "{chain}");
        assert!(s.coupling_force(t, 100.0, 1000.0, 0.0, 0.32) > chain);
    }

    #[test]
    fn braking_pulls_the_coupling_force_down_and_a_bike_with_none_has_none() {
        let s = fork(600.0);
        let t = s.total(0.1);
        assert!(s.coupling_force(t, 0.0, 0.0, 500.0, 0.32) < 0.0);
        assert_eq!(s.coupling_force(t, 300.0, 2000.0, 0.0, 0.32), 0.0);
    }

    #[test]
    fn the_tyre_takes_a_share_of_a_penetration_by_the_softer_element() {
        let s = fork(600.0);
        assert_eq!(s.tyre_share(0.05, 0.0), 0.0);
        let share = s.tyre_share(0.05, 400_000.0);
        assert!((share - 8800.0 / 400_000.0).abs() < 1e-6);
        assert_eq!(s.tyre_share(0.05, 100.0), 1.0);
    }
}
