//! `[bike.burnout]`: what the rear tyre does while the rider holds the front brake and the throttle with the bike standing
//! (`docs/specs/ride6-burnout.md`).
//!
//! The input handler class holds `BurnoutGripMultiplier` and `BurnoutSlipCorrectionMultiplier`, which the handler writes into the rear wheel
//! in a burnout, and `BurnoutMinSpeed`, the speed (km/h) under which a burnout is possible. The handler assets of the install set the
//! grip multiplier to 0.05 (0.15 for the supermoto handler), the slip correction only for the supermoto (0.9) and the speed never; the
//! rest is the native constructor's (`UBikeInputHandler`, build `5.6.1-1697004`, `0x1450bbf80`): a grip multiplier of 1, a slip correction
//! of 0 and a minimum speed of 5 km/h (`0x1450bd6d0`). The table is written for every bike, so a reader can see which handler it came from.
//!
//! It also carries the handler's `BurnoutLeanLimit` (degrees; the lean limit while the flag stands, 18 for the road handlers, 10 for the
//! supermoto one), the traction control element's `bTCSLowSpeedModulationForBurnout` (the constructor sets it, `0x1450fe630`, and no asset
//! overrides it) and the burnout help force (`[bike.burnout.help]`): `EnableBurnoutHelpForce` (off for the supermotos), its factor, yaw
//! rate (degrees per second in the data, radians here), stick filter time, rpm range and the collision reduction, with the constructor's
//! values (`0x1450bbf80`) where the asset says nothing.

use anyhow::Result;

use super::chassis::handler;
use super::toml_text::Toml;
use super::vehicle::Vehicle;

/// The native constructor's `BurnoutGripMultiplier`, `BurnoutSlipCorrectionMultiplier` and `BurnoutMinSpeed` (km/h).
pub const DEFAULT_GRIP: f64 = 1.0;
pub const DEFAULT_SLIP_CORRECTION: f64 = 0.0;
pub const DEFAULT_SPEED_LIMIT_KMH: f64 = 5.0;
/// The native constructors' help force: `EnableBurnoutHelpForce` true, `BurnoutHelpForceFactor` 1, `BurnoutDesiredAngularSpeed` 45 degrees per
/// second, `BurnoutSteerFilterTime` 1 s, `BurnoutHelpForceRPMPercentageRange` 0.75 to 0.95, `BurnoutHelpReduction` 1, its duration 0.1 s and
/// `BurnoutHelpRaiseRate` 10 (`0x1450bbf80`), and the traction control's `bTCSLowSpeedModulationForBurnout` true (`0x1450fe630`).
pub const DEFAULT_HELP_FACTOR: f64 = 1.0;
pub const DEFAULT_YAW_RATE_DEG: f64 = 45.0;
pub const DEFAULT_STEER_FILTER_S: f64 = 1.0;
pub const DEFAULT_RPM_RANGE: [f64; 2] = [0.75, 0.95];
pub const DEFAULT_REDUCTION: f64 = 1.0;
pub const DEFAULT_REDUCTION_S: f64 = 0.1;
pub const DEFAULT_RAISE_RATE: f64 = 10.0;

