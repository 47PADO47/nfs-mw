//! `[bike.transmission]`: gears, final ratio, the automatic gearbox, the clutch and its launch
//! (`docs/specs/ride6-engine-drivetrain.md`, `docs/specs/ride6-gearbox-auto.md`).

use anyhow::{Result, bail};

use super::chassis::handler;
use super::defaults;
use super::engine::EngineData;
use super::node::Node;
use super::shift::{self, Tolerance};
use super::toml_text::Toml;
use super::units::{metres_per_second, newton_metres};
use super::vehicle::Vehicle;

/// A change point of the reference game's `0.1..0.9` range for the down shift; the library refuses others.
const DOWN_SHIFT_RANGE: [f64; 2] = [0.1, 0.9];

pub fn write(t: &mut Toml, vehicle: &Vehicle, engine: &EngineData) -> Result<()> {
    let props = vehicle.props();
    let transmission = props.req("Transmission")?;
    let handler = handler(vehicle)?;
    let mut gears = Vec::new();
    for gear in transmission.req("ForwardGears")?.items()? {
        gears.push(gear.f("Ratio")?);
    }

    t.table("bike.transmission");
    t.nums("gears", &gears);
    t.num("final_ratio", transmission.f("FinalRatio")?);
    t.num("efficiency", defaults::TRANSMISSION_EFFICIENCY);

    // The rows of the change points are generated from the torque curve when the handler says so; the importer does not
    // read hand-written rows.
    let settings = handler.req("TransmissionSettings")?;
    if !settings.req("bAutomaticGearChangePoints")?.flag()? {
        bail!("{}: hand-written gear change points are not supported", settings.path());
    }
    let mut tolerances = Vec::new();
    let rows = settings.req("GearChangePoints")?.items()?;
    for row in rows.iter().take(gears.len().saturating_sub(1)) {
        tolerances.push(Tolerance { up: row.f("ChangeUpTolerance")?, down: row.f("ChangeDownTolerance")? });
    }
    if tolerances.len() + 1 < gears.len() {
        bail!("{}: fewer change point rows than gear changes", settings.path());
    }
    let reduced = shift::reduce(&engine.full_curve, &gears, &tolerances, engine.idle_rpm, engine.max_rpm);
    t.num("up_shift", reduced.up);
    let down =
        reduced.down.filter(|down| *down >= DOWN_SHIFT_RANGE[0] && *down <= DOWN_SHIFT_RANGE[1] && *down < reduced.up);
    match (reduced.down, down) {
        (_, Some(down)) => t.num("down_shift", down),
        (Some(_), None) => {
            t.note("the generated change-down point is outside the manifest's range: `down_shift` is left to the model's choice");
        }
        (None, None) => {}
    }
    t.num("up_shift_time_s", handler.f("GearUpChangeTime")?);
    t.num("down_shift_time_s", handler.f("GearDownChangeTime")?);
    t.num("blip_throttle", handler.f_or("BlipperThrottleOnShift", 0.0)?);
    t.num("clutch_linearity", transmission.f("ClutchLinearity")?);
    t.num("clutch_max_factor", defaults::CLUTCH_MAX_FACTOR);

    if let Some(range) = transmission.get("SlipperClutchDeltaTorqueRange") {
        let [low, high] = range.xy()?;
        t.nums("slipper_range_nm", &[newton_metres(low), newton_metres(high)]);
        t.num("slipper_intensity", transmission.f("SlipperClutchMaxIntensity")?);
    }
    launch(t, &handler)
}

/// The launch: the road speed above which the clutch is locked, and the feathering controller's target engine speed.
fn launch(t: &mut Toml, handler: &Node<'_>) -> Result<()> {
    let [_, release] = handler.xy_or("ClucthReleaseVelocityRange", [0.0, 0.0])?;
    if release > 0.0 {
        t.num("release_speed_mps", metres_per_second(release));
    }
    let Some(feathering) = handler.get("ClutchFeatheringOnStart") else { return Ok(()) };
    if !feathering.flag_or("bEnable", false)? {
        return Ok(());
    }
    t.num("feather_factor", feathering.f("ClutchFeatheringPeakTorqueRPMFactor")?);
    t.num("feather_max_factor", feathering.f("ClutchFeatheringMaxRPMFactor")?);
    Ok(())
}
