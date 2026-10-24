//! Counters of an endpoint, sent to the game as [`super::ServerEvent::Stats`] and [`super::ClientEvent::Stats`].

use crate::secure::ServerStats;
use crate::session::PeerId;

/// One connection's numbers.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PeerStats {
    /// The peer (`PeerId(0)` for a client's server).
    pub peer: PeerId,
    /// Smoothed round-trip time in milliseconds.
    pub rtt_ms: f64,
    /// Loss average between 0 and 1.
    pub loss: f64,
    /// Bytes of packets sent.
    pub bytes_sent: u64,
    /// Bytes of accepted packets received.
    pub bytes_received: u64,
    /// Reliable messages not yet fully acknowledged.
    pub pending_reliable: usize,
}

/// Everything an endpoint counts. Every drop has a counter; none of them is answered on the wire.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct EndpointStats {
    /// The endpoint's clock when the stats were taken.
    pub now_ms: u64,
    /// Datagrams received, of any kind.
    pub datagrams_in: u64,
    /// Bytes received, of any kind.
    pub bytes_in: u64,
    /// Datagrams sent.
    pub datagrams_out: u64,
    /// Bytes sent.
    pub bytes_out: u64,
    /// Datagrams that are neither handshake nor data packets, or longer than one datagram.
    pub garbage: u64,
    /// Data packets for no known connection.
    pub unknown_connection: u64,
    /// Data packets for a connection, from another address, dropped before opening: over the rate for such packets or
    /// the peer's budget, or for the in-memory link. The others are opened (counted in `rejected` if they do not open).
    pub wrong_address: u64,
    /// Path challenges sent to an address a peer's packets came from.
    pub path_challenges: u64,
    /// Peers that moved to a new address after it answered a challenge.
    pub path_migrations: u64,
    /// Data packets over the peer's inbound budget, dropped before opening.
    pub over_budget: u64,
    /// Data packets the connection refused: bad tag, duplicate, too old, malformed.
    pub rejected: u64,
    /// Sends the socket refused.
    pub send_errors: u64,
    /// Receive errors from the socket.
    pub recv_errors: u64,
    /// Sends the game asked for that could not be made: a reserved or oversized frame, a bad channel, a message too
    /// large.
    pub bad_commands: u64,
    /// Events dropped because the game did not read them in time.
    pub events_dropped: u64,
    /// Discovery replies sent.
    pub discovery_replies: u64,
    /// Discovery datagrams not answered: malformed, not opted in, source refused, or over the rate.
    pub discovery_refused: u64,
    /// The handshake server's counters (zero on a client).
    pub handshake: ServerStats,
    /// One entry per open connection.
    pub peers: Vec<PeerStats>,
}
