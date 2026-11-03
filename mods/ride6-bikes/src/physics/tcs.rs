//! `[bike.tcs]`: the traction control element (`docs/specs/ride6-rider-help.md`, "Traction control"). The table is left out of a
//! bike that runs none.

use anyhow::Result;

use super::toml_text::Toml;
use super::units::metres_per_second;
use super::vehicle::Vehicle;

pub fn write(t: &mut Toml, vehicle: &Vehicle) -> Result<()> {
    let Some(control) = vehicle.element("TCS")? else { return Ok(()) };
    let [slip_low, slip_high] = control.xy_of("TCSSlipThresholdsDry")?;
    let [speed_low, speed_high] = control.xy_of("TCSSpeedThresholds")?;
    t.table("bike.tcs");
    t.nums("slip", &[slip_low, slip_high]);
    t.nums("speed_range_mps", &[metres_per_second(speed_low), metres_per_second(speed_high)]);
    Ok(())
}
