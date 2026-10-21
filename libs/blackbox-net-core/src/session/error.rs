//! Refused session calls.

use std::fmt;

use super::Phase;

/// Why a session call was refused. A refused call changes nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionError {
    /// The call does not fit the current phase.
    WrongPhase {
        /// The phase the session is in.
        phase: Phase,
    },
    /// A peer with this id is already in the session.
    Duplicate,
    /// The session does not accept a join in this phase.
    Closed,
    /// No peer with this id is in the session.
    UnknownPeer,
}

impl fmt::Display for SessionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::WrongPhase { phase } => write!(f, "call does not fit the session phase {phase:?}"),
            Self::Duplicate => f.write_str("a peer with this id is already in the session"),
            Self::Closed => f.write_str("the session does not accept joins in this phase"),
            Self::UnknownPeer => f.write_str("no peer with this id is in the session"),
        }
    }
}

impl std::error::Error for SessionError {}
