//! The brake reduction of the anti-lock element: a second share of each brake taken away when the bike leans, drifts or unloads
//! the wheel and the throttle is shut (`docs/specs/ride6-powertrain-round6.md`, section 9). This is the reference's older
//! logic (`bUseDeprecatedLogic`, true for every bike), whose terms are range translators of the IMU channels.

use glam::Vec3;

use super::curve::ramp;
use super::{Bike, REAR};

/// A range translator of the reference (`FClampedRangeTranslator`) without a curve and with linearity 1: `input` is mapped to
/// 0..1 (held at the ends, a step at its end when it has no width) and from there to `output`. All zeros, the constructor
/// default, outputs 0 for every input, which is how an unset tuning switches its term off.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct RangeMap {
    pub input: [f32; 2],
    pub output: [f32; 2],
}

impl RangeMap {
    /// A translator that gives `value` for every input (the constructor default of the engine-brake tunings, with 1).
    pub fn constant(value: f32) -> Self {
        Self { input: [0.0, 0.0], output: [value, value] }
    }

    pub fn map(&self, x: f32) -> f32 {
        self.output[0] + (self.output[1] - self.output[0]) * ramp(x, self.input)
    }
}

/// The tunings of one wheel's reduction.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct WheelReduction {
    /// The reduction's intensity (0 switches the wheel off).
    pub intensity: f32,
    /// Of the lean as a share of the lean limit (its absolute value), of the drift angle (deg, absolute value), of the wheel's
    /// load over its rest load, and of the rider's throttle (0 to 1). The throttle term multiplies the others: a wheel with
    /// none (the front of every bike) has no reduction.
    pub lean: RangeMap,
    pub drift: RangeMap,
    pub load: RangeMap,
    pub throttle: RangeMap,
}

/// The brake reduction of both wheels. `Default` has no reduction.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct BrakeReduction {
    /// Of the speed (km/h), for both wheels.
    pub speed: RangeMap,
    pub front: WheelReduction,
    pub rear: WheelReduction,
}

/// What the reduction reads each step.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ReductionInput {
    pub speed_kmh: f32,
    /// The lean as a share of the lean limit and the drift angle (deg) between the heading and the velocity.
    pub lean_share: f32,
    pub drift_deg: f32,
    /// Each wheel's load over its rest load (front, rear).
    pub load_share: [f32; 2],
    /// The rider's own throttle, before any help.
    pub throttle: f32,
}

/// The rate limiter the reference uses on the releases: up at the rise rate, down at the fall rate, never past the target; a
/// rate of 0 or less jumps to the target.
pub fn rate_limit(current: f32, target: f32, rise: f32, fall: f32, dt: f32) -> f32 {
    let diff = target - current;
    let away = (diff > 0.0) == (current > 0.0) || (diff != 0.0 && current == 0.0);
    let step = match away {
        true => rise,
        false => fall,
    } * dt;
    if step <= 0.0 {
        return target;
    }
    current + diff.clamp(-step, step)
}

impl BrakeReduction {
    /// The share of the brake taken from a wheel (0 to 1): the speed term, the intensity, the sum of the lean, drift and load
    /// terms clamped to 0..1, and the throttle term.
    pub fn target(&self, rear: bool, input: &ReductionInput) -> f32 {
        let wheel = match rear {
            true => &self.rear,
            false => &self.front,
        };
        let load = input.load_share[usize::from(rear)];
        let sum =
            wheel.lean.map(input.lean_share.abs()) + wheel.drift.map(input.drift_deg.abs()) + wheel.load.map(load);
        self.speed.map(input.speed_kmh) * wheel.intensity * sum.clamp(0.0, 1.0) * wheel.throttle.map(input.throttle)
    }
}

impl Bike {
    /// The inputs of the brake reduction now: the speed along the bike, the lean over its limit, the angle between the
    /// heading and the velocity, the wheels' loads over their loads at rest, and the rider's throttle.
    pub(super) fn reduction_input(&self, speed: f32, throttle: f32) -> ReductionInput {
        let rotation = self.body.rotation();
        let velocity = self.body.linear_velocity;
        let (forward, right) = (velocity.dot(rotation * Vec3::Z), velocity.dot(rotation * Vec3::X));
        let share = |index: usize| self.load_share(index);
        ReductionInput {
            speed_kmh: speed.abs() * 3.6,
            lean_share: (super::readout::lean_of(rotation).abs() / self.plan.limit.max(0.05)).min(1.0),
            drift_deg: right.atan2(forward.abs().max(1e-3)).to_degrees(),
            load_share: [share(super::FRONT), share(REAR)],
            throttle: throttle.clamp(0.0, 1.0),
        }
    }
}

