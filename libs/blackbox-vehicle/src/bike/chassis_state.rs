//! What the bike keeps from step to step apart from its filters: the day it rides in (air and road temperature) and the running
//! state of the fall tests (`docs/specs/ride6-chassis-round6.md`).

use crate::bike::choices;
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

/// What a collision has added to the controllers' help and has not yet faded (`choices::COLLISION_LEAN_HELP`): the addition to the
/// lean help's factor and to the stability help's, each 0 to 2.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CollisionHelp {
    pub lean: f32,
    pub stability: f32,
    /// The stability addition's fade rate (per second), that of the collision that set it.
    stability_fade: f32,
}

impl CollisionHelp {
    /// A collision: the additions rise to their values if they are below them; a frontal collision gives the stability help its own.
    pub fn collide(&mut self, frontal: bool) {
        self.lean = self.lean.max(choices::COLLISION_LEAN_HELP.0);
        let (addition, fade) = match frontal {
            true => choices::COLLISION_STABILITY_HELP_FRONTAL,
            false => choices::COLLISION_STABILITY_HELP,
        };
        if addition >= self.stability {
            self.stability_fade = fade;
        }
        self.stability = self.stability.max(addition);
    }

    /// The additions fade for `dt` seconds.
    pub fn fade(&mut self, dt: f32) {
        self.lean = (self.lean - choices::COLLISION_LEAN_HELP.1 * dt).max(0.0);
        self.stability = (self.stability - self.stability_fade * dt).max(0.0);
    }
}

/// The running state of the fall tests.
#[derive(Clone, Copy, Debug)]
pub struct ChassisState {
    /// What a collision added to the controllers' help.
    pub collision_help: CollisionHelp,
    pub environment: Environment,
    /// The lean (rad) beyond which the bike has fallen on the ground now: it follows the front tyre's grip when the spec says so.
    pub fall_lean: f32,
    /// How long the lean has been beyond that limit (s).
    pub fall_timer: f32,
}

impl ChassisState {
    pub fn new(fall: &FallSpec) -> Self {
        Self {
            collision_help: CollisionHelp::default(),
            environment: Environment::default(),
            fall_lean: fall.lean,
            fall_timer: 0.0,
        }
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
