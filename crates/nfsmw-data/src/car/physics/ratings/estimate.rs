//! The original menu's bounded longitudinal estimate. This never steps a live vehicle.

use blackbox_vehicle::VehicleSpec;
use blackbox_vehicle::drivetrain::GEAR_FIRST;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Estimated {
    pub top_speed: f32,
    pub time_to_100: f32,
}

const RPM_TO_RAD: f32 = 0.104_719_76;
const RAD_TO_RPM: f32 = 9.549_296;
const TORQUE_TO_NM: f32 = 1.3558;
const MPH_TO_MPS: f32 = 0.44703;
const TARGET_SPEED: f32 = 44.703;

pub(super) fn estimate(spec: &VehicleSpec) -> Option<Estimated> {
    let engine = &spec.engine;
    let transmission = &spec.transmission;
    let radius = spec.tires.wheel_diameter(1) * 0.5;
    if ![spec.mass, radius, transmission.final_gear, engine.idle, engine.red_line, engine.max_rpm]
        .into_iter()
        .all(|value| value.is_finite() && value > 0.0)
        || engine.red_line <= engine.idle
        || engine.max_rpm <= engine.idle
        || engine.torque.len() < 2
        || transmission.gear_ratio.len() <= GEAR_FIRST
        || !engine.torque.iter().all(|value| value.is_finite())
        || !spec.aero.drag_coefficient.is_finite()
    {
        return None;
    }
    if transmission.gear_ratio[GEAR_FIRST..].iter().any(|ratio| !ratio.is_finite() || *ratio <= 0.0) {
        return None;
    }
    let shifts = shift_points(spec);
    let peak_rpm = peak_rpm(spec);
    let cap = engine.speed_limiter[0] * MPH_TO_MPS;
    let first_rpm = engine.idle * RPM_TO_RAD;
    let rpm_range = 1.0 - engine.idle / engine.red_line;
    let (mut speed, mut elapsed, mut step, mut gear) = (0.0f32, 0.0f32, 0.125f32, GEAR_FIRST);
    let (mut time_to_100, mut top_speed) = (0.0f32, 0.0f32);
    let last = transmission.gear_ratio.len() - 1;
    while elapsed < 120.0 {
        let ratio = transmission.final_gear * transmission.gear_ratio[gear];
        let mut rpm =
            ((speed / radius * ratio * rpm_range + first_rpm) * RAD_TO_RPM).clamp(engine.idle, engine.red_line);
        if gear == GEAR_FIRST {
            rpm = rpm.max(peak_rpm);
        }
        let mut force = engine.torque_ftlb(rpm)
            * TORQUE_TO_NM
            * (1.0 + spec.induction.full_boost(rpm, engine.idle, engine.red_line))
            * ratio;
        if elapsed < spec.nos.nos_capacity && speed > 5.0 {
            force *= 1.0 + spec.nos.torque_boost;
        }
        let acceleration = force / radius / spec.mass;
        let drag = (spec.aero.drag_coefficient * speed * speed).abs() / spec.mass;
        speed = speed + acceleration * step - drag * step;
        if !speed.is_finite() {
            return None;
        }
        if speed >= TARGET_SPEED && time_to_100 <= 0.0 {
            time_to_100 = elapsed;
            step = 1.0;
        }
        if cap > 0.0 && speed >= cap {
            top_speed = cap;
        }
        if (cap <= 0.0 || speed < cap)
            && gear == last
            && (acceleration < drag || rpm >= engine.red_line)
            && top_speed <= 0.0
        {
            top_speed = speed;
        }
        elapsed += step;
        if top_speed > 0.0 && time_to_100 > 0.0 {
            break;
        }
        if rpm >= shifts[gear] {
            gear = (gear + 1).min(last);
        }
    }
    if gear == last || top_speed <= 0.0 {
        top_speed = speed;
    }
    if !top_speed.is_finite() || top_speed <= 0.0 || time_to_100 <= 0.0 {
        return None;
    }
    Some(Estimated { top_speed, time_to_100 })
}

fn peak_rpm(spec: &VehicleSpec) -> f32 {
    let engine = &spec.engine;
    let mut best = 0;
    for i in 1..engine.torque.len() {
        if engine.torque[i] > engine.torque[best] {
            best = i;
        }
    }
    (engine.idle + (engine.max_rpm - engine.idle) * best as f32 / (engine.torque.len() - 1) as f32)
        .clamp(engine.idle, engine.red_line)
}

fn shift_points(spec: &VehicleSpec) -> Vec<f32> {
    let engine = &spec.engine;
    let ratios = &spec.transmission.gear_ratio;
    let mut points = vec![0.0; ratios.len()];
    for gear in GEAR_FIRST..ratios.len() - 1 {
        let fraction = ratios[gear + 1] / ratios[gear];
        let mut rpm = (engine.red_line + engine.idle) * 0.5;
        while rpm < engine.red_line {
            if engine.torque_ftlb(rpm * fraction) * fraction > engine.torque_ftlb(rpm) {
                break;
            }
            rpm += 50.0;
        }
        points[gear] = match rpm < engine.red_line {
            true => rpm,
            false => engine.red_line - 100.0,
        };
    }
    if let Some(top) = points.last_mut() {
        *top = engine.red_line;
    }
    points
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shift_points_retain_first_equal_torque_peak_and_use_the_native_fallback() {
        let mut spec = VehicleSpec::example();
        spec.engine.idle = 1000.0;
        spec.engine.max_rpm = 9000.0;
        spec.engine.red_line = 8000.0;
        spec.engine.torque = vec![100.0, 300.0, 300.0, 100.0];
        assert!((peak_rpm(&spec) - (1000.0 + 8000.0 / 3.0)).abs() < 0.001);
        spec.engine.torque = vec![300.0; 4];
        let points = shift_points(&spec);
        assert!(points[GEAR_FIRST..points.len() - 1].iter().all(|rpm| *rpm == 7900.0));
        assert_eq!(points.last(), Some(&8000.0));
    }

    #[test]
    fn constant_force_uses_beginning_of_step_time_and_last_gear_cap_override() {
        let mut spec = VehicleSpec::example();
        spec.mass = 1000.0;
        spec.tires.rim_size = [0.0; 2];
        spec.tires.section_width = [500.0; 2];
        spec.tires.aspect_ratio = [100.0; 2];
        spec.engine.torque = vec![1000.0; 2];
        spec.engine.red_line = 100_000.0;
        spec.engine.max_rpm = 100_000.0;
        spec.engine.speed_limiter = [100.0, 0.0];
        spec.transmission.gear_ratio = vec![1.0, 0.0, 1.0];
        spec.transmission.final_gear = 1.0;
        spec.nos.nos_capacity = 0.0;
        spec.aero.drag_coefficient = 0.0;
        let result = estimate(&spec).unwrap();
        // Constant acceleration is 1000 ft-lb * 1.3558 / .5m / 1000kg.
        // The 132nd 1/8-second step crosses 100mph; the stored time is its start.
        assert_eq!(result.time_to_100, 16.375);
        assert!((result.top_speed - 44.7414).abs() < 0.001, "{result:?}");
        // This menu estimator intentionally ignores tire grip and driveline efficiency.
        spec.tires.static_grip = [0.0; 2];
        spec.transmission.gear_efficiency.fill(0.0);
        assert_eq!(estimate(&spec), Some(result));
        spec.mass = f32::NAN;
        assert_eq!(estimate(&spec), None);
    }
}
