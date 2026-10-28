//! `[bike.brakes]` and `[bike.brakes.abs]`: the front brake, the rear brake (the hand-brake channel of RIDE 6), the linked
//! brakes and the anti-lock element (`docs/specs/ride6-engine-drivetrain.md`, "Brake torque and its distribution";
//! `docs/specs/ride6-rider-help.md`, "ABS").

use anyhow::{Result, bail};

use super::chassis::handler;
use super::curve::Translator;
use super::defaults;
use super::toml_text::Toml;
use super::units::metres_per_second;
use super::vehicle::Vehicle;

pub fn write(t: &mut Toml, vehicle: &Vehicle) -> Result<()> {
    let props = vehicle.props();
    let brake = props.req("Brake")?;
    // The wheel brake torque is `BrakeForce * BrakeBalance * R` at the front and `HandbrakeForce * R` at the rear wheel
    // (the wheel with `bAffectedByHandbrake`). A balance under 1 would put part of `BrakeForce` on the rear wheel's lever of
    // the front brake, which the manifest has no key for; every bike of the install has 1.
    let balance = brake.f("BrakeBalance")?;
    if (balance - defaults::BRAKE_BALANCE).abs() > 1e-9 {
        bail!("{}: a brake balance of {balance} has no counterpart in the manifest", brake.path());
    }
    let rear_hand = props.req("RearWheel")?.flag_or("bAffectedByHandbrake", false)?;
    let rear_force = match rear_hand {
        true => brake.f("HandbrakeForce")?,
        false => 0.0,
    };
    let handler = handler(vehicle)?;
    t.table("bike.brakes");
    t.num("front_force_n", brake.f("BrakeForce")? * balance);
    t.num("rear_force_n", rear_force);
    t.num("joined_factor", handler.f_or("JoinedBrakesFactor", 0.0)?);
    for (key, data_key) in [("joined_front", "JoinedBrakesFrontFactor"), ("joined_rear", "JoinedBrakesRearFactor")] {
        let share = handler.f_or(data_key, defaults::JOINED_SHARE)?;
        if share != 0.0 {
            t.num(key, share);
        }
    }
    anti_lock(t, vehicle)
}

/// The anti-lock element: per wheel an intensity and the normalised slip ratio (-1 is the force peak) where it starts and
/// where it acts fully, and the speeds over which it fades in.
fn anti_lock(t: &mut Toml, vehicle: &Vehicle) -> Result<()> {
    let Some(abs) = vehicle.element("ABSVehicleElement")? else { return Ok(()) };
    let mut intensity = [0.0, 0.0];
    let mut slips = [[0.0; 2]; 2];
    for (index, (enabled, tuning)) in
        [("bFrontABSEnabled", "FrontABSTuning"), ("bRearABSEnabled", "RearABSTuning")].into_iter().enumerate()
    {
        if !abs.flag_or(enabled, false)? {
            continue;
        }
        let node = abs.req(tuning)?;
        intensity[index] = node.f("Intensity")?;
        slips[index] = Translator::read(vehicle, &node.req("NormalizedSlipTuning")?)?.input_range();
    }
    if intensity == [0.0, 0.0] {
        return Ok(());
    }
    let [low, high] = Translator::read(vehicle, &abs.req("ABSSpeedTuning")?)?.input_range();
    t.table("bike.brakes.abs");
    t.num("front_intensity", intensity[0]);
    t.num("rear_intensity", intensity[1]);
    t.nums("speed_range_mps", &[metres_per_second(low), metres_per_second(high)]);
    t.nums("front_slip", &slips[0]);
    t.nums("rear_slip", &slips[1]);
    Ok(())
}
