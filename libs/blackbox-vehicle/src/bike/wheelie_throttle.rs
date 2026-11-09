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
//! The playtest asked for a gentler assist ("it should happen slower, only in first, second and third", "it should go down a little
//! before going up" at a gear change, "if I put it down it shouldn't do it again automatically"), so the virtual lever also carries
//! an **allowed angle** ([`State::reach`], handed to the pitch assist as a ceiling of its target and to the engine as a soft cut when
//! the nose is above it), which
//!
//! - never exceeds the cap of the gear in use ([`ThrottleWheelie::gear_caps`]: 40, 35 and 30 degrees in first, second and third), and
//!   no wheelie is asked in a higher gear;
//! - rises slowly ([`ThrottleWheelie::rise_rate`]) and never runs far ahead of the nose ([`ThrottleWheelie::lead`]);
//! - dips by [`ThrottleWheelie::shift_dip`] below where it was for the length of a gear change (the clutch cuts the gas), then rises again;
//!
//! and a **latch**: once the front wheel has been put down (touchdown, the rider's lever or front brake, or a gear without a cap) the
//! throttle asks for no more wheelies until the bike is back in first gear or stops. None of this acts on a rider's own lever: the
//! cap applies to the assist only.
//!
//! The constants are one group, [`ThrottleWheelie`]: [`ThrottleWheelie::GAME`] is what the game plays with,
//! [`ThrottleWheelie::REFERENCE`] switches it off and gives the reference's behaviour back exactly
//! (`Bike::set_throttle_wheelie`).

use super::curve::ramp;
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
    /// The highest pitch (rad) the assist lifts to in first, second and third gear. No wheelie is asked in a higher gear (0 is none).
    pub gear_caps: [f32; 3],
    /// How fast (rad/s) the allowed angle rises towards the gear's cap: the slowness of the lift.
    pub rise_rate: f32,
    /// How fast (rad/s) the allowed angle comes down (to a dip, to a lower gear's cap, to nothing in a gear without one).
    pub fall_rate: f32,
    /// How far (rad) the allowed angle may be above the nose: a nose that lags (a bike that cannot lift, a lift just let down) is not run away from.
    pub lead: f32,
    /// How far (rad) the allowed angle is cut under where it was while a gear is being changed.
    pub shift_dip: f32,
    /// The least time (s) the dip lasts from the start of a change: the clutch's own time can be shorter than the nose takes to answer.
    pub shift_dip_time: f32,
    /// How far (rad) the nose may stand over the allowed angle before the engine is cut, and how far for the full cut `cut_most`.
    pub cut_over: [f32; 2],
    /// The share of the engine's drive cut at the far end of `cut_over` (0 to 1).
    pub cut_most: f32,
    /// The share of the engine's drive cut while a gear is being changed and for the dip after it: the clutch cuts the gas (0 to 1).
    pub shift_cut: f32,
    /// The speed (m/s) under which the bike counts as stopped and the latch lets go.
    pub rearm_speed: f32,
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
        gear_caps: [0.0; 3],
        rise_rate: 0.0,
        fall_rate: 0.0,
        lead: 0.0,
        shift_dip: 0.0,
        shift_dip_time: 0.0,
        cut_over: [0.0, 0.0],
        cut_most: 0.0,
        shift_cut: 0.0,
        rearm_speed: 0.0,
    };

    /// What this game plays with: flat out (0.9) with the lever at rest pulls it fully back, below 40 m/s (144 km/h) and
    /// under half the lean limit; the nose stays asked for until it is under 3 degrees. The playtest's numbers: a cap of 40, 35 and 30
    /// degrees in the first three gears, a rise of 8 degrees a second (10 degrees take 1.25 s from the level, the cap of first gear 5 s), a fall
    /// of 34 degrees a second, a dip of 8 degrees for at least 0.3 s at a gear change, the engine cut from 2 degrees over the allowed
    /// angle to 90 percent at 8 degrees over, and the latch letting go under 1 m/s.
    pub const GAME: Self = Self {
        enabled: true,
        throttle: 0.9,
        lever: -1.0,
        free_lever: 0.2,
        max_speed: 40.0,
        max_lean: 0.5,
        hold_pitch: 0.052,
        gear_caps: [0.698, 0.611, 0.524],
        rise_rate: 0.14,
        fall_rate: 0.6,
        lead: 0.1,
        shift_dip: 0.14,
        shift_dip_time: 0.3,
        cut_over: [0.035, 0.14],
        cut_most: 0.9,
        shift_cut: 0.9,
        rearm_speed: 1.0,
    };

    /// The cap (rad) of `gear` (1 for first); `None` for a gear without one.
    pub fn cap(&self, gear: usize) -> Option<f32> {
        let cap = *self.gear_caps.get(gear.checked_sub(1)?)?;
        (cap > 0.0).then_some(cap)
    }
}

/// The library's default is the reference's behaviour; the game switches [`ThrottleWheelie::GAME`] on for the bikes it drives.
impl Default for ThrottleWheelie {
    fn default() -> Self {
        Self::REFERENCE
    }
}

