//! `[bike.rider]`: where the rider sits as the weight, the lean and the speed change, and how his body is held to the bike
//! (`FRiderComponent` of the input handler; `docs/specs/ride6-rider-body-aero.md`, "Target place" and "Rider body"). The offsets
//! are RIDE 6 centimetres (x forward, y right, z up) and become metres in the physics space; speeds are km/h and become m/s.
//! The constructor defaults of the fields an asset leaves out are the ones read from the binary ([`defaults`]).

use anyhow::Result;

use super::chassis::handler;
use super::curve::{self, simplify};
use super::defaults;
use super::toml_text::Toml;
use super::units::{metres, metres_per_second, to_physics_space};
use super::vehicle::Vehicle;

/// The most points of the drag curve the manifest keeps.
const MAX_CURVE_POINTS: usize = 8;

/// A vector of RIDE 6 centimetres as metres in the physics space.
fn offset_m(ride6_cm: [f64; 3]) -> [f64; 3] {
    let v = to_physics_space(ride6_cm);
    [metres(v[0]), metres(v[1]), metres(v[2])]
}

pub fn write(t: &mut Toml, vehicle: &Vehicle) -> Result<()> {
    let handler = handler(vehicle)?;
    let rider = handler.req("Rider")?;
    if !rider.flag_or("SimulateRider", true)? {
        return Ok(());
    }
    t.table("bike.rider");
    let zero = [0.0; 3];
    t.nums("base_offset_m", &offset_m(rider.xyz_or("BaseOffset", zero)?));
    t.nums("lateral_offset_m", &offset_m(rider.xyz_or("LateralOffset", zero)?));
    t.nums("stopped_forward_offset_m", &offset_m(rider.xyz_or("StoppedForwardOffset", zero)?));
    t.nums("stopped_backward_offset_m", &offset_m(rider.xyz_or("StoppedBackwardOffset", zero)?));
    t.nums("stopped_lateral_offset_m", &offset_m(rider.xyz_or("StoppedLateralOffset", zero)?));
    let stopped = rider.xy_or("StoppedSpeedRange", [0.0, 0.0])?;
    t.nums("stopped_speed_range_mps", &[metres_per_second(stopped[0]), metres_per_second(stopped[1])]);
    t.num("weight_influence", rider.f_or("WeightInfluence", defaults::RIDER_WEIGHT_INFLUENCE)?);
    t.num("lean_influence", rider.f_or("LeanInfluence", defaults::RIDER_LEAN_INFLUENCE)?);
    let damp = rider.xy_or("DampSpeedRange", defaults::RIDER_DAMP_SPEED_RANGE_KMH)?;
    t.nums("damp_speed_range_mps", &[metres_per_second(damp[0]), metres_per_second(damp[1])]);
    t.num("damp_multiplier", rider.f_or("DampMultiplier", defaults::RIDER_DAMP_MULTIPLIER)?);
    t.flag("correct_gravity", rider.flag_or("bCorrectCOMOffsetWithGravity", false)?);
    // The data's scales are (x roll, y pitch, z yaw): the same vector as an offset, so the same swap gives (pitch, yaw, roll).
    t.nums("inertia_scale", &to_physics_space(rider.xyz_or("InertiaTensorScale", defaults::RIDER_UNIT_VECTOR)?));
    t.num("angular_k_n_m_per_rad", rider.f_or("RiderAngularK", defaults::RIDER_ANGULAR_K)?);
    t.num("angular_damp", rider.f_or("RiderAngularDamp", defaults::RIDER_ANGULAR_DAMP)?);
    t.num(
        "angular_damp_chassis_factor",
        rider.f_or("RiderAngularDampChassisFactor", defaults::RIDER_ANGULAR_DAMP_CHASSIS_FACTOR)?,
    );
    t.nums(
        "torque_scale",
        &to_physics_space(rider.xyz_or("TorqueAppliedOnBikeMultiplier", defaults::RIDER_UNIT_VECTOR)?),
    );
    t.num("drag_scale", rider.f_or("DragScale", defaults::RIDER_DRAG_SCALE)?);
    // `DragCurve` is read at the rider's longitudinal weight and `DragScaleCurve` at the tuck (`ride6-rider-round6.md`,
    // section 1). An unset curve is 0 for the first and 1 for the second, so only a curve the asset sets is written.
    if let Some(node) = rider.get("DragCurve")
        && let Some(curve) = curve::runtime_curve(vehicle, &node)?
    {
        t.pairs("drag_weight_curve", &simplify(curve.polyline(), MAX_CURVE_POINTS));
    }
    let Some(node) = rider.get("DragScaleCurve") else { return Ok(()) };
    let Some(curve) = curve::runtime_curve(vehicle, &node)? else { return Ok(()) };
    t.pairs("drag_curve", &simplify(curve.polyline(), MAX_CURVE_POINTS));
    Ok(())
}
