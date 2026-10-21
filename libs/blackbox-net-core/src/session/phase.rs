//! The session phases.

/// The phase of a session. `Countdown` carries the server tick at which the race starts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    /// Peers gather. Joins are accepted.
    Lobby,
    /// The track loads. Peers report `peer_loaded`.
    Loading,
    /// The host has loaded and waits for every peer, or for the load deadline.
    WaitLoads,
    /// The intro plays. Only entered when the config's `intro_ms` is above zero.
    Intro,
    /// The countdown runs until the server tick reaches `start_tick`.
    Countdown {
        /// Server tick at which the race starts.
        start_tick: u32,
    },
    /// The race is on. Joins in progress are accepted.
    Running,
    /// The race is over.
    Results,
}
