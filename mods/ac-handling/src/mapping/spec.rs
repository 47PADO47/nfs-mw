//! The car as the mapping reads it: `BbVehicleSpec` of docs/modding/api-handling.md in plain `f64`, axle pairs front then rear.
//! Only the fields listed in docs/specs/ac-handling/parameter-mapping.md section 2 are used in stage 2; the rest is carried so
//! the later stages and the scenario car need no second type.

use blackbox_modding::sdk::{DriveLayout, VehicleSpec};

/// Which axles are driven.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Drive {
    /// The front axle.
    Front,
    /// The rear axle.
    #[default]
    Rear,
    /// Both axles.
    All,
}

impl From<DriveLayout> for Drive {
    fn from(layout: DriveLayout) -> Self {
        match layout {
            DriveLayout::Front => Drive::Front,
            DriveLayout::Rear => Drive::Rear,
            DriveLayout::All => Drive::All,
        }
    }
}

/// The host's best estimate of a car, in SI units.
#[derive(Debug, Clone, PartialEq)]
pub struct CarSpec {
    /// Changes whenever the car changes.
    pub revision: u32,
    /// Mass, kg.
    pub mass_kg: f64,
    /// Distance between the axles, m.
    pub wheelbase_m: f64,
    /// Track of the front and the rear axle, m.
    pub track_m: [f64; 2],
    /// Height of the centre of gravity, m.
    pub cg_height_m: f64,
    /// Share of the weight on the front axle.
    pub front_weight_fraction: f64,
    /// Rolling radius of the front and the rear tyre, m.
    pub tyre_radius_m: [f64; 2],
    /// Section width of the front and the rear tyre, m.
    pub tyre_width_m: [f64; 2],
    /// Peak friction on the game's own scale, front and rear.
    pub peak_friction: [f64; 2],
    /// Sliding friction on the game's own scale, front and rear.
    pub sliding_friction: [f64; 2],
    /// Driven axles.
    pub drive: Drive,
    /// Highest engine power, W.
    pub max_power_w: f64,
    /// Engine speed of that power, rpm.
    pub max_power_rpm: f64,
    /// Rev limiter, rpm.
    pub redline_rpm: f64,
    /// Final drive ratio.
    pub final_drive: f64,
    /// Forward gear ratios from first gear.
    pub gear_ratios: Vec<f64>,
    /// Brake torque at one wheel, front and rear, N m.
    pub brake_torque_nm: [f64; 2],
    /// Drag area `Cd * A`, m^2.
    pub drag_area_m2: f64,
    /// Lift area of the downforce, m^2.
    pub downforce_area_m2: f64,
}

impl CarSpec {
    /// The reference car `ref_sport` of parameter-mapping.md section 2, used by the test vectors and the scenarios.
    pub fn ref_sport() -> CarSpec {
        CarSpec {
            revision: 1,
            mass_kg: 1450.0,
            wheelbase_m: 2.45,
            track_m: [1.52, 1.50],
            cg_height_m: 0.50,
            front_weight_fraction: 0.52,
            tyre_radius_m: [0.33, 0.34],
            tyre_width_m: [0.235, 0.265],
            peak_friction: [2.075, 2.075],
            sliding_friction: [1.85, 1.85],
            drive: Drive::Rear,
            max_power_w: 300_000.0,
            max_power_rpm: 7000.0,
            redline_rpm: 8000.0,
            final_drive: 3.5,
            gear_ratios: vec![3.6, 2.2, 1.6, 1.25, 1.0, 0.82],
            brake_torque_nm: [2000.0, 800.0],
            drag_area_m2: 0.65,
            downforce_area_m2: 0.0,
        }
    }

    /// The spec the host gave, widened to `f64`.
    pub fn from_sdk(spec: &VehicleSpec) -> CarSpec {
        let pair = |values: [f32; 2]| [f64::from(values[0]), f64::from(values[1])];
        CarSpec {
            revision: spec.revision,
            mass_kg: f64::from(spec.mass_kg),
            wheelbase_m: f64::from(spec.wheelbase_m),
            track_m: pair(spec.track_m),
            cg_height_m: f64::from(spec.cg_height_m),
            front_weight_fraction: f64::from(spec.front_weight_fraction),
            tyre_radius_m: pair(spec.tyre_radius_m),
            tyre_width_m: pair(spec.tyre_width_m),
            peak_friction: pair(spec.peak_friction),
            sliding_friction: pair(spec.sliding_friction),
            drive: spec.drive.into(),
            max_power_w: f64::from(spec.max_power_w),
            max_power_rpm: f64::from(spec.max_power_rpm),
            redline_rpm: f64::from(spec.redline_rpm),
            final_drive: f64::from(spec.final_drive),
            gear_ratios: spec.gear_ratios.iter().map(|ratio| f64::from(*ratio)).collect(),
            brake_torque_nm: pair(spec.brake_torque_nm),
            drag_area_m2: f64::from(spec.drag_area_m2),
            downforce_area_m2: f64::from(spec.downforce_area_m2),
        }
    }

    /// The static load on one front wheel and one rear wheel, N (the log line and the test vectors).
    pub fn static_corner_loads(&self) -> [f64; 2] {
        let weight = self.mass_kg * 9.81;
        [weight * self.front_weight_fraction / 2.0, weight * (1.0 - self.front_weight_fraction) / 2.0]
    }

    /// The car id of parameter-mapping.md section 2.1: `spec-<kg>-<cm>-<kW>`, each rounded to the nearest integer (ties away from
    /// zero), for example `spec-1450-245-300`. It is the file name of the car's override (`tuning/<id>.toml`). A field that is
    /// not finite, a mass or wheelbase at or below 0 or a negative power give `spec-invalid`, and no file is looked up.
    pub fn key_name(&self) -> String {
        let usable = self.mass_kg.is_finite()
            && self.wheelbase_m.is_finite()
            && self.max_power_w.is_finite()
            && self.mass_kg > 0.0
            && self.wheelbase_m > 0.0
            && self.max_power_w >= 0.0;
        if !usable {
            return "spec-invalid".to_owned();
        }
        let mass = self.mass_kg.round() as i64;
        let wheelbase = (self.wheelbase_m * 100.0).round() as i64;
        let power = (self.max_power_w / 1000.0).round() as i64;
        format!("spec-{mass}-{wheelbase}-{power}")
    }
}