impl Bike {
    /// The load on wheel `index` over its load at rest (the weight on it with the bike standing).
    pub(super) fn load_share(&self, index: usize) -> f32 {
        let total = self.body.mass() * 9.81;
        let wheelbase = self.spec.chassis.wheelbase.max(f32::EPSILON);
        let front_share = (0.5 + self.body.cog().z / wheelbase).clamp(0.0, 1.0);
        let rest = match index {
            super::FRONT => front_share * total,
            _ => (1.0 - front_share) * total,
        };
        match rest > 1e-3 {
            true => self.wheels[index].load / rest,
            false => 0.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn map(input: [f32; 2], output: [f32; 2]) -> RangeMap {
        RangeMap { input, output }
    }

    /// The rear of the KTM's ABS element and its speed tuning (`Motard_ABSVehicleElement`).
    fn ktm() -> BrakeReduction {
        BrakeReduction {
            speed: map([10.0, 25.0], [0.0, 1.0]),
            front: WheelReduction {
                intensity: 1.0,
                lean: map([0.0, 0.7], [0.0, 0.2]),
                drift: map([0.0, 30.0], [0.0, 1.0]),
                load: map([0.0, 1.0], [1.0, 0.0]),
                throttle: RangeMap::default(),
            },
            rear: WheelReduction {
                intensity: 1.0,
                lean: map([0.0, 0.9], [0.0, 0.5]),
                drift: map([0.0, 35.0], [0.0, 0.2]),
                load: map([1.0, 0.0], [0.0, 0.8]),
                throttle: map([0.0, 0.05], [1.0, 0.0]),
            },
        }
    }

    fn input(speed_kmh: f32, lean: f32, drift: f32, load: f32, throttle: f32) -> ReductionInput {
        ReductionInput { speed_kmh, lean_share: lean, drift_deg: drift, load_share: [load, load], throttle }
    }

    #[test]
    fn the_front_has_no_reduction_because_it_states_no_throttle_term() {
        let reduction = ktm();
        assert_eq!(reduction.target(false, &input(80.0, 1.0, 40.0, 0.0, 0.0)), 0.0);
    }

    #[test]
    fn the_rear_loses_its_brake_when_it_unloads_leans_or_drifts_with_the_throttle_shut() {
        let reduction = ktm();
        // Upright, loaded and straight: nothing.
        assert_eq!(reduction.target(true, &input(80.0, 0.0, 0.0, 1.0, 0.0)), 0.0);
        // Half the rest load: 0.4 from the load alone.
        assert!((reduction.target(true, &input(80.0, 0.0, 0.0, 0.5, 0.0)) - 0.4).abs() < 1e-6);
        // Lean 0.45 of the limit adds 0.25, a drift of 7 deg adds 0.04; the sum clamps to 1.
        let sum = reduction.target(true, &input(80.0, 0.45, 7.0, 0.5, 0.0));
        assert!((sum - (0.4 + 0.25 + 0.04)).abs() < 1e-5, "{sum}");
        assert_eq!(reduction.target(true, &input(80.0, 0.9, 35.0, 0.0, 0.0)), 1.0);
        // The sign of the lean and of the drift does not matter.
        assert_eq!(
            reduction.target(true, &input(80.0, -0.45, -7.0, 0.5, 0.0)),
            reduction.target(true, &input(80.0, 0.45, 7.0, 0.5, 0.0))
        );
    }

    #[test]
    fn the_throttle_and_a_low_speed_switch_it_off() {
        let reduction = ktm();
        let loose = input(80.0, 0.0, 0.0, 0.5, 0.0);
        assert!((reduction.target(true, &input(80.0, 0.0, 0.0, 0.5, 0.025)) - 0.2).abs() < 1e-6);
        assert_eq!(reduction.target(true, &input(80.0, 0.0, 0.0, 0.5, 0.05)), 0.0);
        assert_eq!(reduction.target(true, &input(10.0, 0.0, 0.0, 0.5, 0.0)), 0.0);
        assert!((reduction.target(true, &ReductionInput { speed_kmh: 17.5, ..loose }) - 0.2).abs() < 1e-6);
        assert_eq!(BrakeReduction::default().target(true, &loose), 0.0);
    }

    #[test]
    fn the_limiter_rises_fast_and_falls_at_its_rate() {
        assert_eq!(rate_limit(0.0, 0.7, 10_000.0, 1_000.0, 1.0 / 60.0), 0.7);
        assert!((rate_limit(0.7, 0.0, 10_000.0, 1_000.0, 1.0 / 600.0) - 0.0).abs() < 1e-6);
        assert!((rate_limit(0.7, 0.0, 10_000.0, 6.0, 1.0 / 60.0) - 0.6).abs() < 1e-6);
        assert_eq!(rate_limit(0.7, 0.2, 10_000.0, 0.0, 1.0 / 60.0), 0.2);
    }
}
