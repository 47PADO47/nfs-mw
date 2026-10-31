//! `[bike.burnout]`: what the rear tyre does while the rider holds the front brake and the throttle with the bike standing
//! (`docs/specs/ride6-burnout.md`).
//!
//! The input handler class holds `BurnoutGripMultiplier` and `BurnoutSlipCorrectionMultiplier`, which the handler writes into the rear wheel
//! in a burnout, and `BurnoutMinSpeed`, the speed (km/h) under which a burnout is possible. The handler assets of the install set the
//! grip multiplier to 0.05 (0.15 for the supermoto handler), the slip correction only for the supermoto (0.9) and the speed never; the
//! rest is the native constructor's (`UBikeInputHandler`, build `5.6.1-1697004`, `0x1450bbf80`): a grip multiplier of 1, a slip correction
//! of 0 and a minimum speed of 5 km/h (`0x1450bd6d0`). The table is written for every bike, so a reader can see which handler it came from.

use anyhow::Result;

use super::chassis::handler;
use super::toml_text::Toml;
use super::vehicle::Vehicle;

/// The native constructor's `BurnoutGripMultiplier`, `BurnoutSlipCorrectionMultiplier` and `BurnoutMinSpeed` (km/h).
pub const DEFAULT_GRIP: f64 = 1.0;
pub const DEFAULT_SLIP_CORRECTION: f64 = 0.0;
pub const DEFAULT_SPEED_LIMIT_KMH: f64 = 5.0;

pub fn write(t: &mut Toml, vehicle: &Vehicle) -> Result<()> {
    let handler = handler(vehicle)?;
    t.table("bike.burnout");
    t.num("grip", handler.f_or("BurnoutGripMultiplier", DEFAULT_GRIP)?);
    t.num("slip_correction", handler.f_or("BurnoutSlipCorrectionMultiplier", DEFAULT_SLIP_CORRECTION)?);
    t.num("speed_limit_kmh", handler.f_or("BurnoutMinSpeed", DEFAULT_SPEED_LIMIT_KMH)?);
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
    }
}
