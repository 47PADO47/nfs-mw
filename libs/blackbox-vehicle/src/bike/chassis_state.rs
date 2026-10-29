//! What the bike keeps from step to step apart from its filters: the day it rides in (air and road temperature) and the running
//! state of the fall tests (`docs/specs/ride6-chassis-round6.md`).

use crate::bike::spec::FallSpec;

/// The day: the temperature of the air and of the road (deg C) the tyres cool towards. The game that hosts the bike sets them
/// from its track and weather (`Bike::set_environment`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Environment {
    pub air: f32,
    pub asphalt: f32,
}

impl Default for Environment {
    /// A clear morning at the median of the reference game's track temperature table: 23 deg C air, 28 deg C road
    /// (`docs/provenance/ride6-physics-B.md`). Only a stand-in until the host says what the day is.
    fn default() -> Self {
        Self { air: 23.0, asphalt: 28.0 }
    }
}

/// The running state of the fall tests.
#[derive(Clone, Copy, Debug)]
pub struct ChassisState {
    pub environment: Environment,
    /// The lean (rad) beyond which the bike has fallen on the ground now: it follows the front tyre's grip when the spec says so.
    pub fall_lean: f32,
    /// How long the lean has been beyond that limit (s).
    pub fall_timer: f32,
}

impl ChassisState {
    pub fn new(fall: &FallSpec) -> Self {
        Self { environment: Environment::default(), fall_lean: fall.lean, fall_timer: 0.0 }
    }

    /// Forget the lean history (a bike just placed); the day stays.
    pub fn reset(&mut self, fall: &FallSpec) {
        *self = Self { environment: self.environment, ..Self::new(fall) };
    }

    /// Move the fall lean limit towards the one the grip allows (`atan(mu) x tolerance`, rad): a fraction `rise` of the gap when it
    /// is above, `fall` when below, each step.
    pub fn follow_grip(&mut self, fall: &FallSpec, allowed: f32) {
        let extra = &fall.extra;
        if !extra.lean_by_grip || !allowed.is_finite() {
            return;
        }
        let gap = allowed - self.fall_lean;
        let share = match gap > 0.0 {
            true => extra.lean_rise,
            false => extra.lean_fall,
        };
        self.fall_lean += gap * share;
    }
}
