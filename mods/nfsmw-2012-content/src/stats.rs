//! A 2012 car's own stats, as the `[stats]` table of its car manifest (docs/formats/chameleon-vehicle-stats.md,
//! docs/modding/car-stats.md).
//!
//! Every number comes from the car's own bundle: the mass of its rigid bodies, the places of its axles and wheels, the
//! torque curve of its engine, its gearbox, tyre grip, brake torque, top speed and nitrous strength. What the 2012 data
//! does not give (springs, dampers, steering, downforce, the handbrake) is left out of the table, so the game's generic
//! values apply; no number is taken from a 2005 car.

use chameleon_vehicle::VehicleStats;

use crate::drift_stats::DriftKeys;

/// Most points of the torque curve the manifest allows.
const MAX_POINTS: usize = 16;
/// Closest a sampled point may come to a knot of the curve, as a share of the curve's span.
const MIN_GAP: f32 = 0.02;
/// The idle speed of an engine whose curve starts at or near zero (an electric motor): the game's engine needs one.
const IDLE_FLOOR_RPM: f32 = 500.0;
/// Kilometres per hour per mile per hour.
const KPH_PER_MPH: f32 = 1.609_344;
/// A differential share at or below this is rear drive, at or above `1 - ` this front drive.
const DRIVE_EDGE: f32 = 0.01;

/// The ranges of the game's `[stats]` (libs/blackbox-modding/src/content/stats.rs); imported numbers are clamped to them.
const MASS: (f32, f32) = (200.0, 40_000.0);
const GRIP: (f32, f32) = (0.3, 3.0);
const BRAKE: (f32, f32) = (100.0, 20_000.0);
const TORQUE: (f32, f32) = (0.0, 6_000.0);
/// The range of a forward gear ratio, the reverse ratio and the final drive, and the most forward gears.
const GEAR_RATIO: (f32, f32) = (0.2, 10.0);
const RATIO: (f32, f32) = (0.5, 10.0);
const MAX_GEARS: usize = 10;

/// A car's stats in the manifest's units and keys.
#[derive(Debug, Clone, PartialEq)]
pub struct ImportedStats {
    pub mass_kg: f32,
    pub front_weight_pct: f32,
    pub cg_height_m: f32,
    pub wheelbase_m: f32,
    pub track_m: [f32; 2],
    pub tire_radius_m: [f32; 2],
    pub tire_width_m: [f32; 2],
    pub ride_height_m: f32,
    pub idle_rpm: f32,
    pub redline_rpm: f32,
    pub torque_curve: Vec<[f32; 2]>,
    pub gear_ratios: Vec<f32>,
    pub reverse_ratio: f32,
    pub final_drive: f32,
    /// `rwd`, `fwd` or `awd`.
    pub drive: &'static str,
    pub front_torque_share: f32,
    pub diff_lock: [f32; 2],
    pub top_speed_kph: f32,
    pub tire_grip: [f32; 2],
    pub brake_torque_nm: [f32; 2],
    /// Extra torque while nitrous burns; 0 when the car has none.
    pub nitrous_boost: f32,
    /// The car's drift numbers (`drift_*` keys), set by the importer from the car's handling; `None` writes no key.
    pub drift: Option<DriftKeys>,
}

