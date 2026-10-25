//! The race layout (`docs/specs/custom-hud/race.md`): the intro card and the countdown, a compact status strip, a small
//! leaderboard and the hand-off to the results flow. The cluster and the map are the same components as in free roam.
//!
//! - [`strip`]: the status strip (slot `RaceStatus`): position, lap or checkpoint, progress, distance, rival gap.
//! - [`gap`]: the rival gap in words, never a signed number.
//! - [`leaderboard`]: the compact leaderboard (slot `Leaderboard`).
//! - [`intro`]: the intro card and the countdown (drawn in the `Banner` slot).
//!
//! At `RaceResults` the strip and the board leave; the results flow's screens take over and the banner shows the
//! race-over line as the package did.

pub mod gap;
pub mod intro;
pub mod leaderboard;
pub mod strip;
#[cfg(test)]
mod tests;

pub use leaderboard::Leaderboard;
pub use strip::RaceStrip;
