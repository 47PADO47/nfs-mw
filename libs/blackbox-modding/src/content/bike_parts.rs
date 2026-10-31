//! The parts of the `[bike]` table under the wheels and in the rider's hands: suspension, engine, brakes, steering and
//! the lean controller, the fall and the wheelie assist (the gearbox is in `bike_transmission.rs`). Same units as `blackbox_vehicle::bike::BikeSpec` (see `bike.rs`).

use blackbox_vehicle::bike::{
    AbsSpec, BrakeSpec, Coupling as CouplingSpec, DamperSpec, EngineBrakeReduction, EngineSpec, FallExtra, FallSpec,
    Hydraulic as HydraulicSpec, Progressive as ProgressiveSpec, RangeMap, SteeringSpec, SuspensionSpec,
};
use serde::Deserialize;

pub use super::bike_lean::Lean;
pub use super::bike_transmission::Transmission;
pub use super::bike_wheelie::{Pitch, Wheelie};

use super::bike_lean::one;

pub(super) fn pairs(points: &[[f32; 2]]) -> Vec<(f32, f32)> {
    points.iter().map(|&[x, y]| (x, y)).collect()
}

/// Two-slope damper (`[bike.front_suspension.damper]`): the slow slope in N s/m up to a threshold speed in m/s, the fast
/// slope beyond it, separately for bump (compressing) and rebound.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Damper {
    pub slow_bump: f32,
    pub fast_bump: f32,
    pub bump_threshold: f32,
    pub slow_rebound: f32,
    pub fast_rebound: f32,
    pub rebound_threshold: f32,
}

/// A progressive spring term: where it starts (share of the travel), its factor (0: off) and its exponent.
#[derive(Debug, Clone, Copy, Default, PartialEq, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct Progressive {
    pub start: f32,
    pub factor: f32,
    pub linearity: f32,
}

/// The second, progressive spring-like term: its rate in N/m (0: off) and its progressive part.
#[derive(Debug, Clone, Copy, Default, PartialEq, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct Hydraulic {
    pub stiffness_n_per_m: f32,
    pub start: f32,
    pub factor: f32,
    pub linearity: f32,
}

/// The chain pull, traction and brake coupling into the suspension: a factor (0: off) and the remap, from the value at no
/// compression to the value at full travel.
#[derive(Debug, Clone, Copy, Default, PartialEq, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct Coupling {
    pub chain: f32,
    pub chain_range: [f32; 2],
    pub traction: f32,
    pub traction_range: [f32; 2],
    pub brake: f32,
    pub brake_range: [f32; 2],
}

/// A fork or a shock (`[bike.front_suspension]`, `[bike.rear_suspension]`).
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Suspension {
    /// The length the suspension compresses over from its fully extended state, at the wheel, m.
    pub max_travel_m: f32,
    /// Spring rate at the shock, N/m.
    pub spring_n_per_m: f32,
    /// Stiffness of the spring that opposes full extension, N/m, over the first `top_out_length_m` of the travel.
    pub top_out_stiffness_n_per_m: f32,
    #[serde(default)]
    pub top_out_length_m: f32,
    pub damper: Damper,
    /// `[share of travel, ratio]` of wheel travel to shock travel, share rising from 0 to 1; empty (the default) for a
    /// spring that acts on the wheel directly (a fork).
    #[serde(default)]
    pub leverage: Vec<[f32; 2]>,
    /// Compression added to the spring's input as a preload, m.
    #[serde(default)]
    pub rest_offset_m: f32,
    #[serde(default)]
    pub progressive: Progressive,
    #[serde(default)]
    pub hydraulic: Hydraulic,
    /// The largest compression speed the force law sees, m/s; 0 (the default) for no limit.
    #[serde(default)]
    pub max_speed_mps: f32,
    #[serde(default)]
    pub coupling: Coupling,
}

