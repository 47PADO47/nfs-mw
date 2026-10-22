//! Mode events as the gameplay messages of protocol-messages.md section 7.6.
//!
//! The head carries the tick; `seq` is 0 and the session stamps it per connection when it sends. A player without a
//! seat in the roster is left out. Mappings:
//!
//! - lap, finish: `LapComplete`, `RaceFinish`;
//! - spectate: `RoleChanged` to spectator;
//! - pursuit start, a target added, a heat change: `PursuitStart` with the pursuit's id, the cars chased now and the
//!   heat (a `PursuitStart` for a known id updates it), and for a heat change one `HeatChange` per chased car;
//! - one target's end: `PursuitEnd` for that car (the pursuit's own end has no message of its own: its last target's
//!   `PursuitEnd` is it).
//!
//! Knockouts, trap scores, tag passes, penalties and the end of a round have no message yet; the end of a round is the
//! `RaceResults` of its record ([`crate::results::RaceRecord::to_message`]) and the lobby's phase change.

use nfsmw_sim::{PlayerId, Tick};

use crate::message::{
    EventHead, HeatChange, LapComplete, List, Message, PursuitEnd, PursuitStart, RaceFinish, Role, RoleChanged,
};
use crate::results::Roster;

use super::event::ModeEvent;

/// The messages that tell clients about `event`, which happened at `tick`.
pub fn messages(event: &ModeEvent, tick: Tick, roster: &Roster) -> Vec<Message> {
    let head = EventHead { seq: 0, tick, target_tick: 0 };
    let entity = |player: PlayerId| roster.get(player).map(|seat| seat.entity);
    match event {
        ModeEvent::LapCompleted { player, lap, time_ms, .. } => entity(*player)
            .map(|entity_id| Message::LapComplete(LapComplete { head, entity_id, lap: *lap, time_ms: *time_ms }))
            .into_iter()
            .collect(),
        ModeEvent::Finished { player, place, time_ms } => entity(*player)
            .map(|entity_id| Message::RaceFinish(RaceFinish { head, entity_id, place: *place, time_ms: *time_ms }))
            .into_iter()
            .collect(),
        ModeEvent::Spectate { player } => roster
            .get(*player)
            .map(|seat| Message::RoleChanged(RoleChanged { head, slot: seat.slot, role: Role::Spectator }))
            .into_iter()
            .collect(),
        ModeEvent::PursuitStarted { pursuit, targets, heat }
        | ModeEvent::TargetAdded { pursuit, targets, heat, .. } => {
            vec![pursuit_start(head, *pursuit, targets, *heat, roster)]
        }
        ModeEvent::HeatChanged { level, targets, .. } => targets
            .iter()
            .filter_map(|player| entity(*player))
            .map(|entity_id| Message::HeatChange(HeatChange { head, entity_id, level: *level }))
            .collect(),
        ModeEvent::TargetEnded { pursuit, player, reason } => entity(*player)
            .map(|entity_id| Message::PursuitEnd(PursuitEnd { head, pursuit_id: *pursuit, reason: *reason, entity_id }))
            .into_iter()
            .collect(),
        ModeEvent::PursuitEnded { .. }
        | ModeEvent::KnockedOut { .. }
        | ModeEvent::TrapScored { .. }
        | ModeEvent::TagPassed { .. }
        | ModeEvent::Penalty { .. }
        | ModeEvent::Over { .. } => Vec::new(),
    }
}

fn pursuit_start(head: EventHead, pursuit: u16, targets: &[PlayerId], heat: u8, roster: &Roster) -> Message {
    let mut list = List::empty();
    for entity_id in targets.iter().filter_map(|player| roster.get(*player).map(|seat| seat.entity)) {
        if list.push(entity_id).is_err() {
            break;
        }
    }
    Message::PursuitStart(PursuitStart { head, pursuit_id: pursuit, targets: list, heat })
}
