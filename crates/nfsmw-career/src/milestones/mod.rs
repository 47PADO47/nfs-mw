//! Career milestones: the pursuit goals per Blacklist bin (`docs/specs/career-milestones.md`; the numbers are in
//! `docs/specs/career-milestones-data.md`).
//!
//! - [`stat`]: the 30 tracked statistics, the best-value rule and the threshold test.
//! - [`state`]: the four states and [`MilestoneDef`]. The caller passes the definitions in; this crate does not read
//!   `gameplay.bin`.
//! - [`set`]: [`MilestoneSet`] and its transitions: bin entered, statistic update in free roam, pursuit end (bank on
//!   escape, revert on bust), and the bin count. The unconfirmed choices are in the docs of [`set`].
//!
//! - [`speedtrap`]: the 39 speed traps; their done count adds to the bin count.
//!
//! The save block (section 7) is in `crate::save::milestones_block` and the rival gate (section 6) in
//! `crate::blacklist`. Left out: the screens and the heat adjustment (not read by the exe).

mod set;
pub mod speedtrap;
mod stat;
mod state;

#[cfg(test)]
mod tests;

pub use set::{Banked, MilestoneSet, PURSUIT_SUMMARY_MAX, PursuitEndMilestones, StatUpdate, pay_banked};
pub use stat::{STAT_COUNT, StatType, StatValue};
pub use state::{Milestone, MilestoneDef, MilestoneState};
