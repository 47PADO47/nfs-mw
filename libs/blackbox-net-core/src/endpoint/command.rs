//! What the game thread asks of the net thread. Built by the handle methods; the variants stay inside the crate.

use std::net::SocketAddr;

use crate::discovery::Reply;
use crate::ratelimit::{PeerBudget, PreAuthLimits};
use crate::secure::KeyId;
use crate::session::PeerId;

/// The inbound budget of one peer (security-ops.md 2.1). Datagrams over it are dropped before they are opened.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PeerLimits {
    /// Inbound bytes per second.
    pub bytes_per_s: u32,
    /// Datagrams decoded per [`super::EndpointConfig::tick_ms`].
    pub datagrams_per_tick: u32,
}

impl PeerLimits {
    /// The budget the pre-authentication limits give every new peer.
    pub fn from_limits(limits: &PreAuthLimits) -> Self {
        Self { bytes_per_s: limits.inbound_bytes_per_s, datagrams_per_tick: limits.datagrams_per_tick }
    }

    /// The rate-limit state for these numbers.
    pub(crate) fn budget(&self) -> PeerBudget {
        let limits = PreAuthLimits {
            inbound_bytes_per_s: self.bytes_per_s,
            datagrams_per_tick: self.datagrams_per_tick,
            ..PreAuthLimits::LAN
        };
        PeerBudget::new(&limits)
    }
}

/// A send to one peer or to all of them.
#[derive(Debug)]
pub(crate) enum ServerCommand {
    /// A reliable message to `peer` on `channel`.
    Reliable { peer: PeerId, channel: u8, bytes: Vec<u8> },
    /// An unreliable frame to `peer`, reported back by `tag` when it has one.
    Unreliable { peer: PeerId, frame_type: u8, bytes: Vec<u8>, tag: Option<u32> },
    /// A reliable message to every peer.
    BroadcastReliable { channel: u8, bytes: Vec<u8> },
    /// An unreliable frame to every peer.
    BroadcastUnreliable { frame_type: u8, bytes: Vec<u8> },
}

/// Rare commands, queued apart from the sends.
#[derive(Debug)]
pub(crate) enum ServerControl {
    /// Close `peer` with `reason`.
    Disconnect { peer: PeerId, reason: u8 },
    /// Replace `peer`'s inbound budget.
    SetBudget { peer: PeerId, limits: PeerLimits },
    /// Answer discovery with this reply (its nonce is replaced per request), or stop answering.
    DiscoveryReply(Option<Reply>),
    /// The host's own client on the in-memory link at `addr`.
    AttachLocal { addr: SocketAddr, key: KeyId },
}

/// A client's sends.
#[derive(Debug)]
pub(crate) enum ClientCommand {
    /// A reliable message on `channel`.
    Reliable { channel: u8, bytes: Vec<u8> },
    /// An unreliable frame, reported back by `tag` when it has one.
    Unreliable { frame_type: u8, bytes: Vec<u8>, tag: Option<u32> },
}
