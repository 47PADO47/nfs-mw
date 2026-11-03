//! The rider's lean controller of the `[bike]` table (`[bike.steering.lean]`): the keys of the controller, its help, the stability help and
//! the terrain term. Same units as `blackbox_vehicle::bike::LeanSpec`.

use blackbox_vehicle::bike::{HelpExtra, LeanSpec, StabilitySpec, TerrainLimit};
use serde::Deserialize;

use super::bike_parts::pairs;

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
    /// The speeds in m/s between which the help factor goes from 1 at a standstill to its tuned value (`[0, 0]`, the default:
    /// no blend).
    #[serde(default)]
    pub help_stopped_range_mps: [f32; 2],
    /// `[speed in m/s, share]` of the help that is a force at the front contact; the rest is a couple about the centre of
    /// gravity. Empty, the default: all of it is a force.
    #[serde(default)]
    pub help_balance_speed: Vec<[f32; 2]>,
    /// The skid cut of the help factor: the heading error in rad at which it starts and at which it is full, the exponent of
    /// the ramp, the cut against the speed `[m/s, cut]` and its factor against throttle less brake `[input, factor]`. No cut
    /// curve, the default: no cut.
    #[serde(default)]
    pub help_skid_range_rad: [f32; 2],
    #[serde(default)]
    pub help_skid_linearity: f32,
    #[serde(default)]
    pub help_skid_reduction: Vec<[f32; 2]>,
    #[serde(default)]
    pub help_skid_input: Vec<[f32; 2]>,
    /// The stability help: the factor against the speed `[m/s, factor]` (empty, the default: no help), its lean term
    /// (`max(0, |lean|^sensitivity x influence + 1)`, default 0 and 1), the cut by the throttle and the brake lever against the speed
    /// `[m/s, cut]`, the speeds in m/s over which it comes in (default 2.78 to 4.17), the lean in rad the desired yaw rate is held to
    /// (default 1.309; 0 for none) and whether its axis is the road's up.
    #[serde(default)]
    pub stability_speed: Vec<[f32; 2]>,
    #[serde(default)]
    pub stability_lean_influence: f32,
    #[serde(default = "one")]
    pub stability_lean_sensitivity: f32,
    #[serde(default)]
    pub stability_throttle_reduction: Vec<[f32; 2]>,
    #[serde(default)]
    pub stability_brake_reduction: Vec<[f32; 2]>,
    #[serde(default = "stability_range")]
    pub stability_speed_range_mps: [f32; 2],
    #[serde(default = "stability_max_lean")]
    pub stability_max_lean_rad: f32,
    #[serde(default)]
    pub stability_terrain_normal: bool,
    /// The terrain term of the lean limit: the factor on the road's bank against the speed `[m/s, factor]` (empty, the default: no
    /// term), the most the scaled bank counts in rad (default 1.571), and the shares of it added to the limit for a bank that favours
    /// the lean and one that does not (default 1 and 1).
    #[serde(default)]
    pub terrain_speed: Vec<[f32; 2]>,
    #[serde(default = "quarter_turn")]
    pub terrain_max_angle_rad: f32,
    #[serde(default = "one")]
    pub terrain_increment: f32,
    #[serde(default = "one")]
    pub terrain_reduction: f32,
}

fn quarter_turn() -> f32 {
    std::f32::consts::FRAC_PI_2
}

pub(super) fn one() -> f32 {
    1.0
}

fn stability_range() -> [f32; 2] {
    [10.0 / 3.6, 15.0 / 3.6]
}

fn stability_max_lean() -> f32 {
    75.0_f32.to_radians()
}

impl Lean {
    pub(super) fn to_spec(&self) -> LeanSpec {
        let l = self;
        LeanSpec {
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
            help_extra: HelpExtra {
                stopped_range: l.help_stopped_range_mps,
                balance_speed: pairs(&l.help_balance_speed),
                skid_range: l.help_skid_range_rad,
                skid_linearity: l.help_skid_linearity,
                skid_reduction: pairs(&l.help_skid_reduction),
                skid_input: pairs(&l.help_skid_input),
            },
            terrain: TerrainLimit {
                speed: pairs(&l.terrain_speed),
                max_angle: l.terrain_max_angle_rad,
                increment: l.terrain_increment,
                reduction: l.terrain_reduction,
            },
            stability: StabilitySpec {
                speed: pairs(&l.stability_speed),
                lean_influence: l.stability_lean_influence,
                lean_sensitivity: l.stability_lean_sensitivity,
                throttle_reduction: pairs(&l.stability_throttle_reduction),
                brake_reduction: pairs(&l.stability_brake_reduction),
                speed_range: l.stability_speed_range_mps,
                max_lean: l.stability_max_lean_rad,
                uses_terrain_normal: l.stability_terrain_normal,
            },
        }
    }
}
