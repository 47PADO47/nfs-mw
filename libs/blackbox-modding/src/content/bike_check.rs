//! The range checks of the `[bike]` table (`bike.rs`). A number outside its range, an array that is not rising where it has
//! to, or a value that is not finite refuses the car with the key named. The ranges are wide: they catch a wheel of 3 m or
//! a rake of 26 radians, not an unusual bike.

use super::ContentError;
use super::bike::{Bike, Chassis, Damper, Suspension, Wheel};
use super::bike_check_parts as parts;

const REASON: &str = "must be a number inside the range the docs give";

pub(super) fn field(field: &'static str, reason: &'static str) -> ContentError {
    ContentError::Field { field, reason }
}

/// `value` when it is finite and inside `low..=high`.
pub(super) fn ranged(value: f32, name: &'static str, low: f32, high: f32) -> Result<f32, ContentError> {
    if !value.is_finite() || !(low..=high).contains(&value) {
        return Err(field(name, REASON));
    }
    Ok(value)
}

/// Every number of `values` inside `low..=high`.
pub(super) fn all_ranged(values: &[f32], name: &'static str, low: f32, high: f32) -> Result<(), ContentError> {
    values.iter().try_for_each(|&value| ranged(value, name, low, high).map(|_| ()))
}

/// Check the table. A car with no table has none.
pub(super) fn check(bike: Option<Bike>) -> Result<Option<Bike>, ContentError> {
    let Some(bike) = bike else { return Ok(None) };
    chassis(&bike.chassis)?;
    wheel(&bike.front, WHEEL_FRONT)?;
    wheel(&bike.rear, WHEEL_REAR)?;
    suspension(&bike.front_suspension, SUSPENSION_FRONT)?;
    suspension(&bike.rear_suspension, SUSPENSION_REAR)?;
    parts::engine(&bike.engine)?;
    parts::transmission(&bike.transmission)?;
    parts::brakes(&bike.brakes)?;
    parts::steering(&bike.steering)?;
    parts::fall(&bike.fall)?;
    if let Some(wheelie) = &bike.wheelie {
        parts::wheelie(wheelie)?;
    }
    if let Some(tcs) = &bike.tcs {
        all_ranged(&tcs.slip, "bike.tcs.slip", 0.0, 5.0)?;
        all_ranged(&tcs.speed_range_mps, "bike.tcs.speed_range_mps", 0.0, 100.0)?;
    }
    bike.rider.check()?;
    ranged(bike.gyro_roll_scale, "bike.gyro_roll_scale", 0.0, 10.0)?;
    ranged(bike.gyro_yaw_scale, "bike.gyro_yaw_scale", 0.0, 10.0)?;
    ranged(bike.drag, "bike.drag", 0.0, 3.0)?;
    Ok(Some(bike))
}

fn chassis(chassis: &Chassis) -> Result<(), ContentError> {
    ranged(chassis.mass_kg, "bike.chassis.mass_kg", 20.0, 1_000.0)?;
    ranged(chassis.rider_mass_kg, "bike.chassis.rider_mass_kg", 0.0, 300.0)?;
    all_ranged(&chassis.rider_offset_m, "bike.chassis.rider_offset_m", -2.0, 2.0)?;
    all_ranged(&chassis.rider_forward_shift_m, "bike.chassis.rider_forward_shift_m", -1.0, 1.0)?;
    all_ranged(&chassis.rider_backward_shift_m, "bike.chassis.rider_backward_shift_m", -1.0, 1.0)?;
    ranged(chassis.rider_weight_rate, "bike.chassis.rider_weight_rate", 0.0, 100.0)?;
    ranged(chassis.rider_k_n_per_m, "bike.chassis.rider_k_n_per_m", 0.0, 1.0e7)?;
    ranged(chassis.rider_damp, "bike.chassis.rider_damp", 0.0, 1.0e6)?;
    for half in chassis.half_dimensions_m {
        ranged(half, "bike.chassis.half_dimensions_m", 0.05, 5.0)?;
    }
    for scale in chassis.tensor_scale {
        ranged(scale, "bike.chassis.tensor_scale", 0.01, 10.0)?;
    }
    ranged(chassis.cg_height_m, "bike.chassis.cg_height_m", 0.1, 2.0)?;
    ranged(chassis.front_weight, "bike.chassis.front_weight", 0.2, 0.8)?;
    ranged(chassis.wheelbase_m, "bike.chassis.wheelbase_m", 0.6, 4.0)?;
    ranged(chassis.rake_rad, "bike.chassis.rake_rad", 0.0, 0.9)?;
    ranged(chassis.trail_m, "bike.chassis.trail_m", 0.0, 0.5)?;
    Ok(())
}

