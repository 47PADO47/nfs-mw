//! The net thread (integration.md section 1), behind the `netcode` feature. Each endpoint runs on **one** thread of its
//! own that owns its socket, the handshake, the sealed [`crate::conn::Connection`]s, the timers and the rate limits. The
//! game thread talks to it only through the bounded queues of [`crate::handoff`]: it never blocks, and when a queue is
//! full the oldest entry is dropped and counted.
//!
//! - [`ServerEndpoint`]: accepts handshakes ([`crate::secure::HandshakeServer`] with its pre-authentication gate),
//!   keeps one connection per peer, enforces each peer's inbound budget ([`crate::ratelimit::PeerBudget`]), answers LAN
//!   discovery on a second port, and reports [`ServerEvent`]s. [`ServerEndpoint::connect_local`] attaches the listen
//!   host's own client through an in-memory link ([`crate::link::MemoryNet`]): no socket, no handshake (ADR 0009
//!   section 3), and the same events and calls as a remote peer.
//! - [`ClientEndpoint`]: one connection to one server; [`ClientEvent`]s, and a latest-value cell
//!   ([`ClientEndpoint::take_latest`]) for the frame type the caller marks as the snapshot.
//! - [`DiscoveryScan`]: sends discovery requests (broadcast or to typed addresses) and collects the replies on its own
//!   thread. [`DiscoveryResponder`] and [`SourcePolicy`] are the server's side, sans-IO.
//! - [`Transport`]: the seam under all three. [`UdpTransport`] is a blocking `std` socket with a short read timeout;
//!   the in-memory and lossy links of [`crate::link`] implement it too, so every endpoint runs over the seeded packet
//!   simulator in tests.
//!
//! Messages are opaque bytes here: reliable messages on channels 0 to 3, unreliable frames with a frame type the game
//! picks (any type except `0x00` and `0x10` to `0x13`, which the connection owns). An unreliable frame may carry a `tag`
//! (a snapshot tick, say); the endpoint reports [`ServerEvent::Delivered`] or [`ServerEvent::Lost`] with that tag when
//! the packet that carried it is acknowledged or written off, which is what delta baselines need.
//!
//! **Files:** `transport.rs`, `clock.rs`, `worker.rs` (the thread handle that stops and joins on drop), `config.rs`,
//! `event.rs`, `command.rs`, `stats.rs`, `link.rs` (one connection with its queues, shared by both sides), `wire.rs`
//! (the sockets of one endpoint), `server/`, `client/`, `discovery/`. Tests live in `tests/`.

mod client;
mod clock;
mod command;
mod config;
mod discovery;
mod event;
mod link;
mod path;
mod server;
mod stats;
#[cfg(test)]
mod tests;
mod transport;
mod wire;
mod worker;

pub use client::{ClientEndpoint, LatestFrame};
pub use command::PeerLimits;
pub use config::{ClientEndpointConfig, EndpointConfig, ServerEndpointConfig};
pub use discovery::{DiscoveryResponder, DiscoveryScan, Found, ScanConfig, SourcePolicy, Subnet};
pub use event::{ClientEvent, DisconnectReason, ServerEvent};
pub use server::ServerEndpoint;
pub use stats::{EndpointStats, PeerStats};
pub use transport::{Transport, UdpTransport};

/// DISCONNECT reason bytes (protocol.md section 5.2).
pub mod reason {
    /// A normal close.
    pub const NORMAL: u8 = 0;
    /// The peer was silent too long.
    pub const TIMEOUT: u8 = 1;
    /// The server removed the peer.
    pub const KICKED: u8 = 2;
    /// The peer broke the protocol (it stopped acknowledging and its reliable queue filled).
    pub const PROTOCOL: u8 = 3;
    /// The server is shutting down.
    pub const SHUTTING_DOWN: u8 = 4;
}