/// What the virtual lever remembers from step to step.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(super) struct State {
    /// Whether the throttle asked for the wheelie in the last step.
    pub asked: bool,
    /// Whether the front wheel has been off the road since it was asked (a wheelie that really went up).
    pub lifted: bool,
    /// The front wheel was put down: no wheelie is asked until first gear or a stop.
    pub latched: bool,
    /// The angle (rad) the assist may lift to now.
    pub reach: f32,
    /// What is left (s) of the dip of the last gear change, and the allowed angle (rad) it began from.
    pub dip: f32,
    pub dip_from: f32,
    /// The gear and the gear change of the last step.
    pub gear: usize,
    pub shifting: bool,
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
    /// The gear in use (1 for first), whether it is being changed, and whether the front wheel touches the road.
    pub gear: usize,
    pub shifting: bool,
    pub front_on_ground: bool,
    /// The step (s).
    pub dt: f32,
}

/// What the virtual lever gives the bike this step.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Lever {
    /// The weight the help works with: the player's own, or the virtual lever.
    pub weight: f32,
    /// Whether the throttle is holding the lever back.
    pub active: bool,
    /// The ceiling (rad) of the pitch assist's target; infinite for the rider's own lever.
    pub reach: f32,
    /// The share (0 to 1) of the engine's drive to cut: the nose is over the allowed angle.
    pub cut: f32,
}

impl Lever {
    /// The rider's own weight, the throttle asks for nothing.
    pub(super) fn rider(weight: f32) -> Self {
        Self { weight, active: false, reach: f32::INFINITY, cut: 0.0 }
    }
}

/// Move `value` towards `goal`: up by at most `rise` and down by at most `fall` (both already times the step).
fn move_towards(value: f32, goal: f32, rise: f32, fall: f32) -> f32 {
    match goal > value {
        true => (value + rise).min(goal),
        false => (value - fall).max(goal),
    }
}

/// End the throttle's wheelie; `latch` when it went up and was put down.
fn end(state: &mut State, latch: bool) {
    state.latched |= latch;
    state.asked = false;
    state.lifted = false;
    state.reach = 0.0;
    state.dip = 0.0;
    state.dip_from = 0.0;
}

/// One step of the virtual lever: the weight the help works with, the allowed angle and the engine cut, and the latch.
pub(super) fn update(config: &ThrottleWheelie, state: &mut State, s: &Ask) -> Lever {
    if !config.enabled {
        *state = State::default();
        return Lever::rider(s.weight);
    }
    // The latch lets go in first gear (entered) and at a stop.
    if s.speed.abs() < config.rearm_speed || (s.gear == 1 && state.gear != 1) {
        state.latched = false;
    }
    let was = *state;
    state.gear = s.gear;
    state.shifting = s.shifting;
    // The rider's own lever or front brake: his wheelie, his stoppie. A wheelie of the throttle that he ends this way is one put down.
    if s.weight.abs() > config.free_lever || s.front_brake >= FREE_PEDAL {
        end(state, was.asked && was.lifted);
        return Lever::rider(s.weight);
    }
    // The front wheel touched the road after the throttle's wheelie went up: put down.
    if was.asked && was.lifted && s.front_on_ground {
        end(state, true);
        return Lever::rider(s.weight);
    }
    let cap = config.cap(s.gear);
    let start = !state.latched
        && cap.is_some_and(|cap| s.pitch < cap)
        && s.throttle >= config.throttle
        && s.speed.abs() <= config.max_speed
        && s.lean_share <= config.max_lean;
    let keep = was.asked && s.pitch > config.hold_pitch;
    if !(start || keep) {
        end(state, was.asked && was.lifted);
        return Lever::rider(s.weight);
    }
    if !was.asked {
        state.reach = s.pitch.clamp(0.0, cap.unwrap_or(0.0));
        state.lifted = false;
        state.dip = 0.0;
    }
    state.asked = true;
    state.lifted |= !s.front_on_ground && s.pitch > config.hold_pitch;
    // The dip of a gear change: from its start, for the change and at least `shift_dip_time`.
    if s.shifting && !was.shifting {
        state.dip = config.shift_dip_time;
        state.dip_from = state.reach;
    }
    state.dip = (state.dip - s.dt).max(0.0);
    let dipping = s.shifting || state.dip > 0.0;
    let ceiling = cap.unwrap_or(0.0);
    // The dip is `shift_dip` under where the allowed angle was when the change began (or under the cap if that is lower), however high that was.
    let goal = match dipping {
        true => (state.dip_from.min(ceiling) - config.shift_dip).max(0.0),
        false => ceiling,
    };
    let reach = move_towards(state.reach, goal, config.rise_rate * s.dt, config.fall_rate * s.dt);
    state.reach = reach.min(s.pitch.max(0.0) + config.lead);
    let cut = ramp(s.pitch - state.reach, config.cut_over) * config.cut_most;
    let cut = match dipping {
        true => cut.max(config.shift_cut),
        false => cut,
    };
    Lever { weight: config.lever, active: true, reach: state.reach, cut }
}

#[cfg(test)]
#[path = "wheelie_throttle_tests.rs"]
mod tests;
