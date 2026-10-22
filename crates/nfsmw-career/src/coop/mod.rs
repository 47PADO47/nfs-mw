//! Co-op career rules: two or more players share one profile (docs/specs/coop-career.md, ADR 0010). Every rule here is
//! [ours]; the original has no co-op career.

pub mod names;
pub mod pursuit;
pub mod race;

pub use names::player_name;
pub use race::credited_place;
