//! A vehicle in a dozen physical numbers (SI): what an outside model needs to size itself to a car without reading the
//! spec structs. Every number is derived from the [`VehicleSpec`]; a quantity the spec does not have is 0 and the field
//! says so.

use super::{Vehicle, VehicleSpec};
use crate::math::inch_to_m;

/// Air density (kg/m^3) at which the drag area is quoted: the engine's drag is a plain coefficient, not a drag area.
pub const REFERENCE_AIR_DENSITY: f32 = 1.225;

/// Which axles the engine drives.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DriveLayout {
    /// Front axle only.
    Front,
    /// Rear axle only.
    Rear,
    /// Both axles.
    All,
}

/// The physical summary of one vehicle. Axle pairs are front first.
#[derive(Clone, Debug, PartialEq)]
pub struct VehicleSummary {
    /// Mass of the whole vehicle (kg).
    pub mass_kg: f32,
    /// Distance between the axles (m).
    pub wheelbase_m: f32,
    /// Distance between the tyre centres of the front and the rear axle (m).
    pub track_m: [f32; 2],
    /// Height of the centre of gravity above the road at rest (m).
    pub cg_height_m: f32,
    /// Share of the weight on the front axle at rest, 0 to 1.
    pub front_weight_fraction: f32,
    /// Rolling radius of the front and the rear tyre (m).
    pub tyre_radius_m: [f32; 2],
    /// Section width of the front and the rear tyre (m).
    pub tyre_width_m: [f32; 2],
    /// The friction coefficient of the front and the rear tyre at the limit of grip (dimensionless).
    pub peak_friction: [f32; 2],
    /// The friction coefficient of the front and the rear tyre while it slides (dimensionless).
    pub sliding_friction: [f32; 2],
    /// Which axles are driven.
    pub drive: DriveLayout,
    /// The highest engine power on the torque curve (W), before forced induction and nitrous.
    pub max_power_w: f32,
    /// The engine speed of that power (rpm).
    pub max_power_rpm: f32,
    /// The rev limiter (rpm).
    pub redline_rpm: f32,
    /// The final drive ratio.
    pub final_drive: f32,
    /// The forward gear ratios from first gear, magnitudes (without the final drive).
    pub gear_ratios: Vec<f32>,
    /// Brake torque at one wheel of the front and the rear axle (N m).
    pub brake_torque_nm: [f32; 2],
    /// Drag area, `Cd * A` (m^2), such that `0.5 * 1.225 * area * speed^2` is the drag with the throttle open. 0 without drag.
    pub drag_area_m2: f32,
    /// Lift area (m^2) of the downforce: always 0, because the engine's downforce grows linearly with the speed, so no
    /// quadratic lift area describes it.
    pub downforce_area_m2: f32,
}

impl VehicleSummary {
    /// Summarize `spec`.
    pub fn of(spec: &VehicleSpec) -> Self {
        let chassis = &spec.chassis;
        let tyres = &spec.tires;
        let track = |axle: usize| (chassis.track_width[axle] - tyres.section_width[axle] * 0.001).max(0.0);
        let (max_power_w, max_power_rpm) = peak_power(spec);
        let drive = match (spec.transmission.front_driven(), spec.transmission.rear_driven()) {
            (true, false) => DriveLayout::Front,
            (false, _) => DriveLayout::Rear,
            (true, true) => DriveLayout::All,
        };
        VehicleSummary {
            mass_kg: spec.mass,
            wheelbase_m: chassis.wheel_base,
            track_m: [track(0), track(1)],
            cg_height_m: inch_to_m(chassis.roll_center),
            front_weight_fraction: (chassis.front_weight_bias * 0.01).clamp(0.0, 1.0),
            tyre_radius_m: [tyres.radius(0), tyres.radius(1)],
            tyre_width_m: [tyres.section_width[0] * 0.001, tyres.section_width[1] * 0.001],
            peak_friction: tyres.static_grip,
            sliding_friction: tyres.dynamic_grip,
            drive,
            max_power_w,
            max_power_rpm,
            redline_rpm: spec.engine.red_line,
            final_drive: spec.transmission.final_gear,
            gear_ratios: forward_ratios(spec),
            brake_torque_nm: [spec.brakes.brake_torque(0), spec.brakes.brake_torque(1)],
            drag_area_m2: 2.0 * spec.aero.drag_coefficient.max(0.0) / REFERENCE_AIR_DENSITY,
            downforce_area_m2: 0.0,
        }
    }
}

impl Vehicle {
    /// The physical summary of this vehicle (see [`VehicleSummary`]).
    pub fn summary(&self) -> VehicleSummary {
        VehicleSummary::of(&self.spec)
    }
}

/// The forward gears: the table's entries after reverse and neutral (see `TransmissionSpec`).
fn forward_ratios(spec: &VehicleSpec) -> Vec<f32> {
    spec.transmission.gear_ratio.iter().skip(2).map(|ratio| ratio.abs()).collect()
}

/// The highest power on the torque samples up to the rev limiter, and its engine speed. `(0, 0)` without a curve.
fn peak_power(spec: &VehicleSpec) -> (f32, f32) {
    let engine = &spec.engine;
    let samples = engine.torque.len();
    if samples < 2 {
        return (0.0, 0.0);
    }
    let step = (engine.max_rpm - engine.idle) / (samples - 1) as f32;
    let mut best = (0.0_f32, 0.0_f32);
    for index in 0..samples {
        let rpm = engine.idle + step * index as f32;
        if rpm > engine.rev_limit() {
            break;
        }
        let power = engine.torque_nm(rpm) * rpm * std::f32::consts::TAU / 60.0;
        if power > best.0 {
            best = (power, rpm);
        }
    }
    best
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_example_car_has_plausible_numbers() {
        let summary = Vehicle::new(VehicleSpec::example()).summary();
        assert!(summary.mass_kg > 800.0 && summary.mass_kg < 3000.0, "{}", summary.mass_kg);
        assert!(summary.wheelbase_m > 2.0 && summary.wheelbase_m < 3.5);
        assert!(summary.track_m.iter().all(|t| *t > 1.2 && *t < 2.0), "{:?}", summary.track_m);
        assert!(summary.front_weight_fraction > 0.3 && summary.front_weight_fraction < 0.7);
        assert!(summary.max_power_w > 50_000.0 && summary.max_power_rpm > 1000.0, "{summary:?}");
        assert!(summary.redline_rpm >= summary.max_power_rpm);
        assert!(summary.gear_ratios.len() >= 4 && summary.gear_ratios.windows(2).all(|w| w[0] > w[1]));
        assert!(summary.peak_friction.iter().all(|mu| *mu > 0.5));
        assert_eq!(summary.downforce_area_m2, 0.0);
    }

    #[test]
    fn the_drive_layout_follows_the_torque_split() {
        let mut spec = VehicleSpec::example();
        for (split, layout) in [(0.0, DriveLayout::Rear), (1.0, DriveLayout::Front), (0.4, DriveLayout::All)] {
            spec.transmission.torque_split = split;
            assert_eq!(VehicleSummary::of(&spec).drive, layout, "split {split}");
        }
    }

    #[test]
    fn a_spec_without_a_torque_curve_has_no_power() {
        let mut spec = VehicleSpec::example();
        spec.engine.torque.clear();
        let summary = VehicleSummary::of(&spec);
        assert_eq!((summary.max_power_w, summary.max_power_rpm), (0.0, 0.0));
    }
}
