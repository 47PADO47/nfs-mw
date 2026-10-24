//! The server endpoint: the handle the game thread holds ([`ServerEndpoint`]) and the net thread behind it
//! (`worker.rs` for the loop, timers and sends, `receive.rs` for datagrams, `control.rs` for commands).

mod control;
mod receive;
mod worker;

use std::io;
use std::net::SocketAddr;

use super::client::ClientEndpoint;
use super::clock::Clock;
use super::command::{PeerLimits, ServerCommand, ServerControl};
use super::config::{EndpointConfig, ServerEndpointConfig};
use super::event::ServerEvent;
use super::transport::{Transport, UdpTransport};
use super::worker::Worker;
use crate::discovery::Reply;
use crate::handoff::{RingReceiver, RingSender, ring};
use crate::link::MemoryNet;
use crate::secure::{HandshakeServer, KeyId};
use crate::session::PeerId;
use worker::{ServerWorker, Shared};

/// The server's address on the in-memory network of the host's own client. Nothing outside the process can reach it.
pub(crate) const LOCAL_SERVER_ADDR: SocketAddr =
    SocketAddr::new(std::net::IpAddr::V4(std::net::Ipv4Addr::UNSPECIFIED), 1);
/// The connection id the server answers to on the in-memory link.
pub(crate) const LOCAL_SERVER_ID: u16 = 1;
/// The connection id the host's own client answers to on the in-memory link.
pub(crate) const LOCAL_CLIENT_ID: u16 = 2;

/// The game thread's handle on a server's net thread. Every method returns at once; dropping the handle shuts the
/// thread down (closing every connection with [`super::reason::SHUTTING_DOWN`]) and joins it.
#[derive(Debug)]
pub struct ServerEndpoint {
    events: RingReceiver<ServerEvent>,
    commands: RingSender<ServerCommand>,
    control: RingSender<ServerControl>,
    local_net: MemoryNet,
    next_local_port: u16,
    key: KeyId,
    local_addr: SocketAddr,
    discovery_addr: Option<SocketAddr>,
    worker: Worker,
}

impl ServerEndpoint {
    /// Binds the game port at `addr` and, when given, the discovery port at `discovery`, and starts the net thread.
    pub fn bind(addr: SocketAddr, discovery: Option<SocketAddr>, config: ServerEndpointConfig) -> io::Result<Self> {
        let net = UdpTransport::bind(addr)?;
        let discovery = match discovery {
            Some(addr) => Some(Box::new(UdpTransport::bind_shared(addr)?) as Box<dyn Transport>),
            None => None,
        };
        Self::spawn(net, discovery, config)
    }

