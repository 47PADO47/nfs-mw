//! Police pursuit as pure rules: how hot a chase is ([`heat`]), which offences the player commits ([`infraction`],
//! [`rules`]), how cops box in or pit a car ([`tactics`]) and how the police helicopter flies and sees
//! ([`helicopter`]). Nothing here reads a world or a clock: the caller measures the player each frame and applies
//! the decisions. The values that differ between games (fines, cop waves, per-level settings) come from the caller.
//!
//! The busted meter ([`busted`]) fills while a cop is close, level and the player is slow. The escape ([`cooldown`],
//! with the per-heat tables in [`evade`]) runs the grace, the cool-down meter and EVADED. How a chase ended is
//! [`EndReason`]. A cop car's damage, flipped timer and shock are in [`wreck`] (`docs/specs/ai-pursuit-cop-cars.md`
//! section 6).
//!
//! Specs: `docs/specs/ai-pursuit.md` sections 4 to 6, `docs/specs/ai-pursuit-heat.md`, `docs/specs/ai-pursuit-tactics.md`, `docs/specs/ai-helicopter.md`.

pub mod breaker;
pub mod busted;
pub mod cooldown;
pub mod end;
pub mod evade;
pub mod formation;
pub mod goals;
pub mod heat;
pub mod helicopter;
pub mod infraction;
pub mod roadblock;
pub mod rules;
pub mod sight;
pub mod spikes;
pub mod support;
pub mod tactics;
pub mod wave;
pub mod wreck;

pub use busted::Busted;
pub use cooldown::{Cooldown, CooldownInput, CooldownTuning, Status as CooldownStatus};
pub use end::EndReason;
pub use heat::Heat;
pub use infraction::{Fines, Infraction, InfractionLog};
pub use rules::{Contact, Target, contact_infraction};
pub use tactics::{Tactic, TacticInput};
