//! `[bike.reverse]`: walking the bike backwards (`docs/specs/ride6-rider-styles.md`, section 5). The table is left out of a bike
//! that has no reverse.
//!
//! The input handler class holds three numbers, `BackwardForce`, `BackwardMaxSpeed` and `ReverseTimeThreshold`. The handler assets
//! of the install (all eight classes) set the first two to 200000 and 125; the third they leave at the constructor's default of the
//! native class (`UBikeInputHandler`, build `5.6.1-1697004`: 0.5; the constructor's defaults for the other two, 100000 and 250, are
//! the fallbacks here). The unit of the force is kg cm/s^2 and of the speed cm/s (the binary turns the speed into km/h with 0.036,
//! as `units.rs` says), so the manifest gets them divided by 100: 2000 N and 1.25 m/s for every bike.
//!
//! A bike has a reverse when the engine has more than two gears (the native `HasReverse` tests a count of 3 or more in the vehicle
//! state; reading that count as the number of gears is an inference, the scooters being the vehicles with fewer).

use anyhow::Result;

use super::chassis::handler;
use super::toml_text::Toml;
use super::units::metres;
use super::vehicle::Vehicle;

/// The native constructor's `BackwardForce` (kg cm/s^2), `BackwardMaxSpeed` (cm/s) and `ReverseTimeThreshold` (s).
pub const DEFAULT_FORCE: f64 = 100_000.0;
pub const DEFAULT_MAX_SPEED: f64 = 250.0;
pub const DEFAULT_HOLD: f64 = 0.5;
/// Fewer gears than this and the bike has no reverse.
pub const MIN_GEARS: usize = 3;

pub fn write(t: &mut Toml, vehicle: &Vehicle) -> Result<()> {
    let gears = vehicle.props().req("Transmission")?.req("ForwardGears")?.items()?.len();
    if gears < MIN_GEARS {
        return Ok(());
    }
    let handler = handler(vehicle)?;
    t.table("bike.reverse");
    t.num("force_n", metres(handler.f_or("BackwardForce", DEFAULT_FORCE)?));
    t.num("max_speed_mps", metres(handler.f_or("BackwardMaxSpeed", DEFAULT_MAX_SPEED)?));
    t.num("hold_s", handler.f_or("ReverseTimeThreshold", DEFAULT_HOLD)?);
    Ok(())
}

#[cfg(test)]
mod tests {
    use serde_json::{Value, json};

    use super::*;

    fn bike(gears: usize, handler: Value) -> Vehicle {
        let ratios: Vec<Value> = (0..gears).map(|n| json!({ "Ratio": 3.0 - n as f64 * 0.4 })).collect();
        let path = "/PhysicsRide6/Physics/VehicleElements/InputHandler/Test_InputHandler.0";
        let text = json!({
            "Name": "Test_Bike",
            "Properties": {
                "Transmission": { "ForwardGears": ratios },
                "VehicleElements": [{ "ObjectPath": path }]
            },
            "Linked": { path.to_ascii_lowercase(): { "Properties": handler } }
        });
        Vehicle::parse(&text.to_string(), None).unwrap()
    }

    fn table(vehicle: &Vehicle) -> String {
        let mut t = Toml::new();
        write(&mut t, vehicle).unwrap();
        t.into_string()
    }

    #[test]
    fn the_handlers_numbers_become_newtons_and_metres_per_second() {
        let text = table(&bike(6, json!({ "BackwardForce": 200000.0, "BackwardMaxSpeed": 125.0 })));
        assert!(text.contains("[bike.reverse]"), "{text}");
        assert!(text.contains("force_n = 2000"), "{text}");
        assert!(text.contains("max_speed_mps = 1.25"), "{text}");
        assert!(text.contains("hold_s = 0.5"), "the native default hold time: {text}");
    }

    #[test]
    fn a_handler_that_leaves_the_numbers_out_gets_the_constructors() {
        let text = table(&bike(5, json!({})));
        assert!(text.contains("force_n = 1000") && text.contains("max_speed_mps = 2.5"), "{text}");
    }

    #[test]
    fn a_bike_with_two_gears_or_fewer_has_no_reverse() {
        assert_eq!(table(&bike(2, json!({}))), "");
        assert_eq!(table(&bike(1, json!({}))), "");
        assert!(table(&bike(3, json!({}))).contains("[bike.reverse]"));
    }
}
