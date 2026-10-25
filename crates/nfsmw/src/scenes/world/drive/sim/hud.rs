//! What the HUD reads of the car beyond the readout's telemetry: the nitrous burn, the shift point, the engine load, the
//! lateral acceleration, and the condition of the tyres and the body (`hud/state/vehicle.rs`).
//!
//! The lateral acceleration is the centripetal term `ω × v` along the body's right axis, the acceleration of a body that
//! turns at the rate `ω` while it moves at `v`. It is exact for a steady turn and needs no history, where differencing
//! the velocity between steps would also pick up every contact impulse and wall hit as a spike. A car with every wheel
//! in the air reads 0: it has no grip to turn with, and the HUD does not show a turn that is not happening.

use blackbox_vehicle::drivetrain::GEAR_FIRST;
use blackbox_vehicle::tires::TireState;
use glam::{Mat3, Vec3};

use super::CarSim;
use crate::hud::{HudTelemetry, TireCondition, VehicleCondition, cosmetic_level};

/// Standard gravity, m/s², the unit of the lateral acceleration.
const G: f32 = 9.806_65;

impl CarSim {
    /// The extra telemetry of the car for the HUD.
    pub fn hud_telemetry(&self) -> HudTelemetry {
        let vehicle = &self.vehicle;
        let powertrain = vehicle.powertrain();
        HudTelemetry {
            nos_burning: vehicle.nos_burning(),
            shift_up_rpm: shift_up_rpm(&powertrain.shift_points().up, powertrain.gear(), powertrain.top_gear()),
            throttle: powertrain.throttle().clamp(0.0, 1.0),
            lateral_g: lateral_g(
                vehicle.rotation(),
                vehicle.linear_velocity(),
                vehicle.angular_velocity(),
                vehicle.wheels_on_ground() > 0,
            ),
        }
    }

    /// The car's condition for the HUD: the body damage (when the car has a damage model), the tyres, and whether it
    /// still drives.
    pub fn hud_condition(&self) -> VehicleCondition {
        let damage = &self.vehicle.tire_damage;
        VehicleCondition {
            cosmetic: self.events.as_ref().map(|events| cosmetic_level(&events.damage)),
            tires: std::array::from_fn(|wheel| tire_condition(damage.state(wheel))),
            disabled: self.vehicle.disabled,
            destroyed: self.events.as_ref().is_some_and(|events| events.is_destroyed()),
        }
    }
}

/// The HUD's reading of a tyre's state, without the fuse.
fn tire_condition(state: TireState) -> TireCondition {
    match state {
        TireState::None => TireCondition::Sound,
        TireState::Punctured(_) => TireCondition::Punctured,
        TireState::Blown => TireCondition::Blown,
    }
}

/// The rpm of the up-shift from gear id `gear` (0 reverse, 1 neutral, 2 first, ...): `up` is the powertrain's table by
/// gear id and `top` the top gear's id. `None` in reverse, in neutral, in the top gear and for a gear without a point.
fn shift_up_rpm(up: &[f32], gear: usize, top: usize) -> Option<f32> {
    if gear < GEAR_FIRST || gear >= top {
        return None;
    }
    up.get(gear).copied().filter(|rpm| *rpm > 0.0)
}

/// The sideways acceleration in g, positive towards the body's right (`+x` in physics space), of a body turned by
/// `rotation` (body to world) that moves at `velocity` and turns at `angular` (world axes, rad/s). 0 when not
/// `grounded`. See the module for why it is `ω × v`.
fn lateral_g(rotation: Mat3, velocity: Vec3, angular: Vec3, grounded: bool) -> f32 {
    if !grounded {
        return 0.0;
    }
    let right = rotation * Vec3::X;
    angular.cross(velocity).dot(right) / G
}

#[cfg(test)]
#[path = "hud_tests.rs"]
mod tests;