    /// Starts the net thread over transports the caller made (the packet simulator in tests).
    pub fn spawn<T: Transport + 'static>(
        net: T,
        discovery: Option<Box<dyn Transport>>,
        config: ServerEndpointConfig,
    ) -> io::Result<Self> {
        let clock = Clock::new();
        let handshake = HandshakeServer::new(config.handshake.clone(), config.identity.clone(), clock.now_ms())
            .map_err(|err| io::Error::other(err.to_string()))?;
        let endpoint = config.endpoint;
        let (event_tx, events) = ring(endpoint.event_capacity);
        let (commands, command_rx) = ring(endpoint.command_capacity);
        let (control, control_rx) = ring(endpoint.control_capacity);
        let local_net = MemoryNet::new();
        let local = local_net.endpoint(LOCAL_SERVER_ADDR);
        let local_addr = net.local_addr();
        let discovery_addr = discovery.as_ref().map(|transport| transport.local_addr());
        let shared = Shared {
            config: endpoint,
            limits: PeerLimits::from_limits(&config.handshake.limits),
            events: event_tx,
            commands: command_rx,
            control: control_rx,
        };
        let state = ServerWorker::new(shared, net, local, discovery, handshake, config.discovery_sources, clock);
        let worker = Worker::spawn("net-server", move |stop| state.run(&stop))?;
        let key = config.identity.key_id();
        Ok(Self { events, commands, control, local_net, next_local_port: 2, key, local_addr, discovery_addr, worker })
    }

    /// The game port's address (with the port the system picked for port 0).
    pub fn local_addr(&self) -> SocketAddr {
        self.local_addr
    }

    /// The discovery port's address, when one is bound.
    pub fn discovery_addr(&self) -> Option<SocketAddr> {
        self.discovery_addr
    }

    /// The server's identity key id.
    pub fn key_id(&self) -> KeyId {
        self.key
    }

    /// The next event, oldest first. Never blocks.
    pub fn poll_event(&mut self) -> Option<ServerEvent> {
        self.events.pop()
    }

    /// Events waiting.
    pub fn pending_events(&self) -> usize {
        self.events.len()
    }

    /// Events dropped because the queue was full.
    pub fn dropped_events(&self) -> u64 {
        self.events.dropped()
    }

    /// Commands dropped because the queue was full (sends and control together).
    pub fn dropped_commands(&self) -> u64 {
        self.commands.dropped() + self.control.dropped()
    }

    /// Queues a reliable message to `peer` on `channel` (0 to 3).
    pub fn send_reliable(&mut self, peer: PeerId, channel: u8, bytes: Vec<u8>) {
        self.commands.push(ServerCommand::Reliable { peer, channel, bytes });
    }

    /// Queues an unreliable frame to `peer`. With a `tag`, [`ServerEvent::Delivered`] or [`ServerEvent::Lost`] reports
    /// the packet that carried it.
    pub fn send_unreliable(&mut self, peer: PeerId, frame_type: u8, bytes: Vec<u8>, tag: Option<u32>) {
        self.commands.push(ServerCommand::Unreliable { peer, frame_type, bytes, tag });
    }

    /// Queues a reliable message to every connected peer.
    pub fn broadcast_reliable(&mut self, channel: u8, bytes: Vec<u8>) {
        self.commands.push(ServerCommand::BroadcastReliable { channel, bytes });
    }

    /// Queues an unreliable frame to every connected peer.
    pub fn broadcast_unreliable(&mut self, frame_type: u8, bytes: Vec<u8>) {
        self.commands.push(ServerCommand::BroadcastUnreliable { frame_type, bytes });
    }

    /// Closes `peer` with `reason` (a kick is [`super::reason::KICKED`]). A [`ServerEvent::Disconnected`] follows.
    pub fn disconnect(&mut self, peer: PeerId, reason: u8) {
        self.control.push(ServerControl::Disconnect { peer, reason });
    }

    /// Replaces `peer`'s inbound budget.
    pub fn set_budget(&mut self, peer: PeerId, limits: PeerLimits) {
        self.control.push(ServerControl::SetBudget { peer, limits });
    }

    /// Starts answering LAN discovery with `reply` (its nonce is replaced by each request's), or stops with `None`
    /// (security-ops.md 2.3, rule 4: opt-in). A new server answers nothing until this is called.
    pub fn set_discovery_reply(&mut self, reply: Option<Reply>) {
        self.control.push(ServerControl::DiscoveryReply(reply));
    }

    /// Attaches the listen host's own client through the in-memory link: no socket, no handshake. The server reports
    /// it as an ordinary peer ([`ServerEvent::Connected`] with `local` set and `player_key`), and the returned client
    /// behaves like a remote one. `latest_frame_type` is the client's latest-value frame type.
    pub fn connect_local(
        &mut self,
        player_key: KeyId,
        config: EndpointConfig,
        latest_frame_type: Option<u8>,
    ) -> io::Result<ClientEndpoint> {
        let port = self.next_local_port;
        self.next_local_port = self.next_local_port.checked_add(1).ok_or(io::ErrorKind::AddrInUse)?;
        let addr = SocketAddr::new(LOCAL_SERVER_ADDR.ip(), port);
        let link = self.local_net.endpoint(addr);
        self.control.push(ServerControl::AttachLocal { addr, key: player_key });
        ClientEndpoint::spawn_local(link, LOCAL_SERVER_ADDR, self.key, config, latest_frame_type)
    }

    /// Whether the net thread is still running.
    pub fn is_running(&self) -> bool {
        !self.worker.is_finished()
    }

    /// Stops the net thread: every connection is closed with [`super::reason::SHUTTING_DOWN`] and reported as
    /// [`super::DisconnectReason::Shutdown`]. The events stay readable. Idempotent; dropping the handle does the same.
    pub fn shutdown(&mut self) {
        self.worker.stop();
    }

    /// A token that dies with the net thread.
    #[cfg(test)]
    pub(crate) fn probe(&self) -> std::sync::Weak<()> {
        self.worker.probe()
    }
}