impl Suspension {
    pub(super) fn to_spec(&self) -> SuspensionSpec {
        let d = &self.damper;
        let progressive =
            |p: &Progressive| ProgressiveSpec { start: p.start, factor: p.factor, linearity: p.linearity };
        let c = &self.coupling;
        SuspensionSpec {
            max_travel: self.max_travel_m,
            spring: self.spring_n_per_m,
            top_out_stiffness: self.top_out_stiffness_n_per_m,
            top_out_length: self.top_out_length_m,
            damper: DamperSpec {
                slow_bump: d.slow_bump,
                fast_bump: d.fast_bump,
                bump_threshold: d.bump_threshold,
                slow_rebound: d.slow_rebound,
                fast_rebound: d.fast_rebound,
                rebound_threshold: d.rebound_threshold,
            },
            leverage: pairs(&self.leverage),
            rest_offset: self.rest_offset_m,
            progressive: progressive(&self.progressive),
            hydraulic: HydraulicSpec {
                stiffness: self.hydraulic.stiffness_n_per_m,
                term: ProgressiveSpec {
                    start: self.hydraulic.start,
                    factor: self.hydraulic.factor,
                    linearity: self.hydraulic.linearity,
                },
            },
            max_speed: self.max_speed_mps,
            coupling: CouplingSpec {
                chain: c.chain,
                chain_range: c.chain_range,
                traction: c.traction,
                traction_range: c.traction_range,
                brake: c.brake,
                brake_range: c.brake_range,
            },
        }
    }
}

/// The engine (`[bike.engine]`).
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Engine {
    /// The peak torque the curve's factors multiply, N m.
    pub peak_torque_nm: f32,
    /// `[rpm, share of the peak]`, rpm rising.
    pub torque_curve: Vec<[f32; 2]>,
    /// Drag of the closed throttle at the rev limit, N m, times the friction curve.
    pub friction_torque_nm: f32,
    /// `[rpm / max_rpm, factor]` of the drag; empty (the default) means the factor is the share of the rev limit itself.
    #[serde(default)]
    pub friction_curve: Vec<[f32; 2]>,
    pub idle_rpm: f32,
    pub max_rpm: f32,
    /// Moment of inertia of the rotating engine, kg m^2.
    pub moi_kgm2: f32,
    /// The least throttle below the idle speed; 0 (the default) for none.
    #[serde(default)]
    pub throttle_at_idle: f32,
    /// How long the rev limiter cuts and restores the fuel in turn, s.
    pub limiter_cut_time_s: f32,
    /// The engine-brake reduction (`EngineBrakeReductionFactor` and the load and slip tunings, each `[input low, input high,
    /// output low, output high]`): a factor of 0 (the default) is none; a tuning left out gives 1 for every input.
    #[serde(default)]
    pub brake_reduction_factor: f32,
    #[serde(default = "constant_one")]
    pub brake_reduction_load: [f32; 4],
    #[serde(default = "constant_one")]
    pub brake_reduction_slip: [f32; 4],
}

fn constant_one() -> [f32; 4] {
    [0.0, 0.0, 1.0, 1.0]
}

fn range_map(v: [f32; 4]) -> RangeMap {
    RangeMap { input: [v[0], v[1]], output: [v[2], v[3]] }
}

impl Engine {
    pub(super) fn to_spec(&self) -> EngineSpec {
        EngineSpec {
            peak_torque: self.peak_torque_nm,
            torque_curve: pairs(&self.torque_curve),
            friction_torque: self.friction_torque_nm,
            friction_curve: pairs(&self.friction_curve),
            idle_rpm: self.idle_rpm,
            max_rpm: self.max_rpm,
            moi: self.moi_kgm2,
            throttle_at_idle: self.throttle_at_idle,
            limiter_cut_time: self.limiter_cut_time_s,
            brake_reduction: EngineBrakeReduction {
                factor: self.brake_reduction_factor,
                load: range_map(self.brake_reduction_load),
                slip: range_map(self.brake_reduction_slip),
                ..EngineBrakeReduction::default()
            },
        }
    }
}

/// The anti-lock brake (`[bike.brakes.abs]`); every key left out is 0 and the ABS is off.
#[derive(Debug, Clone, Copy, Default, PartialEq, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct Abs {
    pub front_intensity: f32,
    pub rear_intensity: f32,
    pub speed_range_mps: [f32; 2],
    pub front_slip: [f32; 2],
    pub rear_slip: [f32; 2],
    /// The brake reduction (`[bike.brakes.abs.reduction]`); every term off by default.
    pub reduction: super::bike_reduction::Reduction,
}

/// The brakes (`[bike.brakes]`).
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Brakes {
    /// The force of the front brake and of the rear brake at the tyre patch at full lever, N.
    pub front_force_n: f32,
    pub rear_force_n: f32,
    /// The linked brakes: the rear is at least this multiple of the front. When either share below is above 0 they replace it.
    #[serde(default)]
    pub joined_factor: f32,
    #[serde(default)]
    pub joined_front: f32,
    #[serde(default)]
    pub joined_rear: f32,
    #[serde(default)]
    pub abs: Abs,
}

