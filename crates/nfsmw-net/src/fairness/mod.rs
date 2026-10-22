//! Rules that keep a session fair (features.md section 3.3), as pure functions and small state machines the server
//! runs each tick:
//!
//! - [`CatchUpRule`] (MP-32): a boost for trailing racers over `blackbox_race::CatchUp`. **Off by default**: the
//!   original has none for human racers.
//! - [`pair_collides`] (MP-33): whether two bodies collide at a tick under the session's [`CollisionMode`].
//! - [`Toggles`] (MP-34): nitrous on or off, damage visual or off, repair in the lobby.
//! - [`ResetRule`] (MP-35): the server-validated reset, one per 10 s, a time penalty in a race, free in free roam.
//! - [`WrongWay`]: the wrong-way notice from the heading against the route.
//!
//! [`CollisionMode`]: crate::message::CollisionMode

mod catch_up;
mod collision;
mod reset;
mod toggles;
mod wrong_way;

#[cfg(test)]
mod tests;

pub use catch_up::{CATCH_UP_SAMPLE_TICKS, CatchUpRule, MAX_BOOST, race_percent};
pub use collision::{Body, BodyKind, GHOST_START_TICKS, pair_collides};
pub use reset::{RESET_COOLDOWN_TICKS, RESET_PENALTY_MS, ResetGrant, ResetRefusal, ResetRule, ResetState};
pub use toggles::Toggles;
pub use wrong_way::{
    WRONG_WAY_CLEAR_TICKS, WRONG_WAY_DOT, WRONG_WAY_MIN_SPEED, WRONG_WAY_SHOW_TICKS, WrongWay, heading_dot,
};
