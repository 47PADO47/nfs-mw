//! The per-tick events of the host (`nfsmw_sim::SimEvent`), each found from state the world already keeps: the contacts
//! of the tick, the chase's targets and the police's bust, the race engine's racers and the players' cars. The trackers
//! are plain state machines over plain inputs, so their rules are tested without a world.

mod contacts;
mod moved;
mod pursuit;
mod race;

pub use contacts::contact_events;
pub use moved::{CarSpot, MovedTracker};
pub use pursuit::{ChaseState, PursuitTracker};
pub use race::{RaceTracker, RacerState};
