//! `[bike.steering]`, `[bike.steering.lean]` and `[bike.fall]`: the handlebar, the rider's lean controller and when the
//! rider has lost the bike (`docs/specs/ride6-lean-controller.md`). Speeds are km/h in the data and m/s in the manifest,
//! angles degrees and radians.

use anyhow::Result;

use super::Geometry;
use super::chassis::handler;
use super::curve::{Translator, runtime_curve, simplify};
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

/// The lean the rider can never exceed and at which the bike counts as fallen in the air: `FallLeanLimitAirborne`. On the
/// ground the limit follows the grip (`bFallLeanLimitBasedOnGrip`, the `lean_by_grip` key) and starts here.
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
    // The speed above which the angle of the velocity to the heading is tested: `FallSpeedDirAngleMinSpeed` (0 when the asset
    // leaves it out); `FallMinSpeed` gates only the rider-force tests, which the library does not have.
    t.num("min_speed_mps", metres_per_second(handler.f_or("FallSpeedDirAngleMinSpeed", 0.0)?));
    if handler.flag_or("BlockFallActive", false)? && handler.f("BlockFallLeanLimitRatio")? > 0.0 {
        t.num("block_ratio", handler.f("BlockFallLeanLimitRatio")?);
        t.num("block_min_speed_mps", metres_per_second(handler.f("BlockFallLeanLimitMinSpeedKmh")?));
        t.num("block_time_s", handler.f("BlockFallTimeLimit")?);
        t.num("block_wait_s", handler.f("BlockFallReEnableWaitTimeLimit")?);
    }
    fall_extra(t, &handler)
}

/// The grip-following lean limit, its hold time and the limits in the air (`ride6-chassis-round6.md`, "Fall tests").
fn fall_extra(t: &mut Toml, handler: &Node<'_>) -> Result<()> {
    if handler.flag_or("bOverrideFallLeanLimit", false)? && handler.flag_or("bFallLeanLimitBasedOnGrip", false)? {
        t.flag("lean_by_grip", true);
        t.num("lean_rise", handler.f_or("FallLeanLimitUpFactor", 0.0)?);
        t.num("lean_fall", handler.f_or("FallLeanLimitDownFactor", 0.0)?);
    }
    t.num("lean_duration_s", handler.f_or("FallLeanLimitDuration", 0.0)?);
    t.num("airborne_lean_rad", radians(handler.f_or("FallLeanLimitAirborne", 0.0)?));
    t.num("airborne_pitch_rad", radians(handler.f_or("FallPitchLimitAirborne", 0.0)?));
    t.num("airborne_angular_speed", handler.f_or("FallAngularSpeedLimitAirborne", 0.0)?);
    t.num("terrain_influence", handler.f_or("FallLeanLimitTerrainInfluence", 0.0)?);
    t.num("terrain_reduction", handler.f_or("FallLeanLimitTerrainReduction", 1.0)?);
    t.num("terrain_max_delta_rad", radians(handler.f_or("FallLeanLimitMaxDelta", 0.0)?));
    t.num("highside_lean_rad", radians(handler.f_or("FallHighsideLeanLimit", 0.0)?));
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
    help_extra(t, vehicle, handler)?;
    terrain(t, vehicle, handler)?;
    stability(t, vehicle, handler)
}

/// A speed curve with its outputs times `scale`.
fn scaled(points: Vec<[f64; 2]>, scale: f64) -> Vec<[f64; 2]> {
    points.into_iter().map(|[x, y]| [x, y * scale]).collect()
}

