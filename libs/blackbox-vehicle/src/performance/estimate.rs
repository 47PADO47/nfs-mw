//! Analytical action-planning performance; independent of a physical launch trial.
//! Spec: `docs/specs/ai-pursuit-acceleration-handoff.md`.

use crate::drivetrain::{GEAR_FIRST, Powertrain};
use crate::math::FT_LB_TO_NM;
use crate::vehicle::VehicleSpec;

use super::{GRAVITY, Performance, TABLE_SIZE};

/// Estimates the action's performance curve from gearing, average inducted torque and drag.
/// This does not step a vehicle or modify its physical engine or controls.
pub fn estimate_ai(spec: &VehicleSpec) -> Option<Performance> {
    let mass = spec.mass;
    let radius = spec.tires.wheel_diameter(1) * 0.5;
    let drag = spec.aero.drag_coefficient;
    if !mass.is_finite() || mass <= 0.0 || !radius.is_finite() || radius <= 1e-6 || !drag.is_finite() || drag < 0.0 {
        return None;
    }
    let torque = average_inducted_torque(spec)?;
    let mut points: Vec<(f32, f32)> = Vec::new();
    for gear in GEAR_FIRST..spec.transmission.gear_ratio.len() {
        let ratio = spec.transmission.final_gear * spec.transmission.gear_ratio[gear];
        let efficiency = spec.transmission.efficiency(gear);
        if !ratio.is_finite() || ratio <= 0.0 || !efficiency.is_finite() || efficiency < 0.0 {
            return None;
        }
        let mut speed = spec.engine.red_line * (std::f32::consts::TAU / 60.0) * radius / ratio;
        let force = torque * ratio * efficiency / radius;
        let mut acceleration = (force - drag * speed * speed) / mass;
        if !speed.is_finite() || !acceleration.is_finite() {
            return None;
        }
        if points.last().is_some_and(|&(previous, _)| speed <= previous) {
            return None;
        }
        if acceleration < 0.0 {
            let &(previous_speed, previous_accel) = points.last()?;
            if previous_accel <= 0.0 {
                break;
            }
            let weight = 1.0 - previous_accel / (previous_accel - acceleration);
            speed = previous_speed + (speed - previous_speed) * weight;
            acceleration = 0.0;
        }
        points.push((speed, acceleration));
        if acceleration == 0.0 {
            break;
        }
    }
    let top_speed = points.last()?.0;
    if top_speed <= 0.0 {
        return None;
    }
    let acceleration = std::array::from_fn(|i| interpolate(&points, top_speed * i as f32 / (TABLE_SIZE - 1) as f32));
    let start_grip = spec.tires.static_grip[0].min(spec.tires.static_grip[1]);
    let downforce = top_speed * 2.0 * spec.aero.aero_coefficient * 1000.0;
    let end_grip = start_grip * (GRAVITY + downforce / mass) / GRAVITY;
    if !start_grip.is_finite() || !end_grip.is_finite() {
        return None;
    }
    Some(Performance { top_speed, acceleration, start_grip, end_grip })
}

fn average_inducted_torque(spec: &VehicleSpec) -> Option<f32> {
    let engine = &spec.engine;
    if engine.torque.len() < 2
        || !engine.idle.is_finite()
        || !engine.max_rpm.is_finite()
        || !engine.red_line.is_finite()
        || engine.max_rpm <= engine.idle
        || engine.red_line <= engine.idle
    {
        return None;
    }
    let peak = Powertrain::new(
        engine.clone(),
        spec.transmission.clone(),
        spec.induction,
        spec.nos,
        spec.tires.average_radius(),
    )
    .peak_torque()
    .1;
    let step = (engine.max_rpm - engine.idle) / (engine.torque.len() - 1) as f32;
    let (mut rpm, mut total, mut count) = (engine.idle, 0.0, 0);
    for &torque in &engine.torque {
        if rpm >= engine.red_line {
            break;
        }
        if rpm >= peak {
            total += torque * (1.0 + spec.induction.full_boost(rpm, engine.idle, engine.red_line));
            count += 1;
        }
        rpm += step;
    }
    if count == 0 || !total.is_finite() || total <= 0.0 {
        return None;
    }
    Some(total / count as f32 * FT_LB_TO_NM)
}

fn interpolate(points: &[(f32, f32)], speed: f32) -> f32 {
    for pair in points.windows(2) {
        let [(left, a), (right, b)] = pair else { unreachable!() };
        if speed < *left {
            return *a;
        }
        if speed < *right {
            return a + (b - a) * (speed - left) / (right - left);
        }
    }
    points.last().map_or(0.0, |&(_, acceleration)| acceleration)
}

#[cfg(test)]
#[path = "estimate_tests.rs"]
mod tests;
