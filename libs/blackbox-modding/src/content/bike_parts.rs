//! The parts of the `[bike]` table under the wheels and in the rider's hands: suspension, engine, gearbox, brakes, steering and
//! the lean controller, the fall and the wheelie assist. Same units as `blackbox_vehicle::bike::BikeSpec` (see `bike.rs`).

use blackbox_vehicle::bike::{
    AbsSpec, BrakeSpec, Coupling as CouplingSpec, DamperSpec, EngineSpec, FallSpec, Hydraulic as HydraulicSpec,
    LeanSpec, PitchParams, Progressive as ProgressiveSpec, SteeringSpec, SuspensionSpec, TransmissionSpec, WheelieSpec,
    choices,
};
use serde::Deserialize;

fn pairs(points: &[[f32; 2]]) -> Vec<(f32, f32)> {
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
        }
    }
}

/// The gearbox and the clutch (`[bike.transmission]`).
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Transmission {
    /// Forward gear ratios, first gear first.
    pub gears: Vec<f32>,
    /// The ratio between the gearbox output and the rear wheel, chain included.
    pub final_ratio: f32,
    /// The share of the engine torque that reaches the wheel.
    pub efficiency: f32,
    /// The engine speed, as a share of the rev limit, at which the automatic changes up under throttle.
    pub up_shift: f32,
    /// ... and below which it changes down. Left out (the default): the model's choice, `choices::DOWN_SHIFT_SHARE`.
    #[serde(default)]
    pub down_shift: Option<f32>,
    /// How long a change up and a change down take, s.
    pub up_shift_time_s: f32,
    pub down_shift_time_s: f32,
    /// The throttle blip on a change down; 0 (the default) for none.
    #[serde(default)]
    pub blip_throttle: f32,
    /// The exponent of the clutch's engagement.
    pub clutch_linearity: f32,
    /// The most torque the clutch carries, as a multiple of the engine's peak torque.
    pub clutch_max_factor: f32,
    /// The back torque range in N m over which the slipper clutch opens, and how far it opens (0 to 1); 0 (the default) for
    /// no slipper.
    #[serde(default)]
    pub slipper_range_nm: [f32; 2],
    #[serde(default)]
    pub slipper_intensity: f32,
    /// The road speed in m/s above which a start is over and the clutch is locked; 0 (the default) for no launch slip.
    #[serde(default)]
    pub release_speed_mps: f32,
    /// The engine speed of a launch, as a share of the engine speed of peak torque, and the most it may be as a share of the
    /// rev limit.
    #[serde(default)]
    pub feather_factor: f32,
    #[serde(default)]
    pub feather_max_factor: f32,
}

impl Transmission {
    pub(super) fn to_spec(&self) -> TransmissionSpec {
        TransmissionSpec {
            gears: self.gears.clone(),
            final_ratio: self.final_ratio,
            efficiency: self.efficiency,
            up_shift: self.up_shift,
            down_shift: self.down_shift.unwrap_or(choices::DOWN_SHIFT_SHARE),
            up_shift_time: self.up_shift_time_s,
            down_shift_time: self.down_shift_time_s,
            blip_throttle: self.blip_throttle,
            clutch_linearity: self.clutch_linearity,
            slipper_range: self.slipper_range_nm,
            slipper_intensity: self.slipper_intensity,
            release_speed: self.release_speed_mps,
            feather_factor: self.feather_factor,
            feather_max_factor: self.feather_max_factor,
            clutch_max_factor: self.clutch_max_factor,
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
            },
        }
    }
}

