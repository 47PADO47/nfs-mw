//! The range checks of the engine, gearbox, brakes, steering, fall and wheelie parts of the `[bike]` table (`bike_parts.rs`).
//! The chassis, wheels and suspension are checked in `bike_check.rs`.

use super::ContentError;
use super::bike::{Brakes, Engine, Fall, Steering, Transmission, Wheelie};
use super::bike_check::{all_ranged, field, ranged};
use super::bike_parts::{Lean, Pitch};

/// A curve of `[x, y]` points: `min..=max` points, each number inside its range, `x` rising when `rising`.
fn curve(
    points: &[[f32; 2]],
    name: &'static str,
    count: (usize, usize),
    x: (f32, f32),
    y: (f32, f32),
) -> Result<(), ContentError> {
    if points.len() < count.0 || points.len() > count.1 {
        return Err(field(name, "the curve has too few or too many points"));
    }
    for &[px, py] in points {
        ranged(px, name, x.0, x.1)?;
        ranged(py, name, y.0, y.1)?;
    }
    if points.windows(2).any(|pair| pair[1][0] <= pair[0][0]) {
        return Err(field(name, "the first number of the points rises"));
    }
    Ok(())
}

pub(super) fn engine(engine: &Engine) -> Result<(), ContentError> {
    ranged(engine.peak_torque_nm, "bike.engine.peak_torque_nm", 1.0, 1_500.0)?;
    ranged(engine.friction_torque_nm, "bike.engine.friction_torque_nm", 0.0, 500.0)?;
    ranged(engine.idle_rpm, "bike.engine.idle_rpm", 300.0, 3_000.0)?;
    ranged(engine.max_rpm, "bike.engine.max_rpm", 1_500.0, 20_000.0)?;
    ranged(engine.moi_kgm2, "bike.engine.moi_kgm2", 0.0005, 5.0)?;
    ranged(engine.throttle_at_idle, "bike.engine.throttle_at_idle", 0.0, 1.0)?;
    ranged(engine.limiter_cut_time_s, "bike.engine.limiter_cut_time_s", 0.001, 1.0)?;
    if engine.max_rpm <= engine.idle_rpm {
        return Err(field("bike.engine.max_rpm", "the rev limit is above the idle"));
    }
    let name = "bike.engine.torque_curve";
    let points = &engine.torque_curve;
    curve(points, name, (2, 32), (0.0, 25_000.0), (0.0, 2.0))?;
    if points.iter().all(|point| point[1] <= 0.0) {
        return Err(field(name, "the engine makes torque somewhere"));
    }
    if points.last().is_some_and(|last| last[0] <= engine.idle_rpm) {
        return Err(field(name, "the torque curve reaches above the idle"));
    }
    if !engine.friction_curve.is_empty() {
        curve(&engine.friction_curve, "bike.engine.friction_curve", (2, 32), (0.0, 5.0), (0.0, 10.0))?;
    }
    Ok(())
}

