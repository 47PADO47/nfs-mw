//! The `[stats]` table of a car manifest: what the car is, in absolute SI units.
//!
//! A car that a mod adds is its own car. Its mass, engine, gearbox, tyres, brakes and body come from this table and from
//! nothing else, so the table says what the car is and does not say how it differs from some other car. Every key is
//! optional; a key left out takes the engine's generic default (a 1450 kg rear-drive saloon, see [`Stats::default`]). A
//! number outside its range, or a key that is not in the table, refuses the car with the key named.

use serde::Deserialize;

use super::ContentError;

/// Which wheels the engine drives.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Drive {
    Rear,
    Front,
    All,
}

/// A car's stats. Axle pairs are `[front, rear]`.
#[derive(Debug, Clone, PartialEq)]
pub struct Stats {
    /// Mass of the whole car in kilograms.
    pub mass_kg: f32,
    /// Share of the weight on the front axle, in percent.
    pub front_weight_pct: f32,
    /// Height of the centre of gravity above the road, in metres.
    pub cg_height_m: f32,
    pub wheelbase_m: f32,
    /// Distance between the centres of the tyres of an axle, in metres.
    pub track_m: [f32; 2],
    /// Outer radius and width of the tyre of an axle, in metres.
    pub tire_radius_m: [f32; 2],
    pub tire_width_m: [f32; 2],
    /// Gap between the underside of the body and the road, in metres.
    pub ride_height_m: f32,
    /// Engine speed the engine idles at, and where its rev limiter holds, in rpm.
    pub idle_rpm: f32,
    pub redline_rpm: f32,
    /// Engine torque in newton metres at engine speeds in rpm, rising in rpm.
    pub torque_curve: Vec<[f32; 2]>,
    /// Closed-throttle drag of the engine as a share of its torque.
    pub engine_braking: f32,
    /// Forward gear ratios from first, the reverse ratio and the final drive.
    pub gear_ratios: Vec<f32>,
    pub reverse_ratio: f32,
    pub final_drive: f32,
    /// Share of the power lost in the gearbox, as an efficiency from 0.5 to 1.
    pub gear_efficiency: f32,
    pub drive: Drive,
    /// With [`Drive::All`], the share of the torque that goes to the front axle.
    pub front_torque_share: f32,
    /// How far the front and the rear differential lock, from 0 (open) to 1.
    pub diff_lock: [f32; 2],
    /// Speed the car reaches at full throttle in its top gear, in km/h. The game sets the car's air drag so that it
    /// does; without it the car has the drag coefficient below.
    pub top_speed_kph: Option<f32>,
    /// Air drag: newtons of drag per square metre per second of speed. Used when the car has no `top_speed_kph`.
    pub drag: f32,
    /// Downforce: newtons per square metre per second of speed, split between the axles.
    pub downforce: f32,
    /// Peak friction coefficient of the front and the rear tyre (1.0 is an ordinary road tyre).
    pub tire_grip: [f32; 2],
    /// Brake torque at each wheel of an axle, in newton metres, and of the handbrake at the rear wheels.
    pub brake_torque_nm: [f32; 2],
    pub handbrake_torque_nm: f32,
    /// Multiplier of the steering range of the front wheels.
    pub steering_scale: f32,
    /// Natural frequency of the springs of an axle in hertz, the damping ratio of the dampers (1 is critical) and the
    /// travel of the wheels in metres.
    pub ride_frequency_hz: [f32; 2],
    pub damping_ratio: f32,
    pub travel_m: f32,
    /// Extra torque of a turbocharger or supercharger at the red line, as a fraction of the engine's. 0 for none; the
    /// torque curve of a car that has one already includes it, so this is for a car whose curve is without it.
    pub induction_boost: f32,
    /// The car has nitrous from the factory, with this much extra torque (a fraction of the engine's) for this many
    /// seconds from a full tank.
    pub nitrous: bool,
    pub nitrous_boost: f32,
    pub nitrous_seconds: f32,
    /// How the car drifts while the drift assist is on (a mod turns it on, `bb.vehicles` 1.2). The assist is off in the
    /// game by default, so these keys change nothing until a mod asks for it.
    pub drift: DriftStats,
}

