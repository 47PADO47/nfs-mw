//! What changed in the world, detected once and read by everyone who needs it: the chase's and the races' snapshots
//! with the notices their change makes (the mods hear them through `modding/outbox.rs` and `modding/races_feed.rs`), and
//! the HUD's events, which take the chase's notices and add the changes only the HUD needs.
//!
//! - `pursuit`: the chase's snapshot and its notices;
//! - `races`: the races' snapshot and its notices.

pub(crate) mod pursuit;
pub(crate) mod races;