/// The key names of one wheel's table, so an error names the wheel it is about.
#[derive(Clone, Copy)]
struct WheelKeys {
    radius: &'static str,
    width: &'static str,
    toroid: &'static str,
    mass: &'static str,
    inertia: &'static str,
    rolling: &'static str,
    grip: &'static str,
    tyre: &'static str,
    thermal: &'static str,
    stiffness: &'static str,
    deformation: &'static str,
    shape: &'static str,
    slip: &'static str,
}

const WHEEL_FRONT: WheelKeys = WheelKeys {
    radius: "bike.front.radius_m",
    width: "bike.front.width_m",
    toroid: "bike.front.toroid_factor",
    mass: "bike.front.mass_kg",
    inertia: "bike.front.inertia_kgm2",
    rolling: "bike.front.rolling",
    grip: "bike.front.lateral_grip_limit",
    tyre: "bike.front.tyre",
    thermal: "bike.front.thermal",
    stiffness: "bike.front.tire_stiffness_n_per_m",
    deformation: "bike.front.max_tire_deformation_m",
    shape: "bike.front.compression_shape_factor",
    slip: "bike.front.slip_correction",
};

const WHEEL_REAR: WheelKeys = WheelKeys {
    radius: "bike.rear.radius_m",
    width: "bike.rear.width_m",
    toroid: "bike.rear.toroid_factor",
    mass: "bike.rear.mass_kg",
    inertia: "bike.rear.inertia_kgm2",
    rolling: "bike.rear.rolling",
    grip: "bike.rear.lateral_grip_limit",
    tyre: "bike.rear.tyre",
    thermal: "bike.rear.thermal",
    stiffness: "bike.rear.tire_stiffness_n_per_m",
    deformation: "bike.rear.max_tire_deformation_m",
    shape: "bike.rear.compression_shape_factor",
    slip: "bike.rear.slip_correction",
};

/// The largest magnitude a tyre coefficient may have: a number beyond it is a typing mistake, and its forces would
/// overflow `f32`.
const COEFFICIENT_LIMIT: f32 = 1.0e7;

fn wheel(wheel: &Wheel, keys: WheelKeys) -> Result<(), ContentError> {
    ranged(wheel.radius_m, keys.radius, 0.1, 0.8)?;
    ranged(wheel.width_m, keys.width, 0.03, 0.5)?;
    ranged(wheel.toroid_factor, keys.toroid, 0.0, 1.0)?;
    ranged(wheel.mass_kg, keys.mass, 0.5, 80.0)?;
    ranged(wheel.inertia_kgm2, keys.inertia, 0.01, 20.0)?;
    let limits = [(0.0, 0.2), (0.0, 0.1), (0.0, 0.01)];
    for (value, (low, high)) in wheel.rolling.into_iter().zip(limits) {
        ranged(value, keys.rolling, low, high)?;
    }
    ranged(wheel.lateral_grip_limit, keys.grip, 0.3, 4.0)?;
    ranged(wheel.rolling_slip, keys.rolling, 0.0, 10.0)?;
    ranged(wheel.rolling_slip_linearity, keys.rolling, 0.0, 10.0)?;
    if let Some(thermal) = &wheel.thermal {
        thermal.check(keys.thermal)?;
    }
    ranged(wheel.tire_stiffness_n_per_m, keys.stiffness, 0.0, 10_000_000.0)?;
    ranged(wheel.max_tire_deformation_m, keys.deformation, 0.0, 0.2)?;
    ranged(wheel.compression_shape_factor, keys.shape, 0.0, 1.0)?;
    ranged(wheel.slip_correction, keys.slip, 0.0, 10.0)?;
    ranged(wheel.rest_slip_correction, keys.slip, 0.0, 10.0)?;
    ranged(wheel.rest_speed_threshold_mps, keys.slip, 0.0, 100.0)?;
    let tyre = &wheel.tyre;
    let all = tyre.a.iter().chain(&tyre.b).chain(&tyre.c);
    for &coefficient in all {
        ranged(coefficient, keys.tyre, -COEFFICIENT_LIMIT, COEFFICIENT_LIMIT)?;
    }
    // The shape factors `a0` (lateral) and `b0` (longitudinal) are the C of the formula: it is positive and below 3.
    ranged(tyre.a[0], keys.tyre, 0.01, 3.0)?;
    ranged(tyre.b[0], keys.tyre, 0.01, 3.0)?;
    Ok(())
}