impl Default for Stats {
    /// The engine's generic car: a 1450 kg rear-drive saloon of about 290 hp with six gears and no nitrous. The numbers
    /// are ordinary engineering values, not the data of any car.
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
            top_speed_kph: None,
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
            drift: DriftStats::default(),
        }
    }
}

/// The drift assist's numbers of a car, in the units of the manifest.
pub use blackbox_vehicle::stats::DriftStats;

/// The table as serde reads it. Unknown keys are an error, like everywhere in a car manifest.
#[derive(Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub(super) struct Raw {
    mass_kg: Option<f32>,
    front_weight_pct: Option<f32>,
    cg_height_m: Option<f32>,
    wheelbase_m: Option<f32>,
    front_track_m: Option<f32>,
    rear_track_m: Option<f32>,
    front_tire_radius_m: Option<f32>,
    rear_tire_radius_m: Option<f32>,
    front_tire_width_m: Option<f32>,
    rear_tire_width_m: Option<f32>,
    ride_height_m: Option<f32>,
    idle_rpm: Option<f32>,
    redline_rpm: Option<f32>,
    torque_curve: Option<Vec<[f32; 2]>>,
    engine_braking: Option<f32>,
    gear_ratios: Option<Vec<f32>>,
    reverse_ratio: Option<f32>,
    final_drive: Option<f32>,
    gear_efficiency: Option<f32>,
    drive: Option<RawDrive>,
    front_torque_share: Option<f32>,
    front_diff_lock: Option<f32>,
    rear_diff_lock: Option<f32>,
    top_speed_kph: Option<f32>,
    drag: Option<f32>,
    downforce: Option<f32>,
    front_tire_grip: Option<f32>,
    rear_tire_grip: Option<f32>,
    front_brake_torque_nm: Option<f32>,
    rear_brake_torque_nm: Option<f32>,
    handbrake_torque_nm: Option<f32>,
    steering_scale: Option<f32>,
    front_ride_frequency_hz: Option<f32>,
    rear_ride_frequency_hz: Option<f32>,
    damping_ratio: Option<f32>,
    travel_m: Option<f32>,
    induction_boost: Option<f32>,
    nitrous: Option<bool>,
    nitrous_boost: Option<f32>,
    nitrous_seconds: Option<f32>,
    drift_enter_speed_ms: Option<f32>,
    drift_enter_angle_deg: Option<f32>,
    drift_exit_angle_deg: Option<f32>,
    drift_max_angle_deg: Option<f32>,
    drift_yaw_torque_nm: Option<f32>,
}

#[derive(Deserialize, Clone, Copy)]
#[serde(rename_all = "lowercase")]
enum RawDrive {
    Rwd,
    Fwd,
    Awd,
}

const REASON: &str = "must be a number inside the range the docs give";

/// `value`, or `default` when the key was left out; an error naming `field` when it is outside `low..=high`.
fn ranged(value: Option<f32>, default: f32, field: &'static str, low: f32, high: f32) -> Result<f32, ContentError> {
    let Some(value) = value else { return Ok(default) };
    if !(low..=high).contains(&value) {
        return Err(ContentError::Field { field, reason: REASON });
    }
    Ok(value)
}

