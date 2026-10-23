//! System notices (protocol-messages.md 7.7): a code and up to four numbers that each client turns into a line in its
//! own language. The argument layouts below are the contract with the client.

use crate::message::{LeaveReason, List, NoticeCode, SystemNotice, VoteKind};

/// `Join`: `[slot]`.
pub fn joined(slot: u8) -> SystemNotice {
    notice(NoticeCode::Join, &[u32::from(slot)])
}

/// `Leave`: `[slot, reason]`, the reason a `LeaveReason` code.
pub fn left(slot: u8, reason: LeaveReason) -> SystemNotice {
    notice(NoticeCode::Leave, &[u32::from(slot), u32::from(reason.code())])
}

/// `Kick`: `[slot, ban_minutes]`, 0 minutes for a kick without a ban, `u32::MAX` for a ban to the end of the session.
pub fn kicked(slot: u8, ban_minutes: Option<u32>) -> SystemNotice {
    let minutes = match ban_minutes {
        None => 0,
        Some(0) => u32::MAX,
        Some(minutes) => minutes,
    };
    notice(NoticeCode::Kick, &[u32::from(slot), minutes])
}

/// `Mute`: `[slot, muted]`, `muted` 1 or 0.
pub fn muted(slot: u8, on: bool) -> SystemNotice {
    notice(NoticeCode::Mute, &[u32::from(slot), u32::from(on)])
}

/// What a vote notice reports.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum VoteNotice {
    Started = 0,
    Passed = 1,
    Failed = 2,
    Cancelled = 3,
}

/// `Vote`: `[kind, option, what]`, `what` a [`VoteNotice`].
pub fn vote(kind: VoteKind, option: u32, what: VoteNotice) -> SystemNotice {
    notice(NoticeCode::Vote, &[u32::from(kind.code()), option, what as u32])
}

/// `Result`: `[slot, place, time_ms]`; place 0 for did not finish.
pub fn result(slot: u8, place: u8, time_ms: u32) -> SystemNotice {
    notice(NoticeCode::Result, &[u32::from(slot), u32::from(place), time_ms])
}

fn notice(code: NoticeCode, args: &[u32]) -> SystemNotice {
    // Every builder passes at most four arguments, so the list always fits.
    let args = List::new(args.to_vec()).unwrap_or_default();
    SystemNotice { code, args }
}
