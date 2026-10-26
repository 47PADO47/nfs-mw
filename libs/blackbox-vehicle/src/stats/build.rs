//! [`CarStats`] to [`VehicleSpec`]: every parameter the physics reads, from the car's own stats or a documented generic.
//!
//! What the stats give: mass, weight distribution, centre-of-gravity height, wheelbase, track, tyre size, engine torque,
//! gearbox, tyre grip, brake torque, drag (or a top speed to solve it from), steering, suspension frequency, induction
//! and nitrous. What the physics needs that the stats do not give, and what it gets:
//!
//! | Parameter | Value |
//! |---|---|
//! | Rigid body (gravity, sleep, ground contact, object contact) | the one every stock racing car has ([`racer_body`]) |
//! | Inertia tensor scale | `(1, 2, 1)`: twice the inertia about the vertical axis, as the stock racing cars |
//! | Flywheel mass, rev-limiter ceiling | 20, the last torque point or the red line |
//! | Gear efficiency per gear | the stats' single efficiency |
//! | Clutch slip, shift speed, torque converter, perfect-shift window | 0.3, 0.1, 0, 0 |
//! | Brake lock share | 0.9 of the brake torque |
//! | Yaw control against speed, yaw speed | `[1.0, 0.8, 0.6]`, 1.0 |
//! | Damper valving, digression, blow-off, spring progression | 0, 0, 0, 2.0 |
//! | Damper extension over compression, anti-roll bar over spring | 1.33, 0.15 |
//! | Centre differential of an all-wheel drive | 0.5 |
//! | Downforce centre | halfway along the wheelbase |
//!
//! These are the values of [`VehicleSpec::example`], whose numbers are ordinary engineering values.

use glam::Vec3;

use super::{CarStats, Drive};
use crate::aero::AeroSpec;
use crate::brakes::{BRAKING_TORQUE_SCALE, BrakeSpec, EBRAKING_TORQUE_SCALE};
use crate::drivetrain::TransmissionSpec;
use crate::engine::EngineSpec;
use crate::induction::InductionSpec;
use crate::math::{FT_LB_TO_NM, LB_IN_TO_N_M, RPM_PER_RAD_S, inch_to_m};
use crate::nos::NosSpec;
use crate::rigid_body::{ObjectMaterial, RigidBodySpec};
use crate::suspension::ChassisSpec;
use crate::tires::TireSpec;
use crate::vehicle::VehicleSpec;

/// Torque samples the engine table gets, evenly spaced from idle to its last rpm.
pub const SAMPLES: usize = 16;
/// Friction coefficient of the engine's reference tyre: a high-performance road tyre. The engine's tyre model is
/// calibrated so that this tyre has the grip values below (the capture in `docs/specs/vehicle-calibration.md`).
const REFERENCE_GRIP: f32 = 1.65;
/// `STATIC_GRIP` and `DYNAMIC_GRIP` of the reference tyre.
const REFERENCE_STATIC: f32 = 2.075;
const REFERENCE_DYNAMIC: f32 = 1.85;
/// Share of the tyre width that the sidewall is high, when the rim is chosen to fit the radius.
const SIDEWALL_OF_WIDTH: f32 = 0.45;
/// Smallest rim, m.
const MIN_RIM: f32 = 0.25;
/// Damper extension over compression and anti-roll bar over spring rate, as in [`VehicleSpec::example`].
const EXTENSION_OVER_COMPRESSION: f32 = 1.33;
const SWAYBAR_OVER_SPRING: f32 = 0.5;
/// Spring progression, damper valving speed, valving digression and blow-off of the stock racing cars (400 to 475 lb/in
/// springs: 5 to 7.5, 17 to 24, 0.2 to 0.35, 5 to 6), and the height of their centre of gravity over the box height.
const SPRING_PROGRESSION: f32 = 6.0;
const SHOCK_VALVING: f32 = 20.0;
const SHOCK_DIGRESSION: f32 = 0.3;
const SHOCK_BLOWOUT: f32 = 5.0;
const CG_OF_BOX_HEIGHT: f32 = 0.2;
const MIN_CG: f32 = 0.18;
const MAX_CG: f32 = 0.32;
/// The tyre model's yaw assist against speed and its rate, as the stock M3 has them: weak at low speed, full at speed.
const YAW_CONTROL: [f32; 4] = [0.1, 0.4, 0.6, 1.0];
const YAW_SPEED: f32 = 0.3;
/// The least rear grip over the front grip.
const REAR_OVER_FRONT_GRIP: f32 = 1.1;
/// The share of the drag and torque limit a solved drag keeps under the rev limiter (see [`solve_drag`]).
const LIMITER_MARGIN: f32 = 0.985;

