//! Where a session is in its life, and why it ended.

use blackbox_net_core::endpoint::DisconnectReason;

use crate::message::{JoinRejected, Kick};

/// The session's state.
#[derive(Debug, Clone, PartialEq)]
pub enum Status {
    /// Waiting for the handshake to finish.
    Connecting,
    /// Connected; `Auth` and `Join` are sent and the answer (and perhaps a password exchange) is pending.
    Admitting,
    /// Seated: in the lobby or in the world.
    Joined,
    /// The connection dropped; the session tries to resume its slot until `deadline_ms`.
    Reconnecting { since_ms: u64, deadline_ms: u64 },
    /// Over for good; no more traffic.
    Ended(EndReason),
}

impl Status {
    /// Whether the session has a seat now.
    pub fn is_joined(&self) -> bool {
        matches!(self, Self::Joined)
    }

    /// Whether the session is over.
    pub fn is_ended(&self) -> bool {
        matches!(self, Self::Ended(_))
    }
}

/// Why a session ended.
#[derive(Debug, Clone, PartialEq)]
pub enum EndReason {
    /// The player left ([`super::ClientSession::leave`]).
    Left,
    /// The server refused the seat.
    Rejected(JoinRejected),
    /// The server removed the player.
    Kicked(Kick),
    /// The connection ended before a seat was given, or for a reason that holds no slot.
    Link(DisconnectReason),
    /// The connection dropped and no resume worked inside the window.
    ResumeExpired,
    /// The room has a password and none was given.
    PasswordRequired,
    /// The password exchange failed: a wrong password, or a server that could not prove it knows it.
    PasswordRejected,
    /// The server's entity schema differs from this build's: snapshots could not be read.
    SchemaMismatch,
    /// The server broke the protocol: too many invalid messages, or an answer out of turn.
    Protocol,
}
