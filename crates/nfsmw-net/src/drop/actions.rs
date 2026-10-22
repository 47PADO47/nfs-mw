//! What the session does when a player drops, when a held slot's window ends, and when a dropped player comes back.

use crate::chat::notice;
use crate::message::{LeaveReason, SystemNotice};

/// Why a player is leaving.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DropCause {
    /// A `Disconnect` from the client: a clean leave, no resume.
    Quit,
    /// Silence for [`super::TIMEOUT_MS`]: the slot may be held for a resume.
    Timeout,
    /// Removed by the host, an admin or a vote. The lobby sends the kick notice.
    Kicked,
    /// Kicked with a ban. The lobby sends the kick notice.
    Banned,
}

impl DropCause {
    /// The reason `PlayerLeft` and the leave notice carry.
    pub fn leave_reason(self) -> LeaveReason {
        match self {
            Self::Quit => LeaveReason::Quit,
            Self::Timeout => LeaveReason::Timeout,
            Self::Kicked => LeaveReason::Kicked,
            Self::Banned => LeaveReason::Banned,
        }
    }
}

/// What the session knows about the player who dropped.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DropContext {
    /// The player's car is in the world.
    pub has_car: bool,
    /// The player is a racer of the running race and has not finished.
    pub racing: bool,
    /// The player is a target of a pursuit.
    pub in_pursuit: bool,
    /// The slot holds a resume token, so a timeout keeps it for [`super::RESUME_WINDOW_MS`].
    pub resumable: bool,
}

/// One thing the server does because of a drop, a window's end or a resume, in the order given.
#[derive(Debug, Clone, PartialEq)]
pub enum DropAction {
    /// Park the car (neutral input, brakes on) as a non-colliding ghost: other cars and traffic pass through it.
    GhostCar { slot: u8 },
    /// Take the car out of the world (`EntityRemoved` with reason `Left`).
    RemoveCar { slot: u8 },
    /// Take the player out of its pursuit. The pursuit rules end the pursuit when it has no target left, and police
    /// do not chase a parked ghost.
    LeavePursuit { slot: u8 },
    /// Score the player as not finished in the running race; the race goes on for the others.
    DidNotFinish { slot: u8 },
    /// Keep the slot, the lobby row (flagged away) and the car until `until_ms`.
    HoldSlot { slot: u8, until_ms: u64 },
    /// Free the slot: remove the lobby member (which sends `PlayerLeft` with `reason`), forget the slot's chat bucket,
    /// vote and presence.
    ReleaseSlot { slot: u8, reason: LeaveReason },
    /// Give a held car back to its player: collisions on again after the 3 s late-join ghost (sync-model.md 4.1).
    RestoreCar { slot: u8 },
    /// Send the slot's next snapshot in full, with no baseline, and the lobby state.
    FullSnapshot { slot: u8 },
    /// Send this notice to every member.
    Notice(SystemNotice),
}

/// The actions for `slot` dropping at `now_ms` for `cause`.
///
/// A timeout of a resumable slot parks the car as a ghost, takes the player out of its pursuit and holds the slot; the
/// race result waits for the window. Any other drop removes the car, scores a racer as not finished and frees the slot.
/// Quits and timeouts print a leave notice; kicks and bans leave their notice to the lobby.
pub fn on_drop(slot: u8, cause: DropCause, context: &DropContext, now_ms: u64) -> Vec<DropAction> {
    let mut actions = Vec::new();
    if context.in_pursuit {
        actions.push(DropAction::LeavePursuit { slot });
    }
    if matches!(cause, DropCause::Quit | DropCause::Timeout) {
        actions.push(DropAction::Notice(notice::left(slot, cause.leave_reason())));
    }
    if cause == DropCause::Timeout && context.resumable {
        if context.has_car {
            actions.push(DropAction::GhostCar { slot });
        }
        let until_ms = now_ms.saturating_add(super::RESUME_WINDOW_MS);
        actions.push(DropAction::HoldSlot { slot, until_ms });
        return actions;
    }
    actions.extend(release(slot, cause.leave_reason(), context));
    actions
}

/// The actions when `slot`'s resume window ends: as a drop without resume, with no second notice.
pub fn on_expiry(slot: u8, context: &DropContext) -> Vec<DropAction> {
    release(slot, LeaveReason::Timeout, context)
}

/// The actions when `slot` resumes: the car back, a full snapshot, and a join notice.
pub fn on_resume(slot: u8, has_car: bool) -> Vec<DropAction> {
    let mut actions = Vec::new();
    if has_car {
        actions.push(DropAction::RestoreCar { slot });
    }
    actions.push(DropAction::FullSnapshot { slot });
    actions.push(DropAction::Notice(notice::joined(slot)));
    actions
}

fn release(slot: u8, reason: LeaveReason, context: &DropContext) -> Vec<DropAction> {
    let mut actions = Vec::new();
    if context.has_car {
        actions.push(DropAction::RemoveCar { slot });
    }
    if context.racing {
        actions.push(DropAction::DidNotFinish { slot });
    }
    actions.push(DropAction::ReleaseSlot { slot, reason });
    actions
}
