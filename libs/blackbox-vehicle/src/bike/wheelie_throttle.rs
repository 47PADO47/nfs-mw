//! GAME-SPECIFIC, not RIDE 6: a wheelie asked for with the throttle alone (`docs/specs/ride6-wheelie-throttle.md`).
//!
//! The reference lifts and holds a wheelie with the weight lever pulled back and the throttle (`ride6-tricks.md`, section 1); with
//! the lever released the rider help cuts the throttle from 4 degrees up and the auto-rider puts the weight forward, so a launch
//! never lifts the front wheel ([`ride6-wheelie-hold.md`] section 1). A player on a pad or a keyboard asked to hold a wheelie
//! "spamming the gas", without the lever. This module is the **virtual lever**: while the throttle is flat out and nothing else
//! forbids it, the bike behaves as if the rider held the lever fully back, so the whole existing wheelie model (the pitch assist's
//! target, the forced-wheelie cut at 33 to 45 degrees, the auto-rider's weight at the back, the gentle lowering) acts as it does
//! with the real lever and its measured behaviour (no loop-out, held by blips) carries over. Nothing but the weight the help
//! works with changes; the engine and the tyres do the lifting, so a bike whose power-to-weight cannot lift its front wheel
//! stays down.
//!
//! The constants are one group, [`ThrottleWheelie`]: [`ThrottleWheelie::GAME`] is what the game plays with,
//! [`ThrottleWheelie::REFERENCE`] switches it off and gives the reference's behaviour back exactly
//! (`Bike::set_throttle_wheelie`).

use super::pitch_assist::FREE_PEDAL;

/// The settings of the throttle-only wheelie.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ThrottleWheelie {
    /// Whether the throttle alone asks for a wheelie.
    pub enabled: bool,
    /// The throttle (0 to 1) at or above which it does: a gas that is spammed or held flat out, not one that is rolled on.
    pub throttle: f32,
    /// The virtual lever the throttle pulls (-1 fully back). The pitch assist's target is the lever times the throttle times its
    /// maximum angle, so this also scales the height of the wheelie.
    pub lever: f32,
    /// The real lever (-1 to 1) must be within this of its rest for the virtual one to act: a rider who moves it is in charge.
    pub free_lever: f32,
    /// The fastest the throttle starts a wheelie (m/s); a wheelie already up is kept at any speed.
    pub max_speed: f32,
    /// The share of the lean limit (0 upright, 1 at the limit) above which the throttle starts no wheelie: the bike is cornering.
    pub max_lean: f32,
    /// The pitch (rad, nose up) above which a wheelie that was asked for stays asked for after the throttle is shut: the virtual lever
    /// holds until the front wheel is down, so the nose is let down gently ([`super::pitch_assist::lowering_torque`]) and a blip of
    /// the throttle lifts it again.
    pub hold_pitch: f32,
}

impl ThrottleWheelie {
    /// The reference's behaviour: the throttle alone asks for nothing.
    pub const REFERENCE: Self = Self {
        enabled: false,
        throttle: 1.0,
        lever: 0.0,
        free_lever: 0.0,
        max_speed: 0.0,
        max_lean: 0.0,
        hold_pitch: 0.0,
    };

    /// What this game plays with: flat out (0.9) with the lever at rest pulls it fully back, below 40 m/s (144 km/h) and
    /// under half the lean limit; the nose stays asked for until it is under 3 degrees.
    pub const GAME: Self = Self {
        enabled: true,
        throttle: 0.9,
        lever: -1.0,
        free_lever: 0.2,
        max_speed: 40.0,
        max_lean: 0.5,
        hold_pitch: 0.052,
    };
}

/// The library's default is the reference's behaviour; the game switches [`ThrottleWheelie::GAME`] on for the bikes it drives.
impl Default for ThrottleWheelie {
    fn default() -> Self {
        Self::REFERENCE
    }
}

/// What the virtual lever looks at.
#[derive(Clone, Copy, Debug)]
pub(super) struct Ask {
    /// The weight the player holds (-1 back to 1 forward) and the throttle and front brake he asks, before any intervention.
    pub weight: f32,
    pub throttle: f32,
    pub front_brake: f32,
    /// The speed along the bike (m/s), the body's pitch (rad, nose up) and the lean over the lean limit (0 to 1).
    pub speed: f32,
    pub pitch: f32,
    pub lean_share: f32,
    /// Whether the throttle asked for the wheelie in the last step.
    pub held: bool,
}

/// The weight the help works with and whether the throttle is holding the lever back: the player's own weight, or the virtual lever
/// while the throttle asks for a wheelie.
pub(super) fn lever(config: &ThrottleWheelie, s: &Ask) -> (f32, bool) {
    if !config.enabled || s.weight.abs() > config.free_lever || s.front_brake >= FREE_PEDAL {
        return (s.weight, false);
    }
    let start = s.throttle >= config.throttle && s.speed.abs() <= config.max_speed && s.lean_share <= config.max_lean;
    let keep = s.held && s.pitch > config.hold_pitch;
    match start || keep {
        true => (config.lever, true),
        false => (s.weight, false),
    }
}

#[cfg(test)]
#[path = "wheelie_throttle_tests.rs"]
mod tests;
