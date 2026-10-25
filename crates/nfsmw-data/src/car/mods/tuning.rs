//! How a mod car drives compared with its clone (the `[performance]` table of the car manifest,
//! docs/modding/car-customization.md).
//!
//! The numbers are multipliers of the clone's physics. They are applied last, after the upgrades the player bought, so a
//! car that is 10 percent stronger stays 10 percent stronger at every stage.

use blackbox_vehicle::VehicleSpec;
use blackbox_vehicle::nos::NosSpec;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Tuning {
    /// Engine torque, at every rpm.
    pub power: f32,
    /// Body mass.
    pub mass: f32,
    /// Tyre grip: the lateral curve and both friction coefficients.
    pub grip: f32,
    /// Brake torque of both axles.
    pub brakes: f32,
    /// The car has the clone's first nitrous kit from the factory.
    pub nitrous: bool,
    /// Multiplier of the nitrous torque boost.
    pub nitrous_power: f32,
}

impl Default for Tuning {
    fn default() -> Self {
        Self { power: 1.0, mass: 1.0, grip: 1.0, brakes: 1.0, nitrous: false, nitrous_power: 1.0 }
    }
}

/// The nitrous of a car whose clone has no nitrous kit to copy: seconds of burn, torque added, recharge by speed (the
/// values of a typical first kit).
pub const TYPICAL_NITROUS: NosSpec = NosSpec {
    nos_capacity: 5.0,
    torque_boost: 0.5,
    nos_disengage: 0.5,
    recharge_min: 20.0,
    recharge_max: 5.0,
    recharge_min_speed: 20.0,
    recharge_max_speed: 100.0,
};

impl Tuning {
    /// Whether the car drives exactly as its clone does.
    pub fn is_stock(&self) -> bool {
        *self == Self::default()
    }

    /// Scale `spec`. The factory nitrous is not here: it needs the clone's records (`PhysicsData::car`).
    pub fn apply(&self, spec: &mut VehicleSpec) {
        spec.engine.torque.iter_mut().for_each(|torque| *torque *= self.power);
        spec.mass *= self.mass;
        for axle in 0..2 {
            spec.tires.grip_scale[axle] *= self.grip;
            spec.tires.static_grip[axle] *= self.grip;
            spec.tires.dynamic_grip[axle] *= self.grip;
            spec.brakes.brakes[axle] *= self.brakes;
        }
        spec.nos.torque_boost *= self.nitrous_power;
    }
}