pub(super) fn spec(stats: &CarStats, dimension: Vec3) -> VehicleSpec {
    let mut spec = VehicleSpec {
        mass: stats.mass_kg,
        dimension,
        tensor_scale: TENSOR_SCALE,
        body: racer_body(),
        chassis: chassis(stats, dimension),
        tires: tires(stats),
        brakes: brakes(stats),
        engine: engine(stats),
        transmission: transmission(stats),
        induction: induction(stats),
        nos: nos(stats),
        aero: AeroSpec {
            drag_coefficient: stats.drag,
            aero_coefficient: stats.downforce,
            aero_cg: aero_cg(stats),
            ..AeroSpec::default()
        },
    };
    if let Some(top_speed) = stats.top_speed {
        spec.aero.drag_coefficient = solve_drag(&spec, top_speed);
    }
    spec
}

fn chassis(stats: &CarStats, dimension: Vec3) -> ChassisSpec {
    let front_share = stats.front_weight_pct * 0.01;
    // Mass on one wheel of each axle.
    let corner = [stats.mass_kg * front_share * 0.5, stats.mass_kg * (1.0 - front_share) * 0.5];
    // Spring rate per wheel from the natural frequency, k = m (2 pi f)^2, in lb/in.
    let spring: [f32; 2] = std::array::from_fn(|axle| {
        let omega = std::f32::consts::TAU * stats.ride_frequency_hz[axle];
        corner[axle] * omega * omega / LB_IN_TO_N_M
    });
    // Damper rate from the damping ratio, c = 2 zeta sqrt(k m), in lb s/in.
    let damper: [f32; 2] = std::array::from_fn(|axle| {
        let k = spring[axle] * LB_IN_TO_N_M;
        2.0 * stats.damping_ratio * (k * corner[axle]).sqrt() / LB_IN_TO_N_M
    });
    let travel = stats.travel_m / inch_to_m(1.0);
    ChassisSpec {
        spring_stiffness: spring,
        spring_progression: [SPRING_PROGRESSION; 2],
        shock_stiffness: damper,
        shock_ext_stiffness: damper.map(|rate| rate * EXTENSION_OVER_COMPRESSION),
        shock_valving: [SHOCK_VALVING; 2],
        shock_digression: [SHOCK_DIGRESSION; 2],
        shock_blowout: SHOCK_BLOWOUT,
        swaybar_stiffness: spring.map(|rate| rate * SWAYBAR_OVER_SPRING),
        travel: [travel; 2],
        ride_height: [stats.ride_height_m / inch_to_m(1.0); 2],
        // The spec's track width is between the outer tyre edges.
        track_width: [stats.track_m[0] + stats.tire_width_m[0], stats.track_m[1] + stats.tire_width_m[1]],
        wheel_base: stats.wheelbase_m,
        // The axles are centred on the body box.
        front_axle: stats.wheelbase_m * 0.5,
        front_weight_bias: stats.front_weight_pct,
        roll_center: roll_center(dimension) / inch_to_m(1.0),
    }
}

/// How far a spring sags under its share of the weight, over the closed form `m g / k`: the progression stiffens it as it
/// compresses.
const SAG_FACTOR: f32 = 0.9;

/// See [`VehicleSpec::rest_at`].
pub(super) fn rest_at(spec: &mut VehicleSpec, clearance: f32) {
    let front_share = spec.chassis.front_weight_bias * 0.01;
    let shares = [front_share, 1.0 - front_share];
    for axle in 0..2 {
        let corner = spec.mass * shares[axle] * 0.5;
        let stiffness = (spec.chassis.spring_stiffness[axle] * LB_IN_TO_N_M).max(1.0);
        let sag = SAG_FACTOR * corner * 9.81 / stiffness;
        spec.chassis.ride_height[axle] = (clearance + sag) / inch_to_m(1.0);
    }
}

