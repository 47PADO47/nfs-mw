//! `[bike.input]`: the rates at which the rider's axes move (`docs/specs/ride6-feel-round8.md`, section 4). The vehicle's `InputData`
//! has a rise and a fall rate per second for the throttle and the brake, and for the stick either the deprecated pair or one
//! `InputRate`, by `bUseDeprecatedRate`. A field the asset leaves out takes the constructor default of the reference, 10000 per second:
//! at once.

use anyhow::Result;

use super::toml_text::Toml;
use super::vehicle::Vehicle;

/// The constructor default of every rate (`0x1450a5f20`).
const AT_ONCE: f64 = 10_000.0;

pub fn write(t: &mut Toml, vehicle: &Vehicle) -> Result<()> {
    let data = vehicle.props().req("InputData")?;
    let rate = |axis: &str, key: &str| -> Result<f64> {
        match data.get(axis) {
            Some(node) => node.f_or(key, AT_ONCE),
            None => Ok(AT_ONCE),
        }
    };
    let deprecated = match data.get("Steer") {
        Some(steer) => steer.flag_or("bUseDeprecatedRate", true)?,
        None => true,
    };
    let (steer_rise, steer_fall) = match deprecated {
        true => (rate("Steer", "DeprecatedRiseRate")?, rate("Steer", "DeprecatedFallRate")?),
        false => (rate("Steer", "InputRate")?, rate("Steer", "InputRate")?),
    };
    t.table("bike.input");
    t.num("throttle_rise", rate("Throttle", "RiseRate")?);
    t.num("throttle_fall", rate("Throttle", "FallRate")?);
    t.num("brake_rise", rate("Brake", "RiseRate")?);
    t.num("brake_fall", rate("Brake", "FallRate")?);
    t.num("steer_rise", steer_rise);
    t.num("steer_fall", steer_fall);
    Ok(())
}