impl Brakes {
    pub(super) fn to_spec(self) -> BrakeSpec {
        BrakeSpec {
            front_force: self.front_force_n,
            rear_force: self.rear_force_n,
            joined_factor: self.joined_factor,
            joined_front: self.joined_front,
            joined_rear: self.joined_rear,
            abs: AbsSpec {
                front_intensity: self.abs.front_intensity,
                rear_intensity: self.abs.rear_intensity,
                speed_range: self.abs.speed_range_mps,
                front_slip: self.abs.front_slip,
                rear_slip: self.abs.rear_slip,
                reduction: self.abs.reduction.to_spec(),
            },
        }
    }
}

/// The steering (`[bike.steering]`).
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Steering {
    /// The handlebar's lock to each side, rad.
    pub max_angle_rad: f32,
    /// The handlebar filter: stiffness, 1/s^2, and damping ratio.
    pub response: f32,
    pub damping_ratio: f32,
    /// The upper bound of the lean limit, rad.
    pub max_lean_rad: f32,
    pub lean: Lean,
}

impl Steering {
    pub(super) fn to_spec(&self) -> SteeringSpec {
        SteeringSpec {
            max_angle: self.max_angle_rad,
            response: self.response,
            damping_ratio: self.damping_ratio,
            max_lean: self.max_lean_rad,
            lean: self.lean.to_spec(),
        }
    }
}

/// When the rider has lost the bike (`[bike.fall]`).
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Fall {
    pub lean_rad: f32,
    pub pitch_rad: f32,
    pub speed_direction_rad: f32,
    pub angular_speed: f32,
    pub min_speed_mps: f32,
    /// The block-fall assist (a ratio of 0, the default, switches it off).
    #[serde(default)]
    pub block_ratio: f32,
    #[serde(default)]
    pub block_min_speed_mps: f32,
    #[serde(default)]
    pub block_time_s: f32,
    #[serde(default)]
    pub block_wait_s: f32,
    /// The lean limit on the ground follows the front tyre's grip (`atan(mu)`), rising and falling by these shares of the gap per
    /// step (default: it stays at `lean_rad`), and how long in s the lean must stay beyond it before it is a fall.
    #[serde(default)]
    pub lean_by_grip: bool,
    #[serde(default)]
    pub lean_rise: f32,
    #[serde(default)]
    pub lean_fall: f32,
    #[serde(default)]
    pub lean_duration_s: f32,
    /// The limits with no wheel on the road: lean and pitch in rad, angular speed in rad/s (0, the default: the limits on the
    /// ground).
    #[serde(default)]
    pub airborne_lean_rad: f32,
    #[serde(default)]
    pub airborne_pitch_rad: f32,
    #[serde(default)]
    pub airborne_angular_speed: f32,
    /// A velocity against the heading with the lean under this (rad) is a high side, otherwise a low side (0, the default: always a
    /// low side). It decides how the rider is thrown (`ride6-rider-fall.md`).
    #[serde(default)]
    pub highside_lean_rad: f32,
    /// The road's bank moves the lean the tests see: `lean - f x bank`, `f` the influence (default 0: no term) when the two lean the
    /// same way and the reduction (default 1) when not, the shift held to `terrain_max_delta_rad` (0: no hold).
    #[serde(default)]
    pub terrain_influence: f32,
    #[serde(default = "one")]
    pub terrain_reduction: f32,
    #[serde(default)]
    pub terrain_max_delta_rad: f32,
}

impl Fall {
    pub(super) fn to_spec(self) -> FallSpec {
        FallSpec {
            lean: self.lean_rad,
            pitch: self.pitch_rad,
            speed_direction: self.speed_direction_rad,
            angular_speed: self.angular_speed,
            min_speed: self.min_speed_mps,
            block_ratio: self.block_ratio,
            block_min_speed: self.block_min_speed_mps,
            block_time: self.block_time_s,
            block_wait: self.block_wait_s,
            extra: FallExtra {
                lean_by_grip: self.lean_by_grip,
                lean_rise: self.lean_rise,
                lean_fall: self.lean_fall,
                lean_duration: self.lean_duration_s,
                airborne_lean: self.airborne_lean_rad,
                airborne_pitch: self.airborne_pitch_rad,
                airborne_angular: self.airborne_angular_speed,
                highside_lean: self.highside_lean_rad,
                terrain_influence: self.terrain_influence,
                terrain_reduction: self.terrain_reduction,
                terrain_max_delta: self.terrain_max_delta_rad,
            },
        }
    }
}