pub(super) fn transmission(transmission: &Transmission) -> Result<(), ContentError> {
    let name = "bike.transmission.gears";
    let gears = &transmission.gears;
    if gears.is_empty() || gears.len() > 10 {
        return Err(field(name, "a bike has 1 to 10 forward gears"));
    }
    all_ranged(gears, name, 0.2, 20.0)?;
    if gears.windows(2).any(|pair| pair[1] >= pair[0]) {
        return Err(field(name, "each gear is lower than the gear before it"));
    }
    ranged(transmission.final_ratio, "bike.transmission.final_ratio", 0.5, 30.0)?;
    ranged(transmission.efficiency, "bike.transmission.efficiency", 0.5, 1.0)?;
    for (list, name) in [
        (&transmission.up_tolerance, "bike.transmission.up_tolerance"),
        (&transmission.down_tolerance, "bike.transmission.down_tolerance"),
    ] {
        if list.len() > gears.len() {
            return Err(field(name, "no more tolerances than gears"));
        }
        all_ranged(list, name, 0.1, 10.0)?;
    }
    let holds =
        [transmission.prevent_up_lean, transmission.prevent_down_lean, transmission.prevent_down_throttle].concat();
    all_ranged(&holds, "bike.transmission.prevent_up_lean", 0.0, 100.0)?;
    all_ranged(&transmission.launch_rpm, "bike.transmission.launch_rpm", 0.0, 1.5)?;
    let feather = &transmission.feather;
    let numbers = [
        feather.target_tolerance,
        feather.exit_tolerance,
        feather.throttle_cap,
        feather.min_slip,
        feather.rpm_raise,
        feather.release_factor,
    ];
    all_ranged(&numbers, "bike.transmission.feather", 0.0, 10.0)?;
    ranged(feather.delta_time_s, "bike.transmission.feather", 0.0001, 1.0)?;
    ranged(transmission.up_shift_time_s, "bike.transmission.up_shift_time_s", 0.0, 2.0)?;
    ranged(transmission.down_shift_time_s, "bike.transmission.down_shift_time_s", 0.0, 2.0)?;
    ranged(transmission.blip_throttle, "bike.transmission.blip_throttle", 0.0, 1.0)?;
    ranged(transmission.clutch_linearity, "bike.transmission.clutch_linearity", 0.1, 10.0)?;
    ranged(transmission.clutch_max_factor, "bike.transmission.clutch_max_factor", 0.1, 20.0)?;
    all_ranged(&transmission.slipper_range_nm, "bike.transmission.slipper_range_nm", 0.0, 1_000.0)?;
    ranged(transmission.slipper_intensity, "bike.transmission.slipper_intensity", 0.0, 1.0)?;
    ranged(transmission.release_speed_mps, "bike.transmission.release_speed_mps", 0.0, 100.0)?;
    all_ranged(&transmission.clutch_timing, "bike.transmission.clutch_timing", 0.0, 1.0)?;
    all_ranged(&transmission.throttle_timing, "bike.transmission.throttle_timing", 0.0, 1.0)?;
    all_ranged(&transmission.min_rpm_speed_scale_range, "bike.transmission.min_rpm_speed_scale_range", 0.0, 10.0)?;
    ranged(transmission.slipper_filter_speed, "bike.transmission.slipper_filter_speed", 0.0, 1_000.0)?;
    all_ranged(&transmission.force_downshift_brake_range, "bike.transmission.force_downshift_brake_range", 0.0, 1.0)?;
    ranged(transmission.force_downshift_duration_s, "bike.transmission.force_downshift_duration_s", 0.0, 60.0)?;
    ranged(transmission.force_downshift_factor, "bike.transmission.force_downshift_factor", 0.0, 10.0)?;
    ranged(transmission.force_downshift_linearity, "bike.transmission.force_downshift_linearity", 0.1, 10.0)?;
    all_ranged(
        &transmission.force_downshift_speed_range_mps,
        "bike.transmission.force_downshift_speed_range_mps",
        0.0,
        500.0,
    )?;
    ranged(transmission.feather_factor, "bike.transmission.feather_factor", 0.0, 2.0)?;
    ranged(transmission.feather_max_factor, "bike.transmission.feather_max_factor", 0.0, 1.0)?;
    Ok(())
}

pub(super) fn brakes(brakes: &Brakes) -> Result<(), ContentError> {
    ranged(brakes.front_force_n, "bike.brakes.front_force_n", 0.0, 30_000.0)?;
    ranged(brakes.rear_force_n, "bike.brakes.rear_force_n", 0.0, 30_000.0)?;
    if brakes.front_force_n + brakes.rear_force_n < 100.0 {
        return Err(field("bike.brakes.front_force_n", "the two brakes together make at least 100 N"));
    }
    all_ranged(
        &[brakes.joined_factor, brakes.joined_front, brakes.joined_rear],
        "bike.brakes.joined_factor",
        0.0,
        5.0,
    )?;
    let abs = &brakes.abs;
    all_ranged(&[abs.front_intensity, abs.rear_intensity], "bike.brakes.abs", 0.0, 1.0)?;
    all_ranged(&abs.speed_range_mps, "bike.brakes.abs", 0.0, 100.0)?;
    all_ranged(&[abs.front_slip, abs.rear_slip].concat(), "bike.brakes.abs", -5.0, 5.0)
}

fn lean(lean: &Lean) -> Result<(), ContentError> {
    let name = "bike.steering.lean";
    all_ranged(&[lean.k, lean.steer_k, lean.target_k], name, 1.0, 1.0e6)?;
    all_ranged(&[lean.damping_ratio, lean.steer_damping_ratio, lean.target_damping_ratio], name, 0.0, 20.0)?;
    all_ranged(&[lean.damping_multiplier, lean.time_scale, lean.rider_mass_scale], name, 0.0, 10.0)?;
    all_ranged(&[lean.time_scale_attenuation, lean.time_scale_sensitivity, lean.skid_linearity], name, 0.0, 20.0)?;
    all_ranged(&lean.skid_range_rad, name, 0.0, 3.2)?;
    all_ranged(&[lean.stopped_limit_rad], name, 0.0, 1.3)?;
    all_ranged(&[lean.stopped_speed_mps], name, 0.0, 100.0)?;
    all_ranged(&lean.blend_range_mps, name, 0.0, 100.0)?;
    ranged(lean.help_limit, name, 0.0, 10.0)?;
    all_ranged(&lean.help_stopped_range_mps, name, 0.0, 100.0)?;
    all_ranged(&lean.help_skid_range_rad, name, 0.0, 3.2)?;
    ranged(lean.help_skid_linearity, name, 0.0, 20.0)?;
    let curves = [
        (&lean.time_scale_speed, (0.0, 200.0), (0.0, 10.0)),
        (&lean.time_scale_input, (-1.0, 1.0), (0.0, 10.0)),
        (&lean.g_tuning, (0.0, 20.0), (0.0, 10.0)),
        (&lean.skid_reduction, (0.0, 200.0), (0.0, 1.0)),
        (&lean.help_speed, (0.0, 200.0), (0.0, 1.0)),
        (&lean.help_balance_speed, (0.0, 200.0), (0.0, 1.0)),
        (&lean.help_skid_reduction, (0.0, 200.0), (0.0, 1.0)),
        (&lean.help_skid_input, (-1.0, 1.0), (0.0, 1.0)),
    ];
    for (points, x, y) in curves {
        if !points.is_empty() {
            curve(points, name, (1, 32), x, y)?;
        }
    }
    Ok(())
}

