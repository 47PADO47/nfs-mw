//! `[bike.steering]`, `[bike.steering.lean]` and `[bike.fall]`: the handlebar, the rider's lean controller and when the
//! rider has lost the bike (`docs/specs/ride6-lean-controller.md`). Speeds are km/h in the data and m/s in the manifest,
//! angles degrees and radians.

use anyhow::Result;

use super::Geometry;
use super::chassis::handler;
use super::curve::{Translator, simplify};
use super::damping::damping_multiplier;
use super::defaults;
use super::engine::MAX_CURVE_POINTS;
use super::node::Node;
use super::toml_text::Toml;
use super::units::{metres_per_second, radians};
use super::vehicle::Vehicle;

/// The damping ratio of the data's second-order filter: `Damp <= 0` is the ratio `|Damp|` (1 when 0), `Damp > 0` the absolute
/// coefficient `c = 2 sqrt(K) z` (`ride6-lean-controller.md`, "The second-order filter used everywhere").
pub fn damping_ratio(k: f64, damp: f64) -> f64 {
    if damp < 0.0 {
        return -damp;
    }
    if damp == 0.0 {
        return 1.0;
    }
    damp / (2.0 * k.sqrt())
}

/// The lean the rider can never exceed and at which the bike counts as fallen: `FallLeanLimitAirborne`. The lean limit on
/// the ground is a run-time value of the grip (`bFallLeanLimitBasedOnGrip`) that is not in the data.
pub fn fall_lean_rad(handler: &Node<'_>) -> Result<f64> {
    Ok(radians(handler.f("FallLeanLimitAirborne")?))
}

/// A speed translator as `[m/s, y]` points: the input range is km/h.
fn speed_curve(vehicle: &Vehicle, node: &Node<'_>) -> Result<Vec<[f64; 2]>> {
    let points = Translator::read(vehicle, node)?.points(metres_per_second)?;
    Ok(simplify(points, MAX_CURVE_POINTS))
}

/// A tuning of the stick, a curve in the stick's own unit.
fn stick_curve(vehicle: &Vehicle, node: &Node<'_>) -> Result<Vec<[f64; 2]>> {
    let Some(translator) = Translator::read_opt(vehicle, node)? else { return Ok(Vec::new()) };
    Ok(simplify(translator.points(|x| x)?, MAX_CURVE_POINTS))
}

pub fn write(t: &mut Toml, vehicle: &Vehicle, geometry: &Geometry) -> Result<()> {
    let props = vehicle.props();
    let handler = handler(vehicle)?;
    let steer_angle = props.req("FrontWheel")?.f("SteerAngle")?;
    let (handlebar_k, handlebar_damp) = (handler.f("HandlebarK")?, handler.f("HandlebarDamp")?);

    t.table("bike.steering");
    t.num("max_angle_rad", radians(steer_angle));
    t.num("response", handlebar_k);
    t.num("damping_ratio", damping_ratio(handlebar_k, handlebar_damp));
    t.num("max_lean_rad", fall_lean_rad(&handler)?);
    lean(t, vehicle, &handler, geometry)?;

    t.table("bike.fall");
    t.num("lean_rad", fall_lean_rad(&handler)?);
    t.num("pitch_rad", radians(handler.f("FallPitchLimit")?));
    t.num("speed_direction_rad", radians(handler.f("FallSpeedDirAngleLimit")?));
    t.num("angular_speed", handler.f("FallAngularSpeedLimit")?);
    t.num("min_speed_mps", metres_per_second(handler.f("FallMinSpeed")?));
    Ok(())
}

fn lean(t: &mut Toml, vehicle: &Vehicle, handler: &Node<'_>, geometry: &Geometry) -> Result<()> {
    let (lean_k, lean_damp) = (handler.f("LeanK")?, handler.f("LeanDamp")?);
    let (steer_k, steer_damp) = (handler.f("SteerK")?, handler.f("SteerDamp")?);
    // The target lean's filter. The two bikes of the install that do not use the filter (`bUseKDampFilterForTargetLeanAngle`
    // false) step the target by a square-root rule the library does not have; their asset still holds the filter's numbers.
    let target = handler.req("TargetLeanAngleFilter")?;
    let (target_k, target_damp) = (target.f("KValue")?, target.f("DampValue")?);
    if !handler.req("bUseKDampFilterForTargetLeanAngle")?.flag()? {
        t.note("this bike does not use the second-order filter of the target lean; the model's filter is used with the asset's numbers");
    }

    t.table("bike.steering.lean");
    t.num("k", lean_k);
    t.num("damping_ratio", damping_ratio(lean_k, lean_damp));
    t.num("damping_multiplier", damping_multiplier(vehicle, handler, geometry.wheelbase())?);
    t.num("steer_k", steer_k);
    t.num("steer_damping_ratio", damping_ratio(steer_k, steer_damp));
    t.num("target_k", target_k);
    t.num("target_damping_ratio", damping_ratio(target_k, target_damp));
    t.num("time_scale", handler.f("LeanTimeScale")?);
    t.num("time_scale_attenuation", handler.f_or("LeanTimeScaleAttenuation", defaults::LEAN_TIME_SCALE_ATTENUATION)?);
    t.num("time_scale_sensitivity", handler.f_or("LeanTimeScaleSensitivity", defaults::LEAN_TIME_SCALE_SENSITIVITY)?);
    t.pairs("time_scale_speed", &speed_curve(vehicle, &handler.req("LeanTimeScaleSpeedTuning")?)?);
    let input = stick_curve(vehicle, &handler.req("LeanTimeScaleInputTuning")?)?;
    if !input.is_empty() {
        t.pairs("time_scale_input", &input);
    }
    let g_tuning = match handler.get("LeanLimitGForceTuning") {
        Some(node) => stick_curve(vehicle, &node)?,
        None => Vec::new(),
    };
    if !g_tuning.is_empty() {
        t.pairs("g_tuning", &g_tuning);
    }
    let [skid_low, skid_high] = handler.xy_of("LeanLimitSkidRange")?;
    t.nums("skid_range_rad", &[radians(skid_low), radians(skid_high)]);
    t.num("skid_linearity", handler.f("LeanLimitSkidLinearity")?);
    t.pairs("skid_reduction", &speed_curve(vehicle, &handler.req("LeanLimitSkidReduction")?)?);
    t.num("stopped_limit_rad", radians(handler.f("StoppedLeanLimit")?));
    t.num("stopped_speed_mps", metres_per_second(handler.f("StoppedSpeedThreshold")?));
    // The asset's `SteerAlignmentRange` (m/s) is the range `ride6-lean-controller.md` calls `LowSpeedSteerBlendRange`.
    t.nums("blend_range_mps", &handler.xy_of("SteerAlignmentRange")?);
    t.num("rider_mass_scale", handler.f("RiderMassScale")?);
    t.pairs("help_speed", &speed_curve(vehicle, &handler.req("LeanHelpFactorSpeedTuning")?)?);
    t.num("help_limit", help_limit(vehicle, handler)?);
    Ok(())
}

/// The share of the front tyre's lateral peak the lean help may add: the output of `LeanHelpLimitSpeedTuning` at the top of
/// its speed range, where the bike is riding (the library has one number).
fn help_limit(vehicle: &Vehicle, handler: &Node<'_>) -> Result<f64> {
    let translator = Translator::read(vehicle, &handler.req("LeanHelpLimitSpeedTuning")?)?;
    Ok(translator.output_range()[1])
}