#[derive(Clone, Copy)]
struct SuspensionKeys {
    travel: &'static str,
    spring: &'static str,
    top_out: &'static str,
    damper: &'static str,
    leverage: &'static str,
    extra: &'static str,
}

const SUSPENSION_FRONT: SuspensionKeys = SuspensionKeys {
    travel: "bike.front_suspension.max_travel_m",
    spring: "bike.front_suspension.spring_n_per_m",
    top_out: "bike.front_suspension.top_out_stiffness_n_per_m",
    damper: "bike.front_suspension.damper",
    leverage: "bike.front_suspension.leverage",
    extra: "bike.front_suspension",
};

const SUSPENSION_REAR: SuspensionKeys = SuspensionKeys {
    travel: "bike.rear_suspension.max_travel_m",
    spring: "bike.rear_suspension.spring_n_per_m",
    top_out: "bike.rear_suspension.top_out_stiffness_n_per_m",
    damper: "bike.rear_suspension.damper",
    leverage: "bike.rear_suspension.leverage",
    extra: "bike.rear_suspension",
};

fn suspension(suspension: &Suspension, keys: SuspensionKeys) -> Result<(), ContentError> {
    ranged(suspension.max_travel_m, keys.travel, 0.01, 0.6)?;
    ranged(suspension.spring_n_per_m, keys.spring, 100.0, 500_000.0)?;
    ranged(suspension.top_out_stiffness_n_per_m, keys.top_out, 0.0, 1_000_000.0)?;
    ranged(suspension.top_out_length_m, keys.extra, 0.0, 0.5)?;
    ranged(suspension.rest_offset_m, keys.extra, 0.0, 0.3)?;
    ranged(suspension.max_speed_mps, keys.extra, 0.0, 50.0)?;
    ranged(suspension.hydraulic.stiffness_n_per_m, keys.extra, 0.0, 1_000_000.0)?;
    for term in [
        (suspension.progressive.start, suspension.progressive.factor, suspension.progressive.linearity),
        (suspension.hydraulic.start, suspension.hydraulic.factor, suspension.hydraulic.linearity),
    ] {
        ranged(term.0, keys.extra, 0.0, 1.0)?;
        ranged(term.1, keys.extra, 0.0, 100.0)?;
        ranged(term.2, keys.extra, 0.0, 10.0)?;
    }
    let c = &suspension.coupling;
    all_ranged(&[c.chain, c.traction, c.brake], keys.extra, -1_000.0, 1_000.0)?;
    all_ranged(&[c.chain_range, c.traction_range, c.brake_range].concat(), keys.extra, -10.0, 10.0)?;
    damper(&suspension.damper, keys.damper)?;
    leverage(&suspension.leverage, keys.leverage)
}

fn damper(damper: &Damper, name: &'static str) -> Result<(), ContentError> {
    for slope in [damper.slow_bump, damper.fast_bump, damper.slow_rebound, damper.fast_rebound] {
        ranged(slope, name, 0.0, 100_000.0)?;
    }
    for threshold in [damper.bump_threshold, damper.rebound_threshold] {
        ranged(threshold, name, 0.001, 5.0)?;
    }
    Ok(())
}

/// The leverage curve: up to 16 points `[share of travel, ratio]`, the share rising from 0 to 1 at most, the ratio from 0.1
/// to 10. An empty list is a spring that acts on the wheel directly.
fn leverage(points: &[[f32; 2]], name: &'static str) -> Result<(), ContentError> {
    if points.len() > 16 {
        return Err(field(name, "a leverage curve has up to 16 points"));
    }
    for &[share, ratio] in points {
        ranged(share, name, 0.0, 1.0)?;
        ranged(ratio, name, 0.1, 10.0)?;
    }
    if points.windows(2).any(|pair| pair[1][0] <= pair[0][0]) {
        return Err(field(name, "the share of travel of the points rises"));
    }
    Ok(())
}