/// Check the table and fill the keys that were left out. The checks that compare keys (idle below the redline, a
/// torque curve that rises in rpm) come after the checks of the single keys.
pub(super) fn check(raw: Option<Raw>) -> Result<Stats, ContentError> {
    let Some(raw) = raw else { return Ok(Stats::default()) };
    let d = Stats::default();
    let r = ranged;
    let drift = drift(&raw, d.drift)?;
    let stats = Stats {
        mass_kg: r(raw.mass_kg, d.mass_kg, "stats.mass_kg", 200.0, 40_000.0)?,
        front_weight_pct: r(raw.front_weight_pct, d.front_weight_pct, "stats.front_weight_pct", 20.0, 80.0)?,
        cg_height_m: r(raw.cg_height_m, d.cg_height_m, "stats.cg_height_m", 0.1, 2.0)?,
        wheelbase_m: r(raw.wheelbase_m, d.wheelbase_m, "stats.wheelbase_m", 1.0, 6.0)?,
        track_m: [
            r(raw.front_track_m, d.track_m[0], "stats.front_track_m", 0.8, 3.5)?,
            r(raw.rear_track_m, d.track_m[1], "stats.rear_track_m", 0.8, 3.5)?,
        ],
        tire_radius_m: [
            r(raw.front_tire_radius_m, d.tire_radius_m[0], "stats.front_tire_radius_m", 0.15, 0.8)?,
            r(raw.rear_tire_radius_m, d.tire_radius_m[1], "stats.rear_tire_radius_m", 0.15, 0.8)?,
        ],
        tire_width_m: [
            r(raw.front_tire_width_m, d.tire_width_m[0], "stats.front_tire_width_m", 0.1, 0.5)?,
            r(raw.rear_tire_width_m, d.tire_width_m[1], "stats.rear_tire_width_m", 0.1, 0.5)?,
        ],
        ride_height_m: r(raw.ride_height_m, d.ride_height_m, "stats.ride_height_m", 0.03, 0.5)?,
        idle_rpm: r(raw.idle_rpm, d.idle_rpm, "stats.idle_rpm", 300.0, 3_000.0)?,
        redline_rpm: r(raw.redline_rpm, d.redline_rpm, "stats.redline_rpm", 1_500.0, 20_000.0)?,
        torque_curve: curve(raw.torque_curve, d.torque_curve)?,
        engine_braking: r(raw.engine_braking, d.engine_braking, "stats.engine_braking", 0.0, 1.0)?,
        gear_ratios: gears(raw.gear_ratios, d.gear_ratios)?,
        reverse_ratio: r(raw.reverse_ratio, d.reverse_ratio, "stats.reverse_ratio", 0.5, 10.0)?,
        final_drive: r(raw.final_drive, d.final_drive, "stats.final_drive", 0.5, 10.0)?,
        gear_efficiency: r(raw.gear_efficiency, d.gear_efficiency, "stats.gear_efficiency", 0.5, 1.0)?,
        drive: raw.drive.map_or(d.drive, |drive| match drive {
            RawDrive::Rwd => Drive::Rear,
            RawDrive::Fwd => Drive::Front,
            RawDrive::Awd => Drive::All,
        }),
        front_torque_share: r(raw.front_torque_share, d.front_torque_share, "stats.front_torque_share", 0.0, 1.0)?,
        diff_lock: [
            r(raw.front_diff_lock, d.diff_lock[0], "stats.front_diff_lock", 0.0, 1.0)?,
            r(raw.rear_diff_lock, d.diff_lock[1], "stats.rear_diff_lock", 0.0, 1.0)?,
        ],
        top_speed_kph: raw.top_speed_kph.map(|v| r(Some(v), v, "stats.top_speed_kph", 50.0, 700.0)).transpose()?,
        drag: r(raw.drag, d.drag, "stats.drag", 0.02, 3.0)?,
        downforce: r(raw.downforce, d.downforce, "stats.downforce", 0.0, 0.5)?,
        tire_grip: [
            r(raw.front_tire_grip, d.tire_grip[0], "stats.front_tire_grip", 0.3, 3.0)?,
            r(raw.rear_tire_grip, d.tire_grip[1], "stats.rear_tire_grip", 0.3, 3.0)?,
        ],
        brake_torque_nm: [
            r(raw.front_brake_torque_nm, d.brake_torque_nm[0], "stats.front_brake_torque_nm", 100.0, 20_000.0)?,
            r(raw.rear_brake_torque_nm, d.brake_torque_nm[1], "stats.rear_brake_torque_nm", 100.0, 20_000.0)?,
        ],
        handbrake_torque_nm: r(
            raw.handbrake_torque_nm,
            d.handbrake_torque_nm,
            "stats.handbrake_torque_nm",
            0.0,
            20_000.0,
        )?,
        steering_scale: r(raw.steering_scale, d.steering_scale, "stats.steering_scale", 0.5, 2.0)?,
        ride_frequency_hz: [
            r(raw.front_ride_frequency_hz, d.ride_frequency_hz[0], "stats.front_ride_frequency_hz", 0.5, 6.0)?,
            r(raw.rear_ride_frequency_hz, d.ride_frequency_hz[1], "stats.rear_ride_frequency_hz", 0.5, 6.0)?,
        ],
        damping_ratio: r(raw.damping_ratio, d.damping_ratio, "stats.damping_ratio", 0.1, 1.0)?,
        travel_m: r(raw.travel_m, d.travel_m, "stats.travel_m", 0.03, 0.4)?,
        induction_boost: r(raw.induction_boost, d.induction_boost, "stats.induction_boost", 0.0, 1.5)?,
        nitrous: raw.nitrous.unwrap_or(d.nitrous),
        nitrous_boost: r(raw.nitrous_boost, d.nitrous_boost, "stats.nitrous_boost", 0.0, 3.0)?,
        nitrous_seconds: r(raw.nitrous_seconds, d.nitrous_seconds, "stats.nitrous_seconds", 0.5, 30.0)?,
        drift,
    };
    relations(&stats)?;
    Ok(stats)
}

