//! Police pursuit as pure rules: how hot a chase is ([`heat`]), which offences the player commits ([`infraction`],
//! [`rules`]), how cops box in or pit a car ([`tactics`]) and how the police helicopter flies and sees
//! ([`helicopter`]). Nothing here reads a world or a clock: the caller measures the player each frame and applies
//! the decisions. The values that differ between games (fines, cop waves, per-level settings) come from the caller.
//!
//! Specs: `docs/specs/ai-pursuit-heat.md`, `docs/specs/ai-pursuit-tactics.md`, `docs/specs/ai-helicopter.md`.

pub mod evade;
pub mod formation;
pub mod heat;
pub mod helicopter;
pub mod infraction;
pub mod roadblock;
pub mod rules;
pub mod support;
pub mod tactics;

pub use evade::Evade;
pub use heat::Heat;
pub use infraction::{Fines, Infraction, InfractionLog};
pub use rules::{Contact, Target, contact_infraction};
pub use tactics::{Tactic, TacticInput};
