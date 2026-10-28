//! `[bike.wheelie.help]`: the rider help around a wheelie or a stoppie beyond the plain lift ranges of `wheelie.rs`: the
//! launch blend of the throttle cut, which interventions act on the engine, the intensity and the auto-rider that moves
//! the rider's weight (`docs/specs/ride6-tricks.md`, sections 4 and 5). The fields an asset leaves out take the constructor
//! defaults of `URiderHelpVehicleElement` (`0x1450fda20` of build `5.6.1-1697004`).

use anyhow::Result;

use super::curve::{runtime_curve, simplify};
use super::engine::MAX_CURVE_POINTS;
use super::node::Node;
use super::toml_text::Toml;
use super::units::{metres_per_second, radians};
use super::vehicle::Vehicle;

/// The constructor defaults of the rider help that the table carries.
mod default {
    /// `AutoRiderLaunchSpeedThreshold`, km/h: 0 switches the launch blend off.
    pub const LAUNCH_SPEED_KMH: f64 = 0.0;
    /// `LaunchWheelieThrottleControl`, `AutoRiderForcedStoppieBrakeControl`, `AutoRiderWheelieWeightControl` and
    /// `AutoRiderStoppieWeightControl`, degrees.
    pub const WIDE_RANGE_DEG: [f64; 2] = [30.0, 90.0];
    pub const MULTIPLIER: f64 = 1.0;
    pub const INTENSITY: f64 = 1.0;
    /// `WeightLongitudinalInputDeadzone`.
    pub const WEIGHT_DEADZONE: f64 = 0.8;
    pub const LOOKAHEAD: f64 = 0.0;
    /// `PitchSpeedWeightControl`: a descending range, which is always 0.
    pub const PITCH_SPEED_RANGE: [f64; 2] = [0.0, -1.0];
    pub const ENGAGE_SPEED: f64 = 0.0;
}

fn radians_pair(range: [f64; 2]) -> [f64; 2] {
    [radians(range[0]), radians(range[1])]
}

/// A curve of the rider help (`FRuntimeFloatCurve`), simplified; empty when the asset sets none.
fn curve_of(vehicle: &Vehicle, help: &Node<'_>, key: &str) -> Result<Vec<[f64; 2]>> {
    let Some(node) = help.get(key) else { return Ok(Vec::new()) };
    let Some(curve) = runtime_curve(vehicle, &node)? else { return Ok(Vec::new()) };
    Ok(simplify(curve.polyline(), MAX_CURVE_POINTS))
}

/// Write the table; nothing for a rider help that is switched off.
pub fn write(t: &mut Toml, vehicle: &Vehicle, help: &Node<'_>) -> Result<()> {
    if !help.flag_or("bEnableAutoRider", false)? {
        return Ok(());
    }
    let engine_throttle = help.flag_or("bApplyEngineThrottleModulation", false)?;
    t.table("bike.wheelie.help");
    t.num(
        "launch_speed_mps",
        metres_per_second(help.f_or("AutoRiderLaunchSpeedThreshold", default::LAUNCH_SPEED_KMH)?),
    );
    t.nums(
        "launch_throttle_pitch_rad",
        &radians_pair(help.xy_or("LaunchWheelieThrottleControl", default::WIDE_RANGE_DEG)?),
    );
    t.num("launch_throttle_multiplier", help.f_or("LaunchThrottleControlMultiplier", default::MULTIPLIER)?);
    t.num("throttle_multiplier", help.f_or("AutoRiderThrottleControlMultiplier", default::MULTIPLIER)?);
    t.nums(
        "forced_brake_pitch_rad",
        &radians_pair(help.xy_or("AutoRiderForcedStoppieBrakeControl", default::WIDE_RANGE_DEG)?),
    );
    t.num("brake_multiplier", help.f_or("AutoRiderBrakeControlMultiplier", default::MULTIPLIER)?);
    t.flag(
        "throttle_pitch_electronic",
        engine_throttle && help.flag_or("bEnablePitchAngleElectronicThrottleModulation", false)?,
    );
    t.flag(
        "throttle_speed_electronic",
        engine_throttle && help.flag_or("bEnablePitchSpeedElectronicThrottleModulation", false)?,
    );
    t.flag("brake_pitch_electronic", help.flag_or("bEnablePitchAngleElectronicBrakeModulation", false)?);
    t.flag("brake_speed_electronic", help.flag_or("bEnablePitchSpeedElectronicBrakeModulation", false)?);
    t.num("intensity", help.f_or("AutoRiderIntensity", default::INTENSITY)?);
    t.num("weight_deadzone", help.f_or("WeightLongitudinalInputDeadzone", default::WEIGHT_DEADZONE)?);
    t.num("lookahead_s", help.f_or("AutoRiderLookaheadTime", default::LOOKAHEAD)?);
    if help.flag_or("bEnableGForceWeightControl", false)? {
        t.note("the auto-rider's g-force weight control is not modelled: the pitch speed control is used");
    }
    if !help.flag_or("bEnableRiderHelpWeightControl", false)? {
        return Ok(());
    }
    t.table("bike.wheelie.help.auto_rider");
    t.nums("wheelie_range_rad", &radians_pair(help.xy_or("AutoRiderWheelieWeightControl", default::WIDE_RANGE_DEG)?));
    t.nums("stoppie_range_rad", &radians_pair(help.xy_or("AutoRiderStoppieWeightControl", default::WIDE_RANGE_DEG)?));
    t.num("multiplier", help.f_or("AutoRiderWeightControlMultiplier", default::MULTIPLIER)?);
    t.nums("pitch_speed_range", &help.xy_or("PitchSpeedWeightControl", default::PITCH_SPEED_RANGE)?);
    let throttle = curve_of(vehicle, help, "AutoRiderWeightControlOnThrottleTuning")?;
    if !throttle.is_empty() {
        t.pairs("throttle_curve", &throttle);
    }
    t.num("throttle_gain", help.f_or("AutoRiderWeightControlOnThrottleMultiplier", default::MULTIPLIER)?);
    let brake = curve_of(vehicle, help, "AutoRiderWeightControlOnBrakeTuning")?;
    if !brake.is_empty() {
        t.pairs("brake_curve", &brake);
    }
    t.num("brake_gain", help.f_or("AutoRiderWeightControlOnBrakeMultiplier", default::MULTIPLIER)?);
    t.num("engage_speed", help.f_or("AutoRiderPositionEngageSpeed", default::ENGAGE_SPEED)?);
    t.num("manual_engage_speed", help.f_or("ManualPositionEngageSpeed", default::ENGAGE_SPEED)?);
    Ok(())
}
