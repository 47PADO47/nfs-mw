//! Peer identifiers and per-peer states.

/// A peer's identifier, chosen by the caller. Peers are kept in id order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PeerId(pub u16);

/// The state of one peer within a session.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PeerState {
    /// In the lobby.
    Lobby,
    /// Loading the track.
    Loading,
    /// Finished loading; waiting for the others.
    Loaded,
    /// Taking part in the race.
    InGame,
}