impl ImportedStats {
    /// The stats of a car, read from its bundle.
    pub fn of(car: &VehicleStats) -> Self {
        let [front_axle, rear_axle] = car.axle_z;
        let wheels = &car.wheels;
        let total = car.body_mass_kg + wheels.iter().map(|wheel| wheel.mass_kg).sum::<f32>();
        // The wheels sit on their axles for the weight distribution: one car's wheel bodies are not where its axles are.
        let moment_z = car.body_mass_kg * car.centre_of_mass[2]
            + (wheels[0].mass_kg + wheels[1].mass_kg) * front_axle
            + (wheels[2].mass_kg + wheels[3].mass_kg) * rear_axle;
        let wheelbase = front_axle - rear_axle;
        let moment_y = car.body_mass_kg * car.centre_of_mass[1]
            + wheels.iter().map(|wheel| wheel.mass_kg * wheel.position[1]).sum::<f32>();
        let pair = |a: usize, b: usize, value: &dyn Fn(usize) -> f32| 0.5 * (value(a) + value(b));
        let track = |a: usize, b: usize| pair(a, b, &|n| wheels[n].position[0].abs()) * 2.0;
        let radius = |a: usize, b: usize| pair(a, b, &|n| wheels[n].radius);
        let width = |a: usize, b: usize| pair(a, b, &|n| wheels[n].half_width) * 2.0;
        let lowest = car.lowest_rpm();
        let redline = car.redline_rpm();
        let (drive, share) = drive_of(car.differentials[2]);
        Self {
            mass_kg: total.clamp(MASS.0, MASS.1),
            front_weight_pct: ((moment_z - total * rear_axle) / (total * wheelbase) * 100.0).clamp(20.0, 80.0),
            cg_height_m: (moment_y / total).clamp(0.1, 2.0),
            wheelbase_m: wheelbase.clamp(1.0, 6.0),
            track_m: [track(0, 1).clamp(0.8, 3.5), track(2, 3).clamp(0.8, 3.5)],
            tire_radius_m: [radius(0, 1).clamp(0.15, 0.8), radius(2, 3).clamp(0.15, 0.8)],
            tire_width_m: [width(0, 1).clamp(0.1, 0.5), width(2, 3).clamp(0.1, 0.5)],
            ride_height_m: (car.box_centre[1] - car.box_half_extents[1]).clamp(0.03, 0.5),
            idle_rpm: lowest.max(IDLE_FLOOR_RPM),
            redline_rpm: redline,
            torque_curve: sample_curve(car),
            gear_ratios: valid_gears(&car.gear_ratios),
            reverse_ratio: car.reverse_ratio.clamp(RATIO.0, RATIO.1),
            final_drive: car.final_drive.clamp(RATIO.0, RATIO.1),
            drive,
            front_torque_share: share,
            diff_lock: [car.differentials[1].clamp(0.0, 1.0), car.differentials[3].clamp(0.0, 1.0)],
            top_speed_kph: car.top_speed_mph * KPH_PER_MPH,
            tire_grip: car.tire_grip.map(|grip| grip.clamp(GRIP.0, GRIP.1)),
            brake_torque_nm: car.brake_torque_nm.map(|nm| nm.clamp(BRAKE.0, BRAKE.1)),
            nitrous_boost: (car.nitrous_strength - 1.0).clamp(0.0, 3.0),
            drift: None,
        }
    }

    /// The `[stats]` table of the car manifest.
    pub fn manifest_table(&self) -> String {
        let mut text = String::from("\n[stats]\n");
        let mut line = |key: &str, value: String| text.push_str(&format!("{key} = {value}\n"));
        line("mass_kg", number(self.mass_kg));
        line("front_weight_pct", number(self.front_weight_pct));
        line("cg_height_m", number(self.cg_height_m));
        line("wheelbase_m", number(self.wheelbase_m));
        line("front_track_m", number(self.track_m[0]));
        line("rear_track_m", number(self.track_m[1]));
        line("front_tire_radius_m", number(self.tire_radius_m[0]));
        line("rear_tire_radius_m", number(self.tire_radius_m[1]));
        line("front_tire_width_m", number(self.tire_width_m[0]));
        line("rear_tire_width_m", number(self.tire_width_m[1]));
        line("ride_height_m", number(self.ride_height_m));
        line("idle_rpm", number(self.idle_rpm));
        line("redline_rpm", number(self.redline_rpm));
        let points: Vec<String> =
            self.torque_curve.iter().map(|[rpm, nm]| format!("[{}, {}]", number(*rpm), number(*nm))).collect();
        line("torque_curve", format!("[{}]", points.join(", ")));
        let ratios: Vec<String> = self.gear_ratios.iter().map(|ratio| number(*ratio)).collect();
        line("gear_ratios", format!("[{}]", ratios.join(", ")));
        line("reverse_ratio", number(self.reverse_ratio));
        line("final_drive", number(self.final_drive));
        line("drive", format!("\"{}\"", self.drive));
        line("front_torque_share", number(self.front_torque_share));
        line("front_diff_lock", number(self.diff_lock[0]));
        line("rear_diff_lock", number(self.diff_lock[1]));
        line("top_speed_kph", number(self.top_speed_kph));
        line("front_tire_grip", number(self.tire_grip[0]));
        line("rear_tire_grip", number(self.tire_grip[1]));
        line("front_brake_torque_nm", number(self.brake_torque_nm[0]));
        line("rear_brake_torque_nm", number(self.brake_torque_nm[1]));
        let nitrous = self.nitrous_boost > 0.0;
        line("nitrous", nitrous.to_string());
        if nitrous {
            line("nitrous_boost", number(self.nitrous_boost));
        }
        if let Some(drift) = &self.drift {
            text.push_str(&drift.lines());
        }
        text
    }

