//! The phase of a chase for the HUD (`hud::PursuitPhase`), read from the pursuit's authoritative state: whether a chase
//! runs, the cool-down's status (`blackbox_pursuit::cooldown`), whether a cop or the helicopter has the player in
//! sight, the bust sequence, and how the last chase ended with the age of its end card. The heat never decides it.
//!
//! The mapping, pinned against `blackbox_pursuit::cooldown`:
//!
//! | input                                                   | phase    |
//! |---------------------------------------------------------|----------|
//! | the bust sequence runs (arrest scene, ticket)           | Busted   |
//! | chase, status `Evaded` (the frame before it ends)       | Escaped  |
//! | chase, status `CoolDown` (the meter is up)              | Cooldown |
//! | chase, status `InitialChase`, in sight                  | Active   |
//! | chase, status `InitialChase`, out of sight (the grace)  | Search   |
//! | no chase, ended `Busted`, end card up                   | Busted   |
//! | no chase, ended `Evaded`, end card up                   | Escaped  |
//! | anything else                                           | None     |
//!
//! "In sight" is the raw contact of the last sight test (`Police::has_contact`) or the helicopter's line: the cool-down
//! counts both as seen. The seven-second grace (`cooldown::TUNING.grace`) runs while no one sees the player and the meter
//! is not up yet; that is the search. A sighting during the cool-down takes the chase back to Active once the display's
//! gate lets it (`Cooldown::update`), which is the reacquire.

use blackbox_pursuit::{CooldownStatus, EndReason};

use crate::hud::PursuitPhase;

/// Seconds the end of a chase holds its phase: the end card's time on the screen (`hud/pursuit_end.rs`).
const END_HOLD: f32 = crate::hud::PURSUIT_END_SECS;

/// What the phase is read from.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(in crate::scenes::world) struct PhaseInput {
    /// A chase runs.
    pub pursuing: bool,
    /// The cool-down's status of the running chase.
    pub status: CooldownStatus,
    /// A cop or the helicopter has the player in sight.
    pub contact: bool,
    /// The bust sequence runs: the arrest scene or its ticket.
    pub arrest: bool,
    /// How the last chase ended (`None` while one runs).
    pub end: EndReason,
    /// Seconds since the last chase ended.
    pub since_end: f32,
}

/// The phase of the chase for `input`; see the module for the table.
pub(in crate::scenes::world) fn phase(input: &PhaseInput) -> PursuitPhase {
    if input.arrest {
        return PursuitPhase::Busted;
    }
    if input.pursuing {
        return match (input.status, input.contact) {
            (CooldownStatus::Evaded, _) => PursuitPhase::Escaped,
            (CooldownStatus::CoolDown, _) => PursuitPhase::Cooldown,
            (CooldownStatus::InitialChase, true) => PursuitPhase::Active,
            (CooldownStatus::InitialChase, false) => PursuitPhase::Search,
        };
    }
    if input.since_end >= END_HOLD {
        return PursuitPhase::None;
    }
    match input.end {
        EndReason::Busted => PursuitPhase::Busted,
        EndReason::Evaded => PursuitPhase::Escaped,
        EndReason::None => PursuitPhase::None,
    }
}

#[cfg(test)]
#[path = "pursuit_phase_tests.rs"]
mod tests;