pub(super) fn steering(steering: &Steering) -> Result<(), ContentError> {
    ranged(steering.max_angle_rad, "bike.steering.max_angle_rad", 0.1, 1.5)?;
    ranged(steering.response, "bike.steering.response", 1.0, 1.0e6)?;
    ranged(steering.damping_ratio, "bike.steering.damping_ratio", 0.0, 20.0)?;
    ranged(steering.max_lean_rad, "bike.steering.max_lean_rad", 0.2, 1.3)?;
    lean(&steering.lean)
}

pub(super) fn fall(fall: &Fall) -> Result<(), ContentError> {
    ranged(fall.lean_rad, "bike.fall.lean_rad", 0.2, 1.5)?;
    ranged(fall.pitch_rad, "bike.fall.pitch_rad", 0.2, 1.5)?;
    ranged(fall.speed_direction_rad, "bike.fall.speed_direction_rad", 0.2, 3.2)?;
    ranged(fall.angular_speed, "bike.fall.angular_speed", 0.5, 30.0)?;
    ranged(fall.min_speed_mps, "bike.fall.min_speed_mps", 0.0, 100.0)?;
    ranged(fall.block_ratio, "bike.fall.block_ratio", 0.0, 1.0)?;
    ranged(fall.block_min_speed_mps, "bike.fall.block_min_speed_mps", 0.0, 100.0)?;
    ranged(fall.block_time_s, "bike.fall.block_time_s", 0.0, 60.0)?;
    ranged(fall.block_wait_s, "bike.fall.block_wait_s", 0.0, 600.0)?;
    all_ranged(&[fall.lean_rise, fall.lean_fall], "bike.fall", 0.0, 1.0)?;
    ranged(fall.lean_duration_s, "bike.fall.lean_duration_s", 0.0, 10.0)?;
    all_ranged(&[fall.airborne_lean_rad, fall.airborne_pitch_rad, fall.highside_lean_rad], "bike.fall", 0.0, 3.2)?;
    ranged(fall.airborne_angular_speed, "bike.fall.airborne_angular_speed", 0.0, 30.0)?;
    Ok(())
}

fn pitch(pitch: &Pitch, name: &'static str) -> Result<(), ContentError> {
    ranged(pitch.deadzone, name, 0.0, 1.0)?;
    ranged(pitch.max_angle_rad, name, 0.0, 1.4)?;
    all_ranged(&pitch.speed_ramp_mps, name, 0.0, 100.0)?;
    all_ranged(&pitch.lean_help, name, 0.0, 1.0)?;
    all_ranged(&[pitch.lookahead_s, pitch.acceleration_time_s], name, 0.0, 10.0)?;
    ranged(pitch.min_speed_mps, name, 0.0, 100.0)?;
    ranged(pitch.acceleration, name, 0.0, 200.0)?;
    all_ranged(&pitch.ground_range_m, name, 0.0, 10.0)?;
    ranged(pitch.lean_limit_rad, name, 0.0, 1.3)?;
    all_ranged(&pitch.lean_pitch_range_rad, name, 0.0, 1.6)?;
    if !pitch.help_speed.is_empty() {
        curve(&pitch.help_speed, name, (1, 32), (0.0, 200.0), (0.0, 10.0))?;
    }
    Ok(())
}

pub(super) fn wheelie(wheelie: &Wheelie) -> Result<(), ContentError> {
    pitch(&wheelie.wheelie, "bike.wheelie.wheelie")?;
    pitch(&wheelie.stoppie, "bike.wheelie.stoppie")?;
    let name = "bike.wheelie";
    let angles = [wheelie.throttle_pitch_rad, wheelie.forced_throttle_pitch_rad, wheelie.brake_pitch_rad].concat();
    all_ranged(&angles, name, -1.6, 1.6)?;
    let speeds = [wheelie.throttle_pitch_speed, wheelie.brake_pitch_speed].concat();
    all_ranged(&speeds, name, -20.0, 20.0)
}
