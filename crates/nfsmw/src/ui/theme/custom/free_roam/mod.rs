//! Free roam (`docs/specs/custom-hud/modes.md` section 2): the smallest amount of persistent information. Beside the
//! cluster and the map, free roam adds only what the moment calls for, in the top-left column under the radio card:
//!
//! - [`discovery`]: which places near the car are announced, never twice;
//! - [`chips`]: the discovery chips and the entry card with the live Interact binding (drawn in the `Banner` slot);
//! - [`extras`]: the optional cash, bounty and GPS distance line (`[custom_hud] free_roam_extras`, the `BountyRow`
//!   slot).

pub mod chips;
pub mod discovery;
mod extras;
#[cfg(test)]
mod tests;

pub use extras::Extras;

/// Where the top-left column starts, units down from the safe area's top at scale 1: under the radio card's zone
/// (three lines and their padding).
pub const BELOW_RADIO: f32 = 66.0;
/// The extras line's height at scale 1, and the gap under it.
pub const EXTRAS_HEIGHT: f32 = 16.0;
pub const GAP: f32 = 4.0;