pub fn write(t: &mut Toml, vehicle: &Vehicle) -> Result<()> {
    let handler = handler(vehicle)?;
    t.table("bike.burnout");
    t.num("grip", handler.f_or("BurnoutGripMultiplier", DEFAULT_GRIP)?);
    t.num("slip_correction", handler.f_or("BurnoutSlipCorrectionMultiplier", DEFAULT_SLIP_CORRECTION)?);
    t.num("speed_limit_kmh", handler.f_or("BurnoutMinSpeed", DEFAULT_SPEED_LIMIT_KMH)?);
    t.num("lean_limit_rad", handler.f_or("BurnoutLeanLimit", 0.0)?.to_radians());
    let tcs = match vehicle.element("TCS")? {
        Some(control) => control.flag_or("bTCSLowSpeedModulationForBurnout", true)?,
        None => true,
    };
    t.flag("tcs_low_speed_modulation", tcs);
    t.table("bike.burnout.help");
    t.flag("enabled", handler.flag_or("EnableBurnoutHelpForce", true)?);
    t.num("factor", handler.f_or("BurnoutHelpForceFactor", DEFAULT_HELP_FACTOR)?);
    t.num("yaw_rate_rad_s", handler.f_or("BurnoutDesiredAngularSpeed", DEFAULT_YAW_RATE_DEG)?.to_radians());
    t.num("steer_filter_s", handler.f_or("BurnoutSteerFilterTime", DEFAULT_STEER_FILTER_S)?);
    t.nums("rpm_range", &handler.xy_or("BurnoutHelpForceRPMPercentageRange", DEFAULT_RPM_RANGE)?);
    t.num("reduction", handler.f_or("BurnoutHelpReduction", DEFAULT_REDUCTION)?);
    t.num("reduction_s", handler.f_or("BurnoutHelpReductionDuration", DEFAULT_REDUCTION_S)?);
    t.num("raise_rate", handler.f_or("BurnoutHelpRaiseRate", DEFAULT_RAISE_RATE)?);
    Ok(())
}

#[cfg(test)]
mod tests {
    use serde_json::{Value, json};

    use super::*;

    fn table(handler: Value) -> String {
        let path = "/PhysicsRide6/Physics/VehicleElements/InputHandler/Test_InputHandler.0";
        let text = json!({
            "Name": "Test_Bike",
            "Properties": { "VehicleElements": [{ "ObjectPath": path }] },
            "Linked": { path.to_ascii_lowercase(): { "Properties": handler } }
        });
        let vehicle = Vehicle::parse(&text.to_string(), None).unwrap();
        let mut t = Toml::new();
        write(&mut t, &vehicle).unwrap();
        t.into_string()
    }

    #[test]
    fn a_road_bike_handler_gives_a_twentieth_of_the_grip_and_no_slip_correction() {
        let text = table(json!({ "BurnoutGripMultiplier": 0.05, "BurnoutLeanLimit": 18.0 }));
        assert!(text.contains("[bike.burnout]"), "{text}");
        assert!(text.contains("grip = 0.05") && text.contains("slip_correction = 0"), "{text}");
        assert!(text.contains("speed_limit_kmh = 5"), "the constructor's minimum speed: {text}");
    }

    #[test]
    fn the_supermoto_handler_gives_more_grip_and_a_slip_correction() {
        let text = table(json!({ "BurnoutGripMultiplier": 0.15, "BurnoutSlipCorrectionMultiplier": 0.9 }));
        assert!(text.contains("grip = 0.15") && text.contains("slip_correction = 0.9"), "{text}");
    }

    #[test]
    fn a_handler_that_says_nothing_gets_the_constructors_numbers() {
        let text = table(json!({}));
        assert!(text.contains("grip = 1") && text.contains("slip_correction = 0"), "{text}");
        assert!(text.contains("[bike.burnout.help]") && text.contains("enabled = true"), "{text}");
        assert!(text.contains("steer_filter_s = 1") && text.contains("rpm_range = [0.75, 0.95]"), "{text}");
        assert!(text.contains("tcs_low_speed_modulation = true"), "the constructor's flag: {text}");
    }

    #[test]
    fn the_lean_limit_and_the_help_force_come_from_the_handler_in_radians() {
        let road = table(json!({ "BurnoutLeanLimit": 18.0 }));
        assert!(road.contains("lean_limit_rad = 0.31415"), "18 degrees: {road}");
        assert!(road.contains("yaw_rate_rad_s = 0.7853"), "45 degrees per second: {road}");
        let motard = table(json!({ "BurnoutLeanLimit": 10.0, "EnableBurnoutHelpForce": false }));
        assert!(motard.contains("lean_limit_rad = 0.1745") && motard.contains("enabled = false"), "{motard}");
    }
}