    /// The torque of the curve at its highest point, N m.
    pub fn peak_torque_nm(&self) -> f32 {
        self.torque_curve.iter().map(|point| point[1]).fold(0.0, f32::max)
    }

    /// Scale every point of the curve, clamped to the game's range.
    pub fn scale_torque(&mut self, factor: f32) {
        for point in &mut self.torque_curve {
            point[1] = (point[1] * factor).clamp(TORQUE.0, TORQUE.1);
        }
    }

    pub fn scale_mass(&mut self, factor: f32) {
        self.mass_kg = (self.mass_kg * factor).clamp(MASS.0, MASS.1);
    }

    pub fn scale_grip(&mut self, factor: f32) {
        self.tire_grip = self.tire_grip.map(|grip| (grip * factor).clamp(GRIP.0, GRIP.1));
    }

    pub fn scale_brakes(&mut self, factor: f32) {
        self.brake_torque_nm = self.brake_torque_nm.map(|nm| (nm * factor).clamp(BRAKE.0, BRAKE.1));
    }
}

/// A number as the manifest writes it: four decimals at most, always with a decimal point.
pub(crate) fn number(value: f32) -> String {
    let rounded = (value * 10_000.0).round() / 10_000.0;
    format!("{rounded:?}")
}

/// The forward gears the game's gearbox accepts out of the bundle's: finite ratios inside the game's range, each strictly
/// lower than the gear before it (a gear that is not is left out), at most [`MAX_GEARS`]. The table is refused whole when its
/// gears do not fall, and a car with no gear in it would sit in neutral.
pub(crate) fn valid_gears(ratios: &[f32]) -> Vec<f32> {
    let mut out: Vec<f32> = Vec::with_capacity(ratios.len().min(MAX_GEARS));
    for &ratio in ratios {
        if out.len() == MAX_GEARS {
            break;
        }
        if !ratio.is_finite() || !(GEAR_RATIO.0..=GEAR_RATIO.1).contains(&ratio) {
            continue;
        }
        if out.last().is_some_and(|&last| ratio >= last) {
            continue;
        }
        out.push(ratio);
    }
    out
}

/// The drive of a car from the share of the torque its differential structure sends to the front axle.
fn drive_of(front_share: f32) -> (&'static str, f32) {
    if front_share <= DRIVE_EDGE {
        return ("rwd", 0.0);
    }
    if front_share >= 1.0 - DRIVE_EDGE {
        return ("fwd", 1.0);
    }
    ("awd", front_share)
}

/// Up to [`MAX_POINTS`] points of the torque curve: its knots, and evenly spaced points between them where they leave room.
fn sample_curve(car: &VehicleStats) -> Vec<[f32; 2]> {
    let (low, high) = (car.lowest_rpm(), car.redline_rpm());
    let span = (high - low).max(1.0);
    let mut rpms: Vec<f32> = car.torque_curve.iter().map(|knot| knot.rpm).collect();
    let spare = MAX_POINTS.saturating_sub(rpms.len());
    for step in 1..=spare {
        let rpm = low + span * step as f32 / (spare + 1) as f32;
        if rpms.iter().all(|known| (known - rpm).abs() > span * MIN_GAP) {
            rpms.push(rpm);
        }
    }
    rpms.sort_by(f32::total_cmp);
    rpms.into_iter().map(|rpm| [rpm.round(), (car.torque_at(rpm) * 10.0).round() / 10.0]).collect()
}

#[cfg(test)]
mod tests;
