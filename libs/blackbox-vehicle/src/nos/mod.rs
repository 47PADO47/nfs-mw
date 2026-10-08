//! Nitrous oxide: tank, burn, recharge and the torque multiplier. Spec section 4 of
//! `docs/specs/vehicle-input-induction-brakes.md`.

use crate::drivetrain::GEAR_FIRST;
use crate::math::{lerp, ramp};

/// Nitrous parameters (the attribute class `nos`). Speeds in mph, times in seconds.
#[derive(Clone, Copy, Debug, Default)]
pub struct NosSpec {
    /// Seconds of full burn of a full tank.
    pub nos_capacity: f32,
    /// Extra torque multiplier minus one.
    pub torque_boost: f32,
    /// Delay after releasing the button before recharging may start.
    pub nos_disengage: f32,
    /// Slowest and fastest recharge, in seconds for a full tank.
    pub recharge_min: f32,
    pub recharge_max: f32,
    /// Speeds (mph) mapping onto `recharge_min..recharge_max`.
    pub recharge_min_speed: f32,
    pub recharge_max_speed: f32,
}

impl NosSpec {
    /// The car has nitrous when both the capacity and the boost are positive.
    pub fn present(&self) -> bool {
        self.nos_capacity > 0.0 && self.torque_boost > 0.0
    }
}

/// Inputs of one nitrous update.
#[derive(Clone, Copy, Debug)]
pub struct NosInput {
    pub dt: f32,
    /// The nitrous button is held.
    pub held: bool,
    /// Gear id (reverse 0, neutral 1, first 2, ...).
    pub gear: usize,
    pub throttle: f32,
    pub speed_mph: f32,
    /// Engine blown: no nitrous.
    pub blown: bool,
    /// Tuning slider in [-1, 1].
    pub tuning: f32,
}

/// Nitrous state.
#[derive(Clone, Copy, Debug)]
pub struct Nos {
    /// Tank level, 0..1.
    pub capacity: f32,
    /// 0..1, reaches 1 while burning and decays after release.
    pub engaged: f32,
    /// Torque multiplier (1 = off).
    pub boost: f32,
}

impl Nos {
    pub fn new(spec: &NosSpec) -> Self {
        Self { capacity: if spec.present() { 1.0 } else { 0.0 }, engaged: 0.0, boost: 1.0 }
    }

    /// True while the nitrous is fully on.
    pub fn is_engaged(&self) -> bool {
        self.engaged >= 1.0
    }

    pub fn update(&mut self, spec: &NosSpec, i: &NosInput) {
        let t = i.tuning.clamp(-1.0, 1.0);
        let boost_n = spec.torque_boost * (1.0 + 0.25 * t);
        let capacity_n = spec.nos_capacity * (1.0 - 0.25 * t);
        let was_engaged = self.engaged > 0.0;

        let mut engaged = i.held;
        if i.gear < GEAR_FIRST || i.throttle <= 0.0 || i.blown {
            engaged = false;
        }
        if (i.speed_mph < 10.0 && !was_engaged) || (i.speed_mph < 5.0 && was_engaged) {
            engaged = false;
        }

        if !spec.present() || capacity_n <= 0.0 {
            *self = Self { capacity: 0.0, engaged: 0.0, boost: 1.0 };
            return;
        }
        let recharge = if i.speed_mph >= spec.recharge_min_speed && i.gear >= GEAR_FIRST {
            lerp(
                spec.recharge_min,
                spec.recharge_max,
                ramp(i.speed_mph, spec.recharge_min_speed, spec.recharge_max_speed),
            )
        } else {
            0.0
        };
        if engaged && self.capacity > 0.0 {
            self.capacity = (self.capacity - i.dt / capacity_n).max(0.0);
            self.boost = 1.0 + boost_n;
            self.engaged = 1.0;
        } else if self.engaged > 0.0 && spec.nos_disengage > 0.0 {
            self.engaged = (self.engaged - i.dt / spec.nos_disengage).max(0.0);
            self.boost = 1.0;
        } else if self.capacity < 1.0 && recharge > 0.0 {
            self.capacity = (self.capacity + i.dt / recharge).min(1.0);
            self.engaged = 0.0;
            self.boost = 1.0;
        } else {
            self.engaged = 0.0;
            self.boost = 1.0;
        }
    }
}

#[cfg(test)]
mod tests;
