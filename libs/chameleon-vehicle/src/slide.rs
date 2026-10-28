//! What a car's bundle says about how it slides: the numbers behind the 2012 game's drift physics beyond the entry and exit
//! of [`crate::drift`], and the handbrake. Spec: `docs/formats/chameleon-drift.md#the-sliding-physics`.
//!
//! The handling definition has a `DriftBehaviour` of six structures (`Other`, `Yaw_torque`, `Side_force`, `Drift_scale`,
//! `Drift_exit`, `Drift_trigger`) and a `HandbrakeAbility`. Field names are hashed, so fields are read at fixed offsets and
//! their meaning is read off the values over the 93 cars (their ranges, their relation to the car's mass and layout); the
//! doc tags every one **[unconfirmed]**: no code that reads them was found in the executable.

use chameleon_bundle::Bundle;

use crate::drift::{DriftPhysics, read_drift_physics};
use crate::error::{Error, Result};
use crate::genesys::{Object, TypeNames, find_object};

/// How a car slides, from its handling definition.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DriftHandling {
    /// When a slide is a drift: the speed, the entry angle and the exit angle.
    pub physics: DriftPhysics,
    /// The largest slip angle the drift physics works to, degrees (`DriftBehaviour.Other +0x18`; 40 in 71 cars, 45 in 19,
    /// 55 in 1, 0 in 2) **[unconfirmed: the meaning]**.
    pub max_angle: f32,
    /// The yaw torque of the drift physics, N m (`DriftBehaviour.Yaw_torque +0x24`; 6000 to 60000, 41500 median; no relation
    /// to the car's mass) **[unconfirmed: the unit and the meaning]**.
    pub yaw_torque_nm: f32,
    /// The three smaller torques that follow it (`+0x30`, `+0x34`, `+0x38`; about 0.35, 0.2 and 0.04 of the first)
    /// **[unconfirmed]**.
    pub yaw_torque_steps: [f32; 3],
    /// The handbrake's brake torque at the rear wheels, N m (`HandbrakeAbility +0x10`; 300 to 1800, 1200 median, the same
    /// size as the cars' brake torques) **[unconfirmed]**.
    pub handbrake_torque_nm: f32,
    /// The second handbrake number (`HandbrakeAbility +0x14`; 200 to 750, 400 median) **[unconfirmed]**.
    pub handbrake_second_nm: f32,
}

/// The drift handling of the car in `bundle`.
pub fn read_drift_handling(bundle: &Bundle) -> Result<DriftHandling> {
    let physics = read_drift_physics(bundle)?;
    let types = TypeNames::read(bundle)?;
    let handling = find_object(bundle, &types, "Genesys.Gen.HandlingBehaviour")?;
    handling_of(&handling, physics)
}

/// The numbers of a handling object.
fn handling_of(handling: &Object, physics: DriftPhysics) -> Result<DriftHandling> {
    let other = at(handling, "Genesys.Gen.DriftBehaviour.Other")?;
    let yaw = at(handling, "Genesys.Gen.DriftBehaviour.Yaw_torque")?;
    let handbrake = at(handling, "Genesys.Gen.HandbrakeAbility")?;
    let result = DriftHandling {
        physics,
        max_angle: handling.f32(other + 0x18)?,
        yaw_torque_nm: handling.f32(yaw + 0x24)?,
        yaw_torque_steps: [handling.f32(yaw + 0x30)?, handling.f32(yaw + 0x34)?, handling.f32(yaw + 0x38)?],
        handbrake_torque_nm: handling.f32(handbrake + 0x10)?,
        handbrake_second_nm: handling.f32(handbrake + 0x14)?,
    };
    let numbers = [
        result.max_angle,
        result.yaw_torque_nm,
        result.yaw_torque_steps[0],
        result.yaw_torque_steps[1],
        result.yaw_torque_steps[2],
        result.handbrake_torque_nm,
        result.handbrake_second_nm,
    ];
    if !numbers.iter().all(|value| value.is_finite() && *value >= 0.0) || result.max_angle > 90.0 {
        return Err(Error::Spec(format!("the drift handling does not read sensibly: {result:?}")));
    }
    Ok(result)
}

fn at(object: &Object, name: &str) -> Result<usize> {
    object.at(name, 0).ok_or_else(|| Error::Spec(format!("no {name} in the object")))
}

#[cfg(test)]
#[path = "slide_tests.rs"]
mod tests;