/// The rider's lean controller (`[bike.steering.lean]`).
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Lean {
    pub k: f32,
    pub damping_ratio: f32,
    pub damping_multiplier: f32,
    pub steer_k: f32,
    pub steer_damping_ratio: f32,
    pub target_k: f32,
    pub target_damping_ratio: f32,
    pub time_scale: f32,
    pub time_scale_attenuation: f32,
    pub time_scale_sensitivity: f32,
    /// `[speed in m/s, factor]` and `[stick, factor]`; empty (the default): no effect.
    #[serde(default)]
    pub time_scale_speed: Vec<[f32; 2]>,
    #[serde(default)]
    pub time_scale_input: Vec<[f32; 2]>,
    /// `[lateral acceleration in g, factor]` of the lean limit; empty: no effect.
    #[serde(default)]
    pub g_tuning: Vec<[f32; 2]>,
    /// The heading error in rad at which a skid starts to cut the lean limit and at which it cuts fully, the exponent of the
    /// ramp, and the cut against the speed `[m/s, cut]`.
    pub skid_range_rad: [f32; 2],
    pub skid_linearity: f32,
    #[serde(default)]
    pub skid_reduction: Vec<[f32; 2]>,
    /// The lean limit at a standstill in rad and the speed in m/s below which it blends in.
    pub stopped_limit_rad: f32,
    pub stopped_speed_mps: f32,
    /// The speeds in m/s between which the handlebar goes from the stick to the controller.
    pub blend_range_mps: [f32; 2],
    pub rider_mass_scale: f32,
    /// The lean help: `[speed in m/s, share of the gap to the wanted front side force it closes]` (empty, the default: no
    /// help) and the share of the lateral peak of the tyre it may add.
    #[serde(default)]
    pub help_speed: Vec<[f32; 2]>,
    #[serde(default)]
    pub help_limit: f32,
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
        let l = &self.lean;
        SteeringSpec {
            max_angle: self.max_angle_rad,
            response: self.response,
            damping_ratio: self.damping_ratio,
            max_lean: self.max_lean_rad,
            lean: LeanSpec {
                k: l.k,
                damping_ratio: l.damping_ratio,
                damping_multiplier: l.damping_multiplier,
                steer_k: l.steer_k,
                steer_damping_ratio: l.steer_damping_ratio,
                target_k: l.target_k,
                target_damping_ratio: l.target_damping_ratio,
                time_scale: l.time_scale,
                time_scale_attenuation: l.time_scale_attenuation,
                time_scale_sensitivity: l.time_scale_sensitivity,
                time_scale_speed: pairs(&l.time_scale_speed),
                time_scale_input: pairs(&l.time_scale_input),
                g_tuning: pairs(&l.g_tuning),
                skid_range: l.skid_range_rad,
                skid_linearity: l.skid_linearity,
                skid_reduction: pairs(&l.skid_reduction),
                stopped_limit: l.stopped_limit_rad,
                stopped_speed: l.stopped_speed_mps,
                blend_range: l.blend_range_mps,
                rider_mass_scale: l.rider_mass_scale,
                help_speed: pairs(&l.help_speed),
                help_limit: l.help_limit,
            },
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
}

impl Fall {
    pub(super) fn to_spec(self) -> FallSpec {
        FallSpec {
            lean: self.lean_rad,
            pitch: self.pitch_rad,
            speed_direction: self.speed_direction_rad,
            angular_speed: self.angular_speed,
            min_speed: self.min_speed_mps,
        }
    }
}

/// One parameter set of the wheelie and stoppie assist (`[bike.wheelie.wheelie]`, `[bike.wheelie.stoppie]`).
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Pitch {
    pub deadzone: f32,
    pub max_angle_rad: f32,
    pub speed_ramp_mps: [f32; 2],
    #[serde(default)]
    pub help_speed: Vec<[f32; 2]>,
    pub lean_help: [f32; 2],
    pub lookahead_s: f32,
    pub min_speed_mps: f32,
    pub acceleration: f32,
    pub acceleration_time_s: f32,
    pub ground_range_m: [f32; 2],
    pub lean_limit_rad: f32,
    pub lean_pitch_range_rad: [f32; 2],
}

impl Pitch {
    fn to_spec(&self) -> PitchParams {
        PitchParams {
            deadzone: self.deadzone,
            max_angle: self.max_angle_rad,
            speed_ramp: self.speed_ramp_mps,
            help_speed: pairs(&self.help_speed),
            lean_help: self.lean_help,
            lookahead: self.lookahead_s,
            min_speed: self.min_speed_mps,
            acceleration: self.acceleration,
            acceleration_time: self.acceleration_time_s,
            ground_range: self.ground_range_m,
            lean_limit: self.lean_limit_rad,
            lean_pitch_range: self.lean_pitch_range_rad,
        }
    }
}

/// The wheelie and stoppie assist and the rider help around it (`[bike.wheelie]`). A range of `[0, 0]` switches a part off.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Wheelie {
    pub wheelie: Pitch,
    pub stoppie: Pitch,
    /// The lift angle in rad over which the throttle is cut, while the rider is not asking for a wheelie and while he is, and
    /// the lift speed in rad/s over which it is cut.
    #[serde(default)]
    pub throttle_pitch_rad: [f32; 2],
    #[serde(default)]
    pub forced_throttle_pitch_rad: [f32; 2],
    #[serde(default)]
    pub throttle_pitch_speed: [f32; 2],
    /// The same for the front brake against a stoppie.
    #[serde(default)]
    pub brake_pitch_rad: [f32; 2],
    #[serde(default)]
    pub brake_pitch_speed: [f32; 2],
}

impl Wheelie {
    pub(super) fn to_spec(&self) -> WheelieSpec {
        WheelieSpec {
            wheelie: self.wheelie.to_spec(),
            stoppie: self.stoppie.to_spec(),
            throttle_pitch: self.throttle_pitch_rad,
            forced_throttle_pitch: self.forced_throttle_pitch_rad,
            throttle_pitch_speed: self.throttle_pitch_speed,
            brake_pitch: self.brake_pitch_rad,
            brake_pitch_speed: self.brake_pitch_speed,
        }
    }
}