/// The engine's own rigid body of a racing car: how it sits on the ground and how it bounces off objects. Every stock
/// racing car has these numbers; they are not any one car's.
fn racer_body() -> RigidBodySpec {
    RigidBodySpec {
        gravity: -9.8128,
        sleep_velocity: 1.0,
        ground_friction: [0.55, 0.45],
        ground_elasticity: Vec3::ZERO,
        ground_moment_scale: Vec3::new(1.5, 3.0, 1.5),
        objects: ObjectMaterial {
            immobile: false,
            friction: [0.55, 0.35],
            elasticity: Vec3::new(0.0, 0.0, 0.05),
            inertia_scale: Vec3::new(4.0, 15.0, 3.0),
            mass_scale: 1.0,
        },
        ..RigidBodySpec::default()
    }
}

/// Twice the inertia about the vertical axis, as the stock racing cars have it: a car that yaws as easily as a real one
/// is too twitchy for the engine's steering.
const TENSOR_SCALE: Vec3 = Vec3::new(1.0, 2.0, 1.0);
/// Where the downforce acts along the wheelbase, in percent. The stock racing cars have it 4 to 7 points behind their
/// weight bias (47 to 50 for a 53 to 54 percent front bias); a car whose downforce acts further forward than its weight
/// does lifts its rear and spins at speed.
fn aero_cg(stats: &CarStats) -> f32 {
    (stats.front_weight_pct - AERO_BEHIND_WEIGHT).clamp(25.0, 55.0)
}

const AERO_BEHIND_WEIGHT: f32 = 6.0;

/// Height of the roll centre, which is also the height of the centre of gravity, above the road. The engine's handling is
/// tuned for the low centres of gravity of its cars (0.18 to 0.22 of the height of the body box in the stock cars), not
/// for the real ones (0.35 to 0.5 m), which a car of this engine rolls over and slides on. The box is the car's own.
fn roll_center(dimension: Vec3) -> f32 {
    (CG_OF_BOX_HEIGHT * 2.0 * dimension.y).clamp(MIN_CG, MAX_CG)
}

/// Rim, section width and aspect ratio of the tyre of an axle, whose outer radius and width are in the stats. The rim
/// takes what the sidewall leaves; the aspect ratio is then exact, so the radius is the stats' radius.
fn tire_size(radius: f32, width: f32) -> (f32, f32, f32) {
    let diameter = 2.0 * radius;
    let mut rim = diameter - 2.0 * SIDEWALL_OF_WIDTH * width;
    rim = rim.max(MIN_RIM);
    rim = rim.min(diameter - 0.02);
    let aspect = (diameter - rim) / (2.0 * width) * 100.0;
    (rim / inch_to_m(1.0), width * 1000.0, aspect)
}

fn tires(stats: &CarStats) -> TireSpec {
    let size: [_; 2] = std::array::from_fn(|axle| tire_size(stats.tire_radius_m[axle], stats.tire_width_m[axle]));
    let mut level = stats.tire_grip.map(|grip| grip / REFERENCE_GRIP);
    // The engine's stability assists and its tyre model are tuned for cars whose rear tyres hold at least as well as the
    // front ones (all the stock cars: rear static grip is 1.0 to 1.1 of the front's). A rear that holds less, as on a
    // rear-engined car, spins at once at the speed and steering angle a stock car corners at.
    level[1] = level[1].max(level[0] * REAR_OVER_FRONT_GRIP);
    TireSpec {
        rim_size: size.map(|(rim, ..)| rim),
        section_width: size.map(|(_, width, _)| width),
        aspect_ratio: size.map(|(.., aspect)| aspect),
        grip_scale: level,
        static_grip: level.map(|level| level * REFERENCE_STATIC),
        dynamic_grip: level.map(|level| level * REFERENCE_DYNAMIC),
        steering: stats.steering_scale,
        yaw_control: YAW_CONTROL.to_vec(),
        yaw_speed: YAW_SPEED,
    }
}

fn brakes(stats: &CarStats) -> BrakeSpec {
    BrakeSpec {
        brakes: stats.brake_torque_nm.map(|nm| nm / (FT_LB_TO_NM * BRAKING_TORQUE_SCALE)),
        brake_lock: [0.9; 2],
        ebrake: stats.handbrake_torque_nm / (FT_LB_TO_NM * EBRAKING_TORQUE_SCALE),
    }
}

fn engine(stats: &CarStats) -> EngineSpec {
    let last = stats.torque_curve.last().map_or(stats.redline_rpm, |point| point[0]);
    let max_rpm = last.max(stats.redline_rpm);
    let step = (max_rpm - stats.idle_rpm) / (SAMPLES - 1) as f32;
    EngineSpec {
        torque: (0..SAMPLES).map(|n| stats.torque_at(stats.idle_rpm + step * n as f32) / FT_LB_TO_NM).collect(),
        idle: stats.idle_rpm,
        red_line: stats.redline_rpm,
        max_rpm,
        flywheel_mass: 20.0,
        engine_braking: vec![stats.engine_braking],
        speed_limiter: [0.0; 2],
        no_rev_limiter: false,
    }
}

