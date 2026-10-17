//! The career economy rules (`docs/specs/career-economy.md`, `docs/specs/career-economy-flows.md`).
//!
//! - [`infraction`]: the eight kinds and the fine table.

pub mod infraction;

pub use infraction::{FINES, InfractionKind, InfractionMask, fine_of_counters};