/// The drift keys: a speed, three angles and a torque, each inside its range, the exit angle under the enter angle.
fn drift(raw: &Raw, d: DriftStats) -> Result<DriftStats, ContentError> {
    let r = ranged;
    let drift = DriftStats {
        enter_speed: r(raw.drift_enter_speed_ms, d.enter_speed, "stats.drift_enter_speed_ms", 2.0, 60.0)?,
        enter_angle_deg: r(raw.drift_enter_angle_deg, d.enter_angle_deg, "stats.drift_enter_angle_deg", 2.0, 30.0)?,
        exit_angle_deg: r(raw.drift_exit_angle_deg, d.exit_angle_deg, "stats.drift_exit_angle_deg", 1.0, 20.0)?,
        max_angle_deg: r(raw.drift_max_angle_deg, d.max_angle_deg, "stats.drift_max_angle_deg", 10.0, 70.0)?,
        yaw_torque_nm: r(raw.drift_yaw_torque_nm, d.yaw_torque_nm, "stats.drift_yaw_torque_nm", 0.0, 200_000.0)?,
    };
    if drift.exit_angle_deg >= drift.enter_angle_deg {
        return Err(ContentError::Field {
            field: "stats.drift_exit_angle_deg",
            reason: "the exit angle is under the enter angle",
        });
    }
    Ok(drift)
}

/// The torque curve: 2 to 16 points, rpm rising, torque from 0 to 6000 N m, some torque above 0.
fn curve(points: Option<Vec<[f32; 2]>>, default: Vec<[f32; 2]>) -> Result<Vec<[f32; 2]>, ContentError> {
    let Some(points) = points else { return Ok(default) };
    let bad = |reason| ContentError::Field { field: "stats.torque_curve", reason };
    if points.len() < 2 || points.len() > 16 {
        return Err(bad("a torque curve has 2 to 16 points"));
    }
    let in_range = |[rpm, torque]: &[f32; 2]| (0.0..=25_000.0).contains(rpm) && (0.0..=6_000.0).contains(torque);
    if !points.iter().all(in_range) {
        return Err(bad("each point is [rpm from 0 to 25000, torque from 0 to 6000 N m]"));
    }
    if points.windows(2).any(|pair| pair[1][0] <= pair[0][0]) {
        return Err(bad("the rpm of the points rises"));
    }
    if points.iter().all(|point| point[1] <= 0.0) {
        return Err(bad("the engine makes torque somewhere"));
    }
    Ok(points)
}

