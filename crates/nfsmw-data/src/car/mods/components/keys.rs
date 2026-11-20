//! The numbers a component may change on a bike: the parameter keys the game binds, and what they act on.
//!
//! A key is the path of the number in the bike's own manifest (`[bike.engine] peak_torque_nm`,
//! docs/specs/vehicle-components/manifest.md "Parameter keys"). This is the one list of them; a component that names another
//! key is reported when the car is published and the key changes nothing.

use blackbox_components::{ParamTable, Used};
use blackbox_vehicle::bike::BikeSpec;

/// The peak torque of the engine (N m), `[bike.engine] peak_torque_nm`.
pub const PEAK_TORQUE: &str = "bike.engine.peak_torque_nm";
/// The mass of the bike without the rider (kg), `[bike.chassis] mass_kg`.
pub const MASS: &str = "bike.chassis.mass_kg";
/// The force of the front brake at the tyre patch (N), `[bike.brakes] front_force_n`.
pub const FRONT_BRAKE: &str = "bike.brakes.front_force_n";
/// The force of the rear brake at the tyre patch (N), `[bike.brakes] rear_force_n`.
pub const REAR_BRAKE: &str = "bike.brakes.rear_force_n";

/// Every key the game binds on a bike.
pub const KEYS: [&str; 4] = [PEAK_TORQUE, MASS, FRONT_BRAKE, REAR_BRAKE];

/// The least and the most of its own value a stack of modifiers may leave a number: the physics the bike data was made for
/// does not survive more, and a mod that stacks beyond it is told by the log, not obeyed.
const LOWEST: f64 = 0.5;
const HIGHEST: f64 = 2.0;

/// What the components in `used` ask for that the game does not bind on a bike: one line each, for the log.
pub fn unbound(used: &Used) -> Vec<String> {
    let mut lines: Vec<String> = used
        .params
        .iter()
        .filter(|key| !KEYS.contains(&key.as_str()))
        .map(|key| format!("the parameter {key} is not one of a bike's ({})", KEYS.join(", ")))
        .collect();
    lines.extend(used.behaviours.keys().map(|domain| format!("the behaviour domain {domain} is not bound")));
    lines.extend(used.surfaces.iter().map(|target| format!("the surface {target} is not bound")));
    lines
}

fn bounded(params: &ParamTable, key: &str, base: f32) -> f32 {
    let base = f64::from(base);
    params.apply(key, base).clamp(base * LOWEST, base * HIGHEST) as f32
}

/// The bike `spec` with the number modifiers of `params` applied. Keys that no component touches leave the bike as it is.
pub fn apply(spec: &BikeSpec, params: &ParamTable) -> BikeSpec {
    let mut bike = spec.clone();
    bike.engine.peak_torque = bounded(params, PEAK_TORQUE, spec.engine.peak_torque);
    bike.chassis.mass = bounded(params, MASS, spec.chassis.mass);
    bike.brakes.front_force = bounded(params, FRONT_BRAKE, spec.brakes.front_force);
    bike.brakes.rear_force = bounded(params, REAR_BRAKE, spec.brakes.rear_force);
    bike
}
