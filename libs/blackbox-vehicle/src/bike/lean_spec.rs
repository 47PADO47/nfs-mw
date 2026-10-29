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
}
