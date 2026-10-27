//! A car described by its own absolute stats, and the pure functions that turn the stats into a [`VehicleSpec`].
//!
//! [`CarStats`] says what a car is in SI units: its mass and weight distribution, the torque of its engine at each engine
//! speed, its gear ratios, tyre grip, brake torque and so on. [`VehicleSpec::from_stats`] builds the whole physical car
//! from them; a stat the caller leaves at its default takes the engine's generic value (documented where it is used),
//! never the value of some other car. [`upgrade`] returns the stats of the same car after shop upgrades, as a function
//! of its own stats, and [`handling_index`] scores its handling from them.

mod build;
mod rating;
mod upgrades;

pub use build::{SAMPLES, solve_drag};
pub use rating::{HANDLING_RANGE, handling_index};
pub use upgrades::{Category, MAX_LEVEL, upgrade};

use crate::VehicleSpec;

/// Which wheels the engine drives.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Drive {
    Rear,
    Front,
    All,
}

/// A car's stats. Axle pairs are `[front, rear]`. All units are SI unless a name says otherwise.
#[derive(Debug, Clone, PartialEq)]
pub struct CarStats {
    /// Mass of the whole car, kg.
    pub mass_kg: f32,
    /// Share of the weight on the front axle, percent.
    pub front_weight_pct: f32,
    /// Height of the centre of gravity above the road, m.
    pub cg_height_m: f32,
    pub wheelbase_m: f32,
    /// Distance between the tyre centres of an axle, m.
    pub track_m: [f32; 2],
    pub tire_radius_m: [f32; 2],
    pub tire_width_m: [f32; 2],
    /// Gap between the underside of the body and the road, m.
    pub ride_height_m: f32,
    pub idle_rpm: f32,
    pub redline_rpm: f32,
    /// `[rpm, N m]` points, rising in rpm.
    pub torque_curve: Vec<[f32; 2]>,
    /// Closed-throttle drag of the engine as a share of its torque.
    pub engine_braking: f32,
    /// Forward ratios from first gear.
    pub gear_ratios: Vec<f32>,
    pub reverse_ratio: f32,
    pub final_drive: f32,
    pub gear_efficiency: f32,
    pub drive: Drive,
    /// With [`Drive::All`], the share of the torque that goes to the front axle.
    pub front_torque_share: f32,
    /// Lock of the front and the rear differential, 0 (open) to 1.
    pub diff_lock: [f32; 2],
    /// Speed at full throttle in the top gear, m/s. The air drag is solved from it; without it `drag` is used.
    pub top_speed: Option<f32>,
    /// Air drag, N per (m/s)².
    pub drag: f32,
    /// Downforce, N per (m/s)².
    pub downforce: f32,
    /// Peak friction coefficient of the front and the rear tyre.
    pub tire_grip: [f32; 2],
    /// Brake torque at each wheel of an axle, and of the handbrake at the rear wheels, N m.
    pub brake_torque_nm: [f32; 2],
    pub handbrake_torque_nm: f32,
    pub steering_scale: f32,
    /// Spring natural frequency of an axle, Hz; damper ratio (1 is critical); wheel travel, m.
    pub ride_frequency_hz: [f32; 2],
    pub damping_ratio: f32,
    pub travel_m: f32,
    /// Extra torque of a forced-induction engine at the red line, as a fraction of the engine's torque. 0 for none.
    pub induction_boost: f32,
    pub nitrous: bool,
    /// Extra torque while nitrous burns, as a fraction of the engine's, and the burn time of a full tank, s.
    pub nitrous_boost: f32,
    pub nitrous_seconds: f32,
}

impl Default for CarStats {
    /// The engine's generic car: a 1450 kg rear-drive saloon of about 290 hp with six gears and no nitrous. The numbers
    /// are ordinary engineering values (the ones of [`VehicleSpec::example`]), not the data of any car.
    fn default() -> Self {
        Self {
            mass_kg: 1450.0,
            front_weight_pct: 52.0,
            cg_height_m: 0.5,
            wheelbase_m: 2.7,
            track_m: [1.55, 1.55],
            tire_radius_m: [0.33, 0.33],
            tire_width_m: [0.235, 0.255],
            ride_height_m: 0.12,
            idle_rpm: 900.0,
            redline_rpm: 7000.0,
            torque_curve: vec![
                [900.0, 258.0],
                [1800.0, 305.0],
                [2700.0, 339.0],
                [3600.0, 359.0],
                [4500.0, 359.0],
                [5400.0, 339.0],
                [6300.0, 305.0],
                [7000.0, 270.0],
            ],
            engine_braking: 0.15,
            gear_ratios: vec![3.6, 2.2, 1.5, 1.1, 0.9, 0.75],
            reverse_ratio: 3.4,
            final_drive: 3.7,
            gear_efficiency: 0.9,
            drive: Drive::Rear,
            front_torque_share: 0.4,
            diff_lock: [0.0, 0.3],
            top_speed: None,
            drag: 0.35,
            downforce: 0.2,
            tire_grip: [1.2, 1.2],
            brake_torque_nm: [1400.0, 950.0],
            handbrake_torque_nm: 1500.0,
            steering_scale: 1.0,
            ride_frequency_hz: [3.0, 3.1],
            damping_ratio: 0.7,
            travel_m: 0.18,
            induction_boost: 0.0,
            nitrous: false,
            nitrous_boost: 0.5,
            nitrous_seconds: 5.0,
        }
    }
}

impl CarStats {
    /// Engine torque in N m at `rpm`: linear between the curve's points, flat outside them.
    pub fn torque_at(&self, rpm: f32) -> f32 {
        let points = &self.torque_curve;
        let (Some(first), Some(last)) = (points.first(), points.last()) else { return 0.0 };
        if rpm <= first[0] {
            return first[1];
        }
        let Some(pair) = points.windows(2).find(|pair| rpm <= pair[1][0]) else { return last[1] };
        let t = (rpm - pair[0][0]) / (pair[1][0] - pair[0][0]).max(f32::EPSILON);
        pair[0][1] + (pair[1][1] - pair[0][1]) * t
    }

    /// The torque at the peak of the curve, N m.
    pub fn peak_torque_nm(&self) -> f32 {
        self.torque_curve.iter().map(|point| point[1]).fold(0.0, f32::max)
    }
}

impl VehicleSpec {
    /// The whole car described by `stats`. `dimension` is the half extents of the body box (x right, y up, z forward),
    /// which the caller measures from the car's own geometry.
    pub fn from_stats(stats: &CarStats, dimension: glam::Vec3) -> Self {
        build::spec(stats, dimension)
    }

    /// Set the ride height so that the car rests with the bottom of its body box `clearance` metres above the road, where
    /// the model puts it: the springs sag under the car's weight, so the wheels must reach that much further down than
    /// the clearance. Call it after [`Self::from_stats`] and after any upgrade, which changes the springs.
    pub fn rest_at(&mut self, clearance: f32) {
        build::rest_at(self, clearance);
    }
}

#[cfg(test)]
mod tests;