/// The stopped blend, the split between force and couple, and the skid cut of the lean help factor
/// (`ride6-chassis-round6.md`, "Lean help"). A tuning the asset leaves out is left out of the manifest.
fn help_extra(t: &mut Toml, vehicle: &Vehicle, handler: &Node<'_>) -> Result<()> {
    let [stopped_low, stopped_high] = handler.xy_or("LeanHelpFactorStoppedSpeedRange", [0.0, 0.0])?;
    if stopped_high > 0.0 {
        t.nums("help_stopped_range_mps", &[metres_per_second(stopped_low), metres_per_second(stopped_high)]);
    }
    if handler.flag_or("bOverrideLeanHelpBalance", false)? {
        let multiplier = handler.f_or("LeanHelpBalanceMultiplier", 1.0)?;
        let curve = speed_curve(vehicle, &handler.req("LeanHelpBalanceSpeedTuning")?)?;
        t.pairs("help_balance_speed", &scaled(curve, multiplier));
    } else if handler.has("LeanHelpBalance") {
        t.pairs("help_balance_speed", &[[0.0, handler.f("LeanHelpBalance")?]]);
    }
    let Some(node) = handler.get("LeanHelpFactorSkidReduction") else { return Ok(()) };
    if Translator::read_opt(vehicle, &node)?.is_none() {
        return Ok(());
    }
    let [skid_low, skid_high] = handler.xy_of("LeanHelpFactorSkidRange")?;
    t.nums("help_skid_range_rad", &[radians(skid_low), radians(skid_high)]);
    t.num("help_skid_linearity", handler.f_or("LeanHelpFactorSkidLinearity", 1.0)?);
    let scale = handler.f_or("LeanHelpFactorReductionScale", 1.0)?;
    t.pairs("help_skid_reduction", &scaled(speed_curve(vehicle, &node)?, scale));
    let Some(input) = handler.get("LeanHelpFactorSkidInputInfluence") else { return Ok(()) };
    let Some(curve) = runtime_curve(vehicle, &input)? else { return Ok(()) };
    t.pairs("help_skid_input", &simplify(curve.polyline(), MAX_CURVE_POINTS));
    Ok(())
}

/// The share of the front tyre's lateral peak the lean help may add: the output of `LeanHelpLimitSpeedTuning` at the top of
/// its speed range, where the bike is riding (the library has one number).
fn help_limit(vehicle: &Vehicle, handler: &Node<'_>) -> Result<f64> {
    let translator = Translator::read(vehicle, &handler.req("LeanHelpLimitSpeedTuning")?)?;
    Ok(translator.output_range()[1])
}

/// The stability help (`ride6-chassis-round7.md`, section 3): the speed tuning, its lean term, the throttle and brake cuts, the speed
/// range and the lean limit of the desired yaw rate. A bike whose speed tuning is 0 everywhere (the supermoto) has none.
fn stability(t: &mut Toml, vehicle: &Vehicle, handler: &Node<'_>) -> Result<()> {
    if !handler.flag_or("EnableStabilityHelp", false)? {
        return Ok(());
    }
    let Some(node) = handler.get("StabilityHelpFactorSpeedTuning") else { return Ok(()) };
    let Some(tuning) = Translator::read_opt(vehicle, &node)? else { return Ok(()) };
    let [low, high] = tuning.output_range();
    if low == 0.0 && high == 0.0 {
        return Ok(());
    }
    t.pairs("stability_speed", &speed_curve(vehicle, &node)?);
    t.num("stability_lean_influence", handler.f_or("StabilityHelpFactorLeanInfluence", 0.0)?);
    t.num("stability_lean_sensitivity", handler.f_or("StabilityHelpFactorLeanSensitivity", 1.0)?);
    let scale = handler.f_or("StabilityHelpFactorReductionScale", 1.0)?;
    for (key, name) in [
        ("stability_throttle_reduction", "StabilityHelpFactorThrottleReduction"),
        ("stability_brake_reduction", "StabilityHelpFactorBrakeReduction"),
    ] {
        let Some(node) = handler.get(name) else { continue };
        if Translator::read_opt(vehicle, &node)?.is_some() {
            t.pairs(key, &scaled(speed_curve(vehicle, &node)?, scale));
        }
    }
    let [range_low, range_high] = handler.xy_or("StabilityHelpSpeedRange", [10.0, 15.0])?;
    t.nums("stability_speed_range_mps", &[metres_per_second(range_low), metres_per_second(range_high)]);
    t.num("stability_max_lean_rad", radians(handler.f_or("StabilityHelpMaxLeanAngle", 75.0)?));
    t.flag("stability_terrain_normal", handler.flag_or("StabilityHelpUsesTerrainNormal", false)?);
    Ok(())
}

/// The terrain term of the lean limit (`ride6-chassis-round7.md`, section 4). Constructor defaults: maximum 90 degrees, both shares 1.
fn terrain(t: &mut Toml, vehicle: &Vehicle, handler: &Node<'_>) -> Result<()> {
    let Some(node) = handler.get("TerrainInfluenceSpeedTuning") else { return Ok(()) };
    if Translator::read_opt(vehicle, &node)?.is_none() {
        return Ok(());
    }
    t.pairs("terrain_speed", &speed_curve(vehicle, &node)?);
    t.num("terrain_max_angle_rad", radians(handler.f_or("TerrainInfluenceMaxAngle", 90.0)?));
    t.num("terrain_increment", handler.f_or("TerrainLeanLimitIncrementScale", 1.0)?);
    t.num("terrain_reduction", handler.f_or("TerrainLeanLimitReductionScale", 1.0)?);
    Ok(())
}
