//! Endpoint settings: the queues and timers shared by both sides, and what each side adds.

use super::discovery::SourcePolicy;
use crate::conn::ConnConfig;
use crate::secure::{ClientConfig, IdentityKey, ServerConfig};

/// Queues and timers of one endpoint. The defaults follow protocol.md section 2 and security-ops.md section 2.1.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EndpointConfig {
    /// Events the game has not read yet; beyond this the oldest is dropped and counted.
    pub event_capacity: usize,
    /// Data commands (sends) the net thread has not taken yet; beyond this the oldest is dropped and counted.
    pub command_capacity: usize,
    /// Control commands (disconnect, budgets, discovery reply, local attach), queued apart so data cannot push them out.
    pub control_capacity: usize,
    /// Longest wait on the socket when there is nothing to do, in milliseconds (at least 1).
    pub poll_ms: u64,
    /// Silence after which a connection is dropped.
    pub idle_timeout_ms: u64,
    /// Silence after which an acknowledgement-only packet is sent.
    pub keepalive_ms: u64,
    /// The period of the per-peer datagram budget ("datagrams decoded per tick").
    pub tick_ms: u64,
    /// A [`super::EndpointStats`] event every this many milliseconds; 0 for none.
    pub stats_ms: u64,
    /// Datagrams taken from the socket in one pass before timers, commands and sends get their turn, so a flood
    /// cannot starve the established peers.
    pub max_datagrams_per_pass: usize,
}

impl Default for EndpointConfig {
    fn default() -> Self {
        Self {
            event_capacity: 4096,
            command_capacity: 4096,
            control_capacity: 256,
            poll_ms: 1,
            idle_timeout_ms: 10_000,
            keepalive_ms: 2_000,
            tick_ms: 16,
            stats_ms: 1_000,
            max_datagrams_per_pass: 256,
        }
    }
}

impl EndpointConfig {
    /// The connection timers with the given connection ids.
    pub(crate) fn conn(&self, local_id: u16, remote_id: u16) -> ConnConfig {
        ConnConfig { local_id, remote_id, idle_timeout_ms: self.idle_timeout_ms, keepalive_ms: self.keepalive_ms }
    }
}

/// A server endpoint: queues and timers, the handshake (with its pre-authentication limits) and the identity.
#[derive(Debug, Clone)]
pub struct ServerEndpointConfig {
    /// Queues and timers.
    pub endpoint: EndpointConfig,
    /// The handshake: protocol major, prologue, accepted patterns, the pre-authentication caps, rekeying.
    pub handshake: ServerConfig,
    /// The server's long-term key.
    pub identity: IdentityKey,
    /// Which sources the discovery responder answers.
    pub discovery_sources: SourcePolicy,
}

impl ServerEndpointConfig {
    /// A LAN server with the default queues, answering discovery from private LAN ranges only.
    pub fn lan(handshake: ServerConfig, identity: IdentityKey) -> Self {
        Self {
            endpoint: EndpointConfig::default(),
            handshake,
            identity,
            discovery_sources: SourcePolicy::private_lan(),
        }
    }
}

/// A client endpoint: queues and timers, the handshake and the identity.
#[derive(Debug, Clone)]
pub struct ClientEndpointConfig {
    /// Queues and timers.
    pub endpoint: EndpointConfig,
    /// The handshake: protocol major, prologue, pattern, the pinned server key, retries.
    pub handshake: ClientConfig,
    /// The player's long-term key.
    pub identity: IdentityKey,
    /// Unreliable frames of this type go to the latest-value cell ([`super::ClientEndpoint::take_latest`]) instead of
    /// the event queue. Only for frames the game may skip: a delta snapshot whose baseline the transport acknowledged
    /// must be decoded, so leave this `None` when baselines follow packet acknowledgements.
    pub latest_frame_type: Option<u8>,
}

impl ClientEndpointConfig {
    /// A client with the default queues and no latest-value cell.
    pub fn new(handshake: ClientConfig, identity: IdentityKey) -> Self {
        Self { endpoint: EndpointConfig::default(), handshake, identity, latest_frame_type: None }
    }
}
