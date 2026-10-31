//! The rest of the tuning of the lean help and of the fall tests (`docs/specs/ride6-chassis-round6.md`). Every field defaults to
//! "not there": the help is then the plain speed curve of [`super::LeanSpec`] and the fall tests are those of
//! [`super::FallSpec`] alone.

/// The parts of the lean help beyond its speed curve and its limit.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct HelpExtra {
    /// The speeds (m/s) between which the help factor goes from 1 at a standstill to its tuned value: `factor = 1 + ramp
    /// (factor - 1)`. `[0, 0]` (a flat range) switches the blend off.
    pub stopped_range: [f32; 2],
    /// The share of the help that is a force at the front contact, against the speed `(m/s, share)`; the rest is the same force's
    /// moment about the centre of gravity as a pure couple. Empty: all of it is a force.
    pub balance_speed: Vec<(f32, f32)>,
    /// The skid cut of the help factor: the heading error (rad) at which it starts and at which it is full, the exponent of the
    /// ramp, the cut against the speed `(m/s, cut)` and its factor against `throttle - brake` `(input, factor)` (empty: 1).
    /// An empty cut curve is no cut.
    pub skid_range: [f32; 2],
    pub skid_linearity: f32,
    pub skid_reduction: Vec<(f32, f32)>,
    pub skid_input: Vec<(f32, f32)>,
}

/// The parts of the fall tests beyond the four limits of [`super::FallSpec`].
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct FallExtra {
    /// The lean limit on the ground follows the front tyre's grip (`atan(mu) x tolerance`), rising by `lean_rise` and falling by
    /// `lean_fall` of the gap per step, instead of staying at `FallSpec::lean`.
    pub lean_by_grip: bool,
    pub lean_rise: f32,
    pub lean_fall: f32,
    /// How long (s) the lean must stay beyond its limit on the ground before it is a fall.
    pub lean_duration: f32,
    /// The limits when no wheel touches the road: lean (rad), pitch (rad) and angular speed (rad/s). 0 takes the limit on the
    /// ground.
    pub airborne_lean: f32,
    pub airborne_pitch: f32,
    pub airborne_angular: f32,
    /// A velocity against the heading is a high side (the bike is thrown over by a tyre that grips again) when the lean is under
    /// this (rad), a low side when it is over. 0: it is always a low side.
    pub highside_lean: f32,
    /// The terrain's bank (rad, positive when the road is lower on the right) moves the lean the tests see: `lean - f x slope`, `f` the
    /// influence when the slope and the lean lean the same way and the reduction when they do not, the shift held to `terrain_max_delta`
    /// (0: no hold). Influence 0 is no terrain term.
    pub terrain_influence: f32,
    pub terrain_reduction: f32,
    pub terrain_max_delta: f32,
}

/// The terrain term of the lean limit (`docs/specs/ride6-chassis-round7.md`, section 4): the road's bank, scaled by a speed curve and held to
/// a maximum, moves the limit by `increment` of itself when it favours the lean and by `reduction` of itself when it does not.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TerrainLimit {
    /// The factor on the bank against the speed `(m/s, factor)`; empty: no term.
    pub speed: Vec<(f32, f32)>,
    /// The most the scaled bank counts (rad), and the shares of it added to the limit for a favourable and an unfavourable bank.
    pub max_angle: f32,
    pub increment: f32,
    pub reduction: f32,
}

/// The stability help (`docs/specs/ride6-chassis-round7.md`, section 3): a yaw torque that pulls the bike's yaw rate towards the one a
/// steady turn at its lean needs. The default is "off" (an empty speed curve).
#[derive(Clone, Debug, PartialEq)]
pub struct StabilitySpec {
    /// The factor against the speed `(m/s, factor)`; empty: no help.
    pub speed: Vec<(f32, f32)>,
    /// The factor is multiplied by `max(0, |normalised lean|^lean_sensitivity x lean_influence + 1)`.
    pub lean_influence: f32,
    pub lean_sensitivity: f32,
    /// The cut of the factor by the throttle and by the brake lever: each lever times this curve of the speed `(m/s, cut)`.
    pub throttle_reduction: Vec<(f32, f32)>,
    pub brake_reduction: Vec<(f32, f32)>,
    /// The speeds (m/s) between which the help comes in; below the first there is none.
    pub speed_range: [f32; 2],
    /// The lean (rad) the desired yaw rate is computed for is held to this much either side; 0 for no limit.
    pub max_lean: f32,
    /// The axis of the yaw is the road's up (perpendicular to the line between the two contacts) rather than the world's.
    pub uses_terrain_normal: bool,
}

impl Default for StabilitySpec {
    fn default() -> Self {
        Self {
            speed: Vec::new(),
            lean_influence: 0.0,
            lean_sensitivity: 1.0,
            throttle_reduction: Vec::new(),
            brake_reduction: Vec::new(),
            speed_range: [10.0 / 3.6, 15.0 / 3.6],
            max_lean: 75.0_f32.to_radians(),
            uses_terrain_normal: false,
        }
    }
}
