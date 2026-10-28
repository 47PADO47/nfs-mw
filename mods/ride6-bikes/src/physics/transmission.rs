//! `[bike.transmission]`: gears, final ratio, the automatic gearbox, the clutch and its launch
//! (`docs/specs/ride6-engine-drivetrain.md`, `docs/specs/ride6-gearbox-auto.md`).

use anyhow::{Result, bail};

use super::chassis::handler;
use super::defaults::{self, gearbox as default};
use super::node::Node;
use super::toml_text::Toml;
use super::units::{metres_per_second, newton_metres};
use super::vehicle::Vehicle;

pub fn write(t: &mut Toml, vehicle: &Vehicle) -> Result<()> {
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

    // The change points are generated from the torque curve and the ratios when the handler says so, and the library does the
    // same; the data gives it the tolerances that scale the compared torques.
    let settings = handler.req("TransmissionSettings")?;
    if !settings.req("bAutomaticGearChangePoints")?.flag()? {
        bail!("{}: hand-written gear change points are not supported", settings.path());
    }
    let rows = settings.req("GearChangePoints")?.items()?;
    if rows.len() + 1 < gears.len() {
        bail!("{}: fewer change point rows than gear changes", settings.path());
    }
    let mut up = Vec::new();
    let mut down = Vec::new();
    for row in rows.iter().take(gears.len()) {
        up.push(row.f("ChangeUpTolerance")?);
        down.push(row.f("ChangeDownTolerance")?);
    }
    t.nums("up_tolerance", &up);
    t.nums("down_tolerance", &down);
    hold_off(t, &handler, "prevent_up_lean", "PreventGearUpChangeLean")?;
    hold_off(t, &handler, "prevent_down_lean", "PreventGearDownChangeLean")?;
    hold_off(t, &handler, "prevent_down_throttle", "PreventGearDownChangeThrottle")?;
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

/// A hold-off of the automatic changes: `[factor, linearity]` of the fields `{prefix}Factor` and `{prefix}Linearity`.
fn hold_off(t: &mut Toml, handler: &Node<'_>, key: &str, prefix: &str) -> Result<()> {
    let factor = handler.f_or(&format!("{prefix}Factor"), default::HOLD_FACTOR)?;
    let linearity = handler.f_or(&format!("{prefix}Linearity"), default::HOLD_LINEARITY)?;
    if factor != 0.0 {
        t.nums(key, &[factor, linearity]);
    }
    Ok(())
}

/// The launch: the road speed above which the clutch is locked, the feathering controller, or the plain launch clutch.
fn launch(t: &mut Toml, handler: &Node<'_>) -> Result<()> {
    let [_, release] = handler.xy_or("ClucthReleaseVelocityRange", [0.0, 0.0])?;
    if release > 0.0 {
        t.num("release_speed_mps", metres_per_second(release));
    }
    let feathering = handler.get("ClutchFeatheringOnStart");
    let enabled = match &feathering {
        Some(node) => node.flag_or("bEnable", false)?,
        None => false,
    };
    let Some(feathering) = feathering.filter(|_| enabled) else {
        t.nums(
            "launch_rpm",
            &[
                handler.f_or("ClucthMinRPMPercentage", default::CLUTCH_MIN_RPM)?,
                handler.f_or("ClucthMaxRPMPercentage", default::CLUTCH_MAX_RPM)?,
            ],
        );
        return Ok(());
    };
    t.num("feather_factor", feathering.f("ClutchFeatheringPeakTorqueRPMFactor")?);
    t.num("feather_max_factor", feathering.f("ClutchFeatheringMaxRPMFactor")?);
    t.table("bike.transmission.feather");
    t.num(
        "target_tolerance",
        feathering.f_or("ClutchFeatheringTargetRPMTolerance", default::FEATHER_TARGET_TOLERANCE)?,
    );
    t.num(
        "exit_tolerance",
        feathering.f_or("ClutchFeatheringTargetRPMExitTolerance", default::FEATHER_EXIT_TOLERANCE)?,
    );
    t.num("throttle_cap", feathering.f_or("ClutchFeatheringThrottleCap", default::FEATHER_THROTTLE_CAP)?);
    t.num("min_slip", feathering.f_or("ClutchFeatheringMinSlip", default::FEATHER_MIN_SLIP)?);
    t.num("delta_time_s", feathering.f_or("ClutchFeatheringDeltaTime", default::FEATHER_DELTA_TIME)?);
    t.num("rpm_raise", feathering.f_or("ClutchFeatheringRPMRaiseFactor", default::FEATHER_RPM_RAISE)?);
    t.num("release_factor", feathering.f_or("ClutchFeatheringClutchReleaseFactor", default::FEATHER_RELEASE_FACTOR)?);
    Ok(())
}
