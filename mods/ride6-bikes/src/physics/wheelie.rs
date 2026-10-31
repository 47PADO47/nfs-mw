//! `[bike.wheelie]`: the wheelie and stoppie assist (`UWheelieControl`), the rider help that cuts the throttle and eases the
//! front brake as a wheel lifts, and the lean limit while pitched (`docs/specs/ride6-rider-help.md`). The table is left out
//! of a bike that runs no wheelie control.

use anyhow::Result;

use super::chassis::handler;
use super::curve::{runtime_curve, simplify};
use super::defaults::pitch as default;
use super::engine::MAX_CURVE_POINTS;
use super::node::Node;
use super::steering::fall_lean_rad;
use super::toml_text::Toml;
use super::units::{metres_per_second, radians};
use super::vehicle::Vehicle;
use super::wheelie_help;

/// An `FVector2D` of degrees as radians.
fn radians_pair(range: [f64; 2]) -> [f64; 2] {
    [radians(range[0]), radians(range[1])]
}

pub fn write(t: &mut Toml, vehicle: &Vehicle) -> Result<()> {
    let Some(control) = vehicle.element("WheelieControl")? else { return Ok(()) };
    let handler = handler(vehicle)?;
    t.table("bike.wheelie");
    let help = vehicle.element("RiderHelp")?;
    if let Some(help) = &help {
        rider_help(t, help)?;
        wheelie_help::write(t, vehicle, help)?;
    }
    let (wheelie_lean, stoppie_lean) = (lean_tuning(&handler, "Wheelie")?, lean_tuning(&handler, "Stoppie")?);
    if wheelie_lean.is_none() || stoppie_lean.is_none() {
        t.note(
            "the lean limit while a wheelie or a stoppie is not tuned for this bike: the lean limit is not pulled in",
        );
    }
    let inert_limit = fall_lean_rad(&handler)?;
    let wheelie = Pitch { params: control.req("WheelieParams")?, lean: wheelie_lean };
    let stoppie = Pitch { params: control.req("StoppieParams")?, lean: stoppie_lean };
    pitch(t, vehicle, "bike.wheelie.wheelie", &wheelie, inert_limit)?;
    pitch(t, vehicle, "bike.wheelie.stoppie", &stoppie, inert_limit)
}

/// The parameter set of one lift direction and the lean tuning that goes with it.
struct Pitch<'a> {
    params: Node<'a>,
    lean: Option<(f64, [f64; 2])>,
}

/// The throttle and brake control of the rider help: the lift angles (degrees in the data) and lift speeds (rad/s) between
/// which the help cuts the throttle for a wheelie and eases the front brake for a stoppie.
fn rider_help(t: &mut Toml, help: &Node<'_>) -> Result<()> {
    if help.flag_or("bEnableRiderHelpThrottleControl", false)? {
        t.nums("throttle_pitch_rad", &radians_pair(help.xy_of("AutoRiderWheelieThrottleControl")?));
        t.nums("forced_throttle_pitch_rad", &radians_pair(help.xy_of("AutoRiderForcedWheelieThrottleControl")?));
        t.nums("throttle_pitch_speed", &help.xy_of("PitchSpeedWheelieThrottleControl")?);
    }
    if help.flag_or("bEnableRiderHelpBrakeControl", false)? {
        t.nums("brake_pitch_rad", &radians_pair(help.xy_of("AutoRiderStoppieBrakeControl")?));
        t.nums("brake_pitch_speed", &help.xy_of("PitchSpeedStoppieBrakeControl")?);
    }
    Ok(())
}

/// The lean limit during a wheelie or stoppie (`WheelieLeanLimitTuning`, `StoppieLeanLimitTuning` of the input handler): the
/// limit in degrees and the pitch range in degrees over which it is blended in. `None` when the tuning is off.
fn lean_tuning(handler: &Node<'_>, kind: &str) -> Result<Option<(f64, [f64; 2])>> {
    if !handler.flag_or(&format!("bEnable{kind}LeanLimitTuning"), false)? {
        return Ok(None);
    }
    let tuning = handler.req(&format!("{kind}LeanLimitTuning"))?;
    Ok(Some((tuning.f("LeanLimit")?, tuning.xy_of("AngleRange")?)))
}

/// One parameter set. The fields the asset leaves out take the constructor defaults of `FWheelieControlParams`
/// (`ride6-rider-help.md`, "Wheelie and stoppie control").
fn pitch(t: &mut Toml, vehicle: &Vehicle, table: &str, set: &Pitch<'_>, inert_limit: f64) -> Result<()> {
    let params = &set.params;
    t.table(table);
    t.num("deadzone", params.f_or("InputDeadzone", default::DEADZONE)?);
    t.num("max_angle_rad", radians(params.f_or("MaxAngle", default::MAX_ANGLE_DEG)?));
    let [low, high] = params.xy_or("MaxAngleFactorSpeedTuning", default::SPEED_RAMP_KMH)?;
    t.nums("speed_ramp_mps", &[metres_per_second(low), metres_per_second(high)]);
    let speed_help = help_speed(vehicle, params)?;
    if !speed_help.is_empty() {
        t.pairs("help_speed", &speed_help);
    }
    t.nums("lean_help", &params.xy_or("HelpIntensityLeanTuning", default::LEAN_HELP)?);
    t.num("lookahead_s", params.f_or("LookaheadTime", default::LOOKAHEAD)?);
    t.num("min_speed_mps", metres_per_second(params.f_or("MinSpeed", default::MIN_SPEED_KMH)?));
    t.num("acceleration", params.f_or("Acceleration", default::ACCELERATION)?);
    t.num("acceleration_time_s", params.f_or("AccelerationTime", default::ACCELERATION_TIME)?);
    t.nums("ground_range_m", &params.xy_or("GroundDistanceFactorRange", default::GROUND_RANGE)?);
    t.num("terrain_influence", params.f_or("TerrainInclinationInfluence", default::TERRAIN_INFLUENCE)?);
    let filtered = params.flag_or("bFilterTargetAngle", default::FILTER_TARGET)?;
    let timescale = params.f_or("timescale", default::FILTER_TIMESCALE)?;
    t.num("target_filter_timescale", if filtered { timescale } else { 0.0 });
    match set.lean {
        Some((limit, range)) => {
            t.num("lean_limit_rad", radians(limit));
            t.nums("lean_pitch_range_rad", &radians_pair(range));
        }
        None => {
            // Not pulled in: a limit at the steering bound changes nothing.
            t.num("lean_limit_rad", inert_limit);
            t.nums("lean_pitch_range_rad", &[0.0, 0.0]);
        }
    }
    Ok(())
}

/// The strength of the help against the speed (`HelpIntensitySpeedCurve`, km/h in the data), empty for the factor 1.
fn help_speed(vehicle: &Vehicle, params: &Node<'_>) -> Result<Vec<[f64; 2]>> {
    let Some(node) = params.get("HelpIntensitySpeedCurve") else { return Ok(Vec::new()) };
    let Some(curve) = runtime_curve(vehicle, &node)? else { return Ok(Vec::new()) };
    let points = curve.polyline().into_iter().map(|[speed, factor]| [metres_per_second(speed), factor]).collect();
    Ok(simplify(points, MAX_CURVE_POINTS))
}
