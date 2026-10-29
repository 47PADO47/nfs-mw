//! The lean help's factor and its split between a force and a couple (`docs/specs/ride6-chassis-round6.md`, "Lean help").
//!
//! The help pulls the front tyre's side force towards the one the lean controller wants. How much of the gap it closes is the
//! help factor: the speed curve, cut in a skid, blended up to 1 at a standstill. How the resulting force acts on the bike is the
//! balance: above 30 km/h it is the force at the front contact; slower it turns into the same force's moment about the centre of
//! gravity with no net force, which is what holds a slow or standing bike up.

use crate::bike::curve::{curve, factor, ramp};
use crate::bike::spec::LeanSpec;

/// The speeds (m/s) between which the balance goes from the couple to its tuned share, whatever the tuning: 0 at 1 m/s, 1 at 3 m/s
/// (a recovered constant of the reference game, `ride6-chassis-round6.md`).
pub const BALANCE_FADE: [f32; 2] = [1.0, 3.0];

/// What the help factor depends on this step.
#[derive(Clone, Copy, Debug)]
pub struct HelpInput {
    /// The speed (m/s, absolute).
    pub speed: f32,
    /// The angle between the velocity and the heading (rad, absolute).
    pub heading_error: f32,
    /// The throttle less the brake, -1 to 1.
    pub pedals: f32,
}

/// The help factor, 0 to 1: the share of the gap between the front tyre's side force and the wanted one that the help closes.
pub fn help_factor(spec: &LeanSpec, input: &HelpInput) -> f32 {
    let extra = &spec.help_extra;
    let tuned = curve(&spec.help_speed, input.speed).clamp(0.0, 1.0);
    let skid = ramp(input.heading_error, extra.skid_range).powf(extra.skid_linearity.max(0.0));
    let cut = curve(&extra.skid_reduction, input.speed) * factor(&extra.skid_input, input.pedals);
    let value = tuned * (1.0 - cut * skid).clamp(0.0, 1.0);
    // From the stopped speed range upwards the tuned factor, below it all the way to 1 at a standstill.
    let value = match extra.stopped_range[1] > 0.0 {
        true => 1.0 + ramp(input.speed, extra.stopped_range) * (value - 1.0),
        false => value,
    };
    value.clamp(0.0, 1.0)
}

/// The share of the help force that acts as a force at the front contact (the rest is a couple), 0 to 1.
pub fn balance(spec: &LeanSpec, speed: f32) -> f32 {
    let tuned = factor(&spec.help_extra.balance_speed, speed).clamp(0.0, 1.0);
    match speed < BALANCE_FADE[1] {
        true => tuned * ramp(speed, BALANCE_FADE),
        false => tuned,
    }
}

/// The roll torque (N m, about the forward axis, positive for a roll to the left) of the part of the help that is not a force.
///
/// In the reference game this part is the moment of the whole help force about the centre of gravity, `(contact - cog) x F`, that
/// closes `share` of the gap to the torque the controller wants (its force is `F = share (F_wanted - F_tyre)`, capped by what the
/// tyre can give, `cap`). This model realises the same roll over a 60 Hz step with the torque that closes `share` of the gap
/// between `wanted` (the torque the controller prescribes) and `others` (what the other forces already put on the roll axis),
/// with the same cap: a roll torque of at most `cap x arm`, `arm` the height of the centre of gravity over the contact. The moment
/// about the vertical axis that the game's couple also has is left out.
pub fn couple_torque(share: f32, balance: f32, wanted: f32, others: f32, cap: f32, arm: f32) -> f32 {
    let limit = (1.0 - balance) * cap.max(0.0) * arm.abs();
    ((1.0 - balance) * share * (wanted - others)).clamp(-limit, limit)
}
