//! The pursuit layout (`docs/specs/custom-hud/pursuit.md` section 2), driven by the explicit pursuit phase, never by
//! the heat or the presence of a cop.
//!
//! - [`model`]: what the panel shows per phase, detail level and `[custom_hud] pursuit_info`.
//! - [`panel`]: the compact panel (slot `PursuitMeter`), and the compact strip under the leaderboard in a race.
//! - [`end_card`]: the escape and busted cards (drawn in the `Banner` slot).
//!
//! The bounty is folded into the panel, so the `BountyRow` slot draws the free-roam extras instead; the `PoliceRadar`
//! slot draws nothing, since the map and the edge arrows show what the player can know of the police.

pub mod end_card;
pub mod model;
mod panel;
#[cfg(test)]
mod tests;

pub use panel::PursuitPanel;