fn transmission(stats: &CarStats) -> TransmissionSpec {
    // Gear ids: reverse, neutral, then the forward gears.
    let ratios: Vec<f32> = [stats.reverse_ratio, 0.0].into_iter().chain(stats.gear_ratios.iter().copied()).collect();
    let (torque_split, differential) = match stats.drive {
        Drive::Rear => (0.0, [0.0, stats.diff_lock[1], 0.0]),
        Drive::Front => (1.0, [stats.diff_lock[0], 0.0, 0.0]),
        Drive::All => (stats.front_torque_share, [stats.diff_lock[0], stats.diff_lock[1], 0.5]),
    };
    TransmissionSpec {
        gear_efficiency: vec![stats.gear_efficiency; ratios.len()],
        gear_ratio: ratios,
        final_gear: stats.final_drive,
        torque_split,
        differential,
        torque_converter: 0.0,
        clutch_slip: 0.3,
        shift_speed: 0.1,
        optimal_shift: 0.0,
    }
}

/// A turbocharger of the strength the stats give: no boost at the idle, `induction_boost` at the red line, spooling over
/// about a second. These shapes are generic; the stats give only the strength.
fn induction(stats: &CarStats) -> InductionSpec {
    if stats.induction_boost <= 0.0 {
        return InductionSpec::default();
    }
    InductionSpec {
        low_boost: 0.0,
        high_boost: stats.induction_boost,
        spool: 0.35,
        spool_time_up: 1.2,
        spool_time_down: 0.8,
        vacuum: -0.03,
        psi: stats.induction_boost * 30.0,
    }
}

/// Nitrous of the stats' strength and tank. The recharge shape is generic: a full tank takes 20 s at 20 mph and 5 s at
/// 100 mph, and recharging starts half a second after the button is released.
fn nos(stats: &CarStats) -> NosSpec {
    if !stats.nitrous {
        return NosSpec::default();
    }
    NosSpec {
        nos_capacity: stats.nitrous_seconds,
        torque_boost: stats.nitrous_boost,
        nos_disengage: 0.5,
        recharge_min: 20.0,
        recharge_max: 5.0,
        recharge_min_speed: 20.0,
        recharge_max_speed: 100.0,
    }
}

/// The air drag that makes the car's top speed `top_speed` (m/s) in its top gear at full throttle.
///
/// The engine speed at a road speed follows the drivetrain's own map, `rpm = idle + (v / r) ratio (1 - idle / red_line)`
/// in rad/s, and the wheel force is torque x ratio x efficiency / r. Drag force is `c v²`, so `c` is the wheel force at
/// that speed over `v²`. A top speed the car cannot reach before the rev limiter is replaced by the speed at the limiter
/// (a little under it), so the drag never goes to zero and the limiter ends the run, as it would.
pub fn solve_drag(spec: &VehicleSpec, top_speed: f32) -> f32 {
    let (engine, trans) = (&spec.engine, &spec.transmission);
    let radius = spec.tires.radius(1);
    let top_ratio = trans.gear_ratio.last().copied().unwrap_or(1.0).abs() * trans.final_gear;
    if top_ratio <= f32::EPSILON || top_speed <= 0.0 || engine.red_line <= engine.idle {
        return 0.0;
    }
    let share = 1.0 - engine.idle / engine.red_line;
    let idle_rad = engine.idle / RPM_PER_RAD_S;
    // Road speed at an engine speed in rpm, and the other way round.
    let speed_at = |rpm: f32| (rpm / RPM_PER_RAD_S - idle_rad) / (top_ratio * share) * radius;
    let limiter_speed = speed_at(engine.red_line) * LIMITER_MARGIN;
    let speed = top_speed.min(limiter_speed).max(1.0);
    let rpm = ((speed / radius) * top_ratio * share + idle_rad) * RPM_PER_RAD_S;
    let efficiency = trans.efficiency(trans.gear_ratio.len().saturating_sub(1));
    let boost = 1.0 + spec.induction.full_boost(rpm, engine.idle, engine.red_line);
    let force = engine.torque_nm(rpm) * boost * top_ratio * efficiency / radius;
    force / (speed * speed)
}
