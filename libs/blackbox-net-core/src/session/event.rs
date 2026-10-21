//! Notifications queued by a session.

use super::{PeerId, Phase};

/// A notification queued by a [`super::Session`] and read with `poll_event`, oldest first.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionEvent {
    /// The session entered this phase.
    Phase(Phase),
    /// A peer had not loaded at the deadline and was removed.
    PeerDropped(PeerId),
    /// A peer left and was removed.
    PeerLeft(PeerId),
}
