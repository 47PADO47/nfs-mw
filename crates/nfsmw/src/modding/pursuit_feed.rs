//! The chase as the mods see it: the snapshot the world publishes in its police update, the notices the mods hear when
//! the snapshot changes (`observe`), and the requests the mods make of the chase.
//!
//! A snapshot is plain data, so the rules that turn two of them into notices can be tested without a world. The world
//! publishes once a frame, at the point where the bust and the escape are decided, so a bust is seen in the frame it
//! happens; the end of the chase follows in the next frame's snapshot.

use blackbox_pursuit::{EndReason, Fines, Infraction, InfractionLog};
use nfsmw_modding::abi::{EndMode, EndReason as MwEndReason, Infraction as MwInfraction};

/// The chase at one moment, as the world reports it. Without a chase `pursuing` is false and the counts are zero; the
/// infraction log and the busted meter keep their last values until the next chase.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PursuitView {
    /// A chase is on.
    pub pursuing: bool,
    /// The heat value, 1 to 10 with its fraction; 0 without a chase.
    pub heat: f32,
    /// The heat level, the integer part of `heat`; 0 without a chase.
    pub level: u32,
    /// Seconds of pursuit so far.
    pub secs: f32,
    /// Cop cars in the chase now.
    pub cops: u32,
    /// Cops in the wave of the current heat, the helicopter not counted.
    pub wave_cops: u32,
    /// The player has been busted in this chase.
    pub busted: bool,
    /// The busted meter, 0 to 1.
    pub busted_meter: f32,
    /// How the last chase ended (`None` while one runs, and before any chase).
    pub end_reason: EndReason,
    /// The infractions of the chase.
    pub infractions: InfractionLog,
    /// The fine of each infraction.
    pub fines: Fines,
}

impl PursuitView {
    /// A view with no chase, with the install's `fines`.
    pub fn idle(fines: Fines) -> Self {
        PursuitView {
            pursuing: false,
            heat: 0.0,
            level: 0,
            secs: 0.0,
            cops: 0,
            wave_cops: 0,
            busted: false,
            busted_meter: 0.0,
            end_reason: EndReason::None,
            infractions: InfractionLog::default(),
            fines,
        }
    }
}

/// What the mods hear: one `nfsmw.*` event, with the values it carries. The frame is added when it is sent.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Notice {
    /// A chase began at `level`, with `cops` in its first wave.
    PursuitStarted { level: u32, cops: u32, heat: f32 },
    /// A chase ended for `reason`, with `bounty` (the fines of the chase) and lasting `duration` seconds.
    PursuitEnded { reason: MwEndReason, bounty: u64, duration: f32 },
    /// The heat moved to another level; `old_heat` and `new_heat` are the values either side of the move.
    HeatChanged { level: u32, old_heat: f32, new_heat: f32 },
    /// The player was busted at `level`, with `bounty` on the chase.
    Busted { level: u32, bounty: u64 },
    /// The player got away at `level`, after `duration` seconds of chase.
    Escaped { level: u32, duration: f32 },
    /// A new infraction was recorded, with its fine.
    InfractionRecorded { infraction: MwInfraction, fine: u32 },
}

/// A change a mod asks of the chase. The world applies them in its police update, in order.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PursuitRequest {
    /// Set the heat of the running chase. The world keeps it within the chase's range.
    SetHeat(f32),
    /// End the running chase the way `mode` says.
    End(EndMode),
}

/// The notices the change from `before` to `now` makes, in the order the mods hear them.
pub fn observe(before: &PursuitView, now: &PursuitView) -> Vec<Notice> {
    let mut out = Vec::new();
    if !before.pursuing && now.pursuing {
        out.push(Notice::PursuitStarted { level: now.level, cops: now.wave_cops, heat: now.heat });
    }
    if before.pursuing && now.pursuing && before.level != now.level {
        out.push(Notice::HeatChanged { level: now.level, old_heat: before.heat, new_heat: now.heat });
    }
    if now.pursuing {
        out.extend(new_infractions(before, now));
    }
    if !before.busted && now.busted {
        out.push(Notice::Busted { level: now.level, bounty: u64::from(now.infractions.total_fines()) });
    }
    if before.pursuing && !now.pursuing && now.end_reason == EndReason::Evaded {
        out.push(Notice::Escaped { level: before.level, duration: now.secs });
    }
    if before.pursuing && !now.pursuing {
        out.push(Notice::PursuitEnded {
            reason: reason_of(now.end_reason),
            bounty: u64::from(now.infractions.total_fines()),
            duration: now.secs,
        });
    }
    out
}

/// The infractions of `now` that `before` did not have. A chase that starts had an empty log: the last chase's log is
/// cleared when the next one begins.
fn new_infractions(before: &PursuitView, now: &PursuitView) -> Vec<Notice> {
    let known = match before.pursuing {
        true => before.infractions,
        false => InfractionLog::default(),
    };
    now.infractions
        .iter()
        .filter(|infraction| !known.contains(*infraction))
        .map(|infraction| Notice::InfractionRecorded {
            infraction: infraction_of(infraction),
            fine: now.fines.of(infraction),
        })
        .collect()
}

/// The wire reason of a chase's end. A chase stopped without a result (the `pursuit off` command) is `Aborted`.
fn reason_of(reason: EndReason) -> MwEndReason {
    match reason {
        EndReason::Busted => MwEndReason::Busted,
        EndReason::Evaded => MwEndReason::Evaded,
        EndReason::None => MwEndReason::Aborted,
    }
}

/// The wire infraction of a `blackbox_pursuit` one. Both list the offences in the same order.
fn infraction_of(infraction: Infraction) -> MwInfraction {
    match infraction {
        Infraction::Speeding => MwInfraction::Speeding,
        Infraction::Racing => MwInfraction::Racing,
        Infraction::RecklessDriving => MwInfraction::RecklessDriving,
        Infraction::Assault => MwInfraction::Assault,
        Infraction::HitAndRun => MwInfraction::HitAndRun,
        Infraction::DamageToProperty => MwInfraction::DamageToProperty,
        Infraction::ResistingArrest => MwInfraction::ResistingArrest,
        Infraction::OffRoad => MwInfraction::OffRoad,
    }
}

#[cfg(test)]
mod tests;
