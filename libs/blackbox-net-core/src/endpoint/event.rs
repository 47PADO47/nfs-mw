//! What the net thread tells the game thread.

use std::fmt;
use std::net::SocketAddr;

use super::stats::EndpointStats;
use crate::secure::{HandshakeError, KeyId};
use crate::session::PeerId;

/// Why a connection ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisconnectReason {
    /// The other side sent a DISCONNECT with this reason byte (see [`super::reason`]).
    Remote(u8),
    /// This side closed it with this reason byte (a kick on the server, a leave on the client).
    Local(u8),
    /// No valid packet for the idle timeout.
    Timeout,
    /// The peer stopped acknowledging and a reliable channel filled up (security-ops.md 2.1); closed with
    /// [`super::reason::PROTOCOL`].
    Overflow,
    /// This endpoint shut down.
    Shutdown,
    /// The handshake failed (client only).
    Handshake(HandshakeError),
}

impl fmt::Display for DisconnectReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Remote(reason) => write!(f, "closed by the other side (reason {reason})"),
            Self::Local(reason) => write!(f, "closed here (reason {reason})"),
            Self::Timeout => f.write_str("timed out"),
            Self::Overflow => f.write_str("the other side stopped acknowledging"),
            Self::Shutdown => f.write_str("shut down"),
            Self::Handshake(err) => write!(f, "handshake failed: {err}"),
        }
    }
}

/// An event of a [`super::ServerEndpoint`], in the order the net thread saw it.
#[derive(Debug, Clone, PartialEq)]
pub enum ServerEvent {
    /// A peer finished the handshake (or the host's own client attached, with `local` set and the key the caller
    /// passed). The key id is the peer's identity; trust on first use and bans are the game's.
    Connected {
        /// The peer.
        peer: PeerId,
        /// The peer's identity key id.
        key: KeyId,
        /// The address the handshake ran with (an in-process address for a local peer).
        addr: SocketAddr,
        /// The host's own client on the in-memory link.
        local: bool,
        /// The Noise handshake hash the room password is bound to; all zero for a local peer.
        handshake_hash: [u8; 32],
    },
    /// A peer's packets now come from `addr`: it answered a path challenge there (a NAT rebinding or a roaming device).
    /// The connection sends to the new address from now on.
    PathChanged {
        /// The peer.
        peer: PeerId,
        /// The address it was proved at.
        addr: SocketAddr,
    },
    /// A peer is gone; commands naming it are ignored from now on.
    Disconnected {
        /// The peer.
        peer: PeerId,
        /// Why.
        reason: DisconnectReason,
    },
    /// A complete reliable message.
    Reliable {
        /// The sender.
        peer: PeerId,
        /// The channel, 0 to 3.
        channel: u8,
        /// The message.
        bytes: Vec<u8>,
    },
    /// An unreliable frame.
    Unreliable {
        /// The sender.
        peer: PeerId,
        /// The frame type.
        frame_type: u8,
        /// The payload.
        bytes: Vec<u8>,
    },
    /// The packet that carried an unreliable frame sent with this tag was acknowledged.
    Delivered {
        /// The receiver.
        peer: PeerId,
        /// The tag given to the send.
        tag: u32,
    },
    /// The packet that carried an unreliable frame sent with this tag was written off.
    Lost {
        /// The receiver.
        peer: PeerId,
        /// The tag given to the send.
        tag: u32,
    },
    /// Counters, every [`super::EndpointConfig::stats_ms`].
    Stats(Box<EndpointStats>),
}

/// An event of a [`super::ClientEndpoint`], in the order the net thread saw it.
#[derive(Debug, Clone, PartialEq)]
pub enum ClientEvent {
    /// The handshake finished and the server's first sealed packet arrived (at once for the host's own client).
    Connected {
        /// The server's identity key id.
        server_key: KeyId,
        /// The Noise handshake hash the room password is bound to; all zero on the host's in-memory link.
        handshake_hash: [u8; 32],
    },
    /// The connection is over; the net thread has stopped.
    Disconnected(DisconnectReason),
    /// A complete reliable message.
    Reliable {
        /// The channel, 0 to 3.
        channel: u8,
        /// The message.
        bytes: Vec<u8>,
    },
    /// An unreliable frame (other than the latest-value frame type).
    Unreliable {
        /// The frame type.
        frame_type: u8,
        /// The payload.
        bytes: Vec<u8>,
    },
    /// The packet that carried an unreliable frame sent with this tag was acknowledged.
    Delivered(u32),
    /// The packet that carried an unreliable frame sent with this tag was written off.
    Lost(u32),
    /// Counters, every [`super::EndpointConfig::stats_ms`].
    Stats(Box<EndpointStats>),
}
