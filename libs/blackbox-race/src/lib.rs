//! The rules of a race for EA Black Box games, as pure functions and small state machines.
//! Spec: `docs/specs/race-mode.md`, `docs/specs/ai-racers-catchup.md`.
//!
//! - [`Gate`]: an oriented box across the road with a point test and a segment test.
//! - [`Course`] and [`Progress`]: the ordered checkpoints, the finish line and the laps, and the events a racer's
//!   movement raises.
//! - [`Race`]: the course, the [`Clock`] and the racers together; it raises [`RaceEvent`]s and gives the places.
//! - [`ranking`]: the order of the racers. [`knockout_due`]: the lap knockout rule. [`SpeedTraps`]: points.
//! - [`CatchUp`]: the rubber band controller.
//!
//! No I/O and no dependency on vehicles, roads or rendering. The caller picks the up axis and passes all tuning.

mod catchup;
mod clock;
mod course;
mod gate;
mod knockout;
mod points;
mod race;
pub mod ranking;

#[cfg(test)]
mod tests;

pub use catchup::{CatchUp, CatchUpGains, HISTORY};
pub use clock::{Clock, ClockEvent, Phase};
pub use course::{Course, Event, Progress};
pub use gate::Gate;
pub use knockout::knockout_due;
pub use points::{KMH_TO_MS, SpeedTraps, overtime_loss};
pub use race::{Outcome, Race, RaceConfig, RaceEvent, Racer};
pub use ranking::Standing;
