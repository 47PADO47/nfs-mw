//! The physics side of the engine sound: the telemetry turned into the sound scale. Spec:
//! `docs/specs/engine-sound.md` §4.1.

use crate::input::{CarInput, GEAR_NEUTRAL};
use crate::math::{bezier_y, finite, slew};
use crate::tuning::EngineTuning;

/// Above this throttle (percent) the car counts as accelerating.
const ACCELERATING_THROTTLE: f32 = 30.0;
/// `PhysicsTRQ` follows the throttle by at most this much per tick.
const TORQUE_STEP: f32 = 100.0;

#[derive(Debug, Clone, Copy)]
pub(crate) struct Physics {
    /// `PhysicsRPM` on the 1000 to 10000 sound scale.
    pub rpm: f32,
    /// `PhysicsTRQ`: the throttle in percent, slewed.
    pub torque: f32,
    /// Throttle in percent, this tick and the one before.
    pub throttle: f32,
    pub old_throttle: f32,
    pub accelerating: bool,
    pub gear: i32,
    pub last_gear: i32,
    /// The physics RPM fraction before the remap (`GetRPMPct`).
    pub rpm_pct: f32,
    seen_gear: bool,
}

impl Physics {
    pub fn new() -> Self {
        Self {
            rpm: 1000.0,
            torque: 0.0,
            throttle: 0.0,
            old_throttle: 0.0,
            accelerating: false,
            gear: GEAR_NEUTRAL,
            last_gear: GEAR_NEUTRAL,
            rpm_pct: 0.0,
            seen_gear: false,
        }
    }

    /// True on the tick the gear changed (the packet's gear-shift flag).
    pub fn gear_changed(&self) -> bool {
        self.gear != self.last_gear
    }

    pub fn update(&mut self, input: &CarInput, tuning: &EngineTuning, remap: bool) {
        self.last_gear = self.gear;
        self.gear = input.gear;
        if !self.seen_gear {
            self.seen_gear = true;
            self.last_gear = self.gear;
        }
        self.old_throttle = self.throttle;
        self.throttle = finite(input.throttle).clamp(0.0, 1.0) * 100.0;
        if self.throttle > ACCELERATING_THROTTLE {
            self.accelerating = true;
        }
        if self.throttle <= ACCELERATING_THROTTLE {
            self.accelerating = false;
        }
        self.torque = slew(self.torque, self.throttle, TORQUE_STEP);
        self.rpm_pct = finite(input.rpm_pct).clamp(0.0, 1.0);
        let mut fraction = self.rpm_pct;
        if remap {
            fraction = bezier_y(tuning.rpm_map, fraction);
        }
        self.rpm = finite(fraction).clamp(0.0, 1.0) * 9000.0 + 1000.0;
    }
}