/// The forward gear ratios: 1 to 10 numbers from 0.2 to 10, each below the one before it.
fn gears(ratios: Option<Vec<f32>>, default: Vec<f32>) -> Result<Vec<f32>, ContentError> {
    let Some(ratios) = ratios else { return Ok(default) };
    let bad = |reason| ContentError::Field { field: "stats.gear_ratios", reason };
    if ratios.is_empty() || ratios.len() > 10 {
        return Err(bad("a car has 1 to 10 forward gears"));
    }
    if !ratios.iter().all(|ratio| (0.2..=10.0).contains(ratio)) {
        return Err(bad("each ratio is a number from 0.2 to 10"));
    }
    if ratios.windows(2).any(|pair| pair[1] >= pair[0]) {
        return Err(bad("each gear is lower than the gear before it"));
    }
    Ok(ratios)
}

/// Checks that compare keys.
fn relations(stats: &Stats) -> Result<(), ContentError> {
    let field = |field: &'static str, reason: &'static str| ContentError::Field { field, reason };
    if stats.redline_rpm <= stats.idle_rpm {
        return Err(field("stats.redline_rpm", "the redline is above the idle"));
    }
    let top = stats.torque_curve.last().map_or(0.0, |point| point[0]);
    if stats.idle_rpm >= top {
        return Err(field("stats.idle_rpm", "the torque curve reaches above the idle"));
    }
    Ok(())
}

/// The stats as the vehicle library builds a car from them: the same numbers, with the top speed in m/s.
impl From<&Stats> for blackbox_vehicle::stats::CarStats {
    fn from(stats: &Stats) -> Self {
        use blackbox_vehicle::stats::Drive as Wheels;
        Self {
            mass_kg: stats.mass_kg,
            front_weight_pct: stats.front_weight_pct,
            cg_height_m: stats.cg_height_m,
            wheelbase_m: stats.wheelbase_m,
            track_m: stats.track_m,
            tire_radius_m: stats.tire_radius_m,
            tire_width_m: stats.tire_width_m,
            ride_height_m: stats.ride_height_m,
            idle_rpm: stats.idle_rpm,
            redline_rpm: stats.redline_rpm,
            torque_curve: stats.torque_curve.clone(),
            engine_braking: stats.engine_braking,
            gear_ratios: stats.gear_ratios.clone(),
            reverse_ratio: stats.reverse_ratio,
            final_drive: stats.final_drive,
            gear_efficiency: stats.gear_efficiency,
            drive: match stats.drive {
                Drive::Rear => Wheels::Rear,
                Drive::Front => Wheels::Front,
                Drive::All => Wheels::All,
            },
            front_torque_share: stats.front_torque_share,
            diff_lock: stats.diff_lock,
            top_speed: stats.top_speed_kph.map(|kph| kph / 3.6),
            drag: stats.drag,
            downforce: stats.downforce,
            tire_grip: stats.tire_grip,
            brake_torque_nm: stats.brake_torque_nm,
            handbrake_torque_nm: stats.handbrake_torque_nm,
            steering_scale: stats.steering_scale,
            ride_frequency_hz: stats.ride_frequency_hz,
            damping_ratio: stats.damping_ratio,
            travel_m: stats.travel_m,
            induction_boost: stats.induction_boost,
            nitrous: stats.nitrous,
            nitrous_boost: stats.nitrous_boost,
            nitrous_seconds: stats.nitrous_seconds,
            drift: stats.drift,
        }
    }
}
