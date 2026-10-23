//! The client endpoint: the handle the game thread holds ([`ClientEndpoint`]) and the net thread behind it
//! (`worker.rs` for the loop, `receive.rs` for datagrams).

mod receive;
mod worker;

use std::io;
use std::net::{Ipv4Addr, Ipv6Addr, SocketAddr};

use super::command::ClientCommand;
use super::config::{ClientEndpointConfig, EndpointConfig};
use super::event::ClientEvent;
use super::link::PeerLink;
use super::server::{LOCAL_CLIENT_ID, LOCAL_SERVER_ID};
use super::transport::{Transport, UdpTransport};
use super::worker::Worker;
use crate::conn::{Connection, Plain, Sealer};
use crate::handoff::{LatestReader, RingReceiver, RingSender, latest, ring};
use crate::link::MemoryLink;
use crate::secure::{HandshakeClient, KeyId};
use worker::{ClientWorker, Start};

/// The newest frame of the latest-value frame type ([`ClientEndpointConfig::latest_frame_type`]).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LatestFrame {
    /// 1 for the first frame received, +1 per frame; 0 before any.
    pub seq: u64,
    /// The frame type.
    pub frame_type: u8,
    /// The payload.
    pub bytes: Vec<u8>,
}

/// The game thread's handle on a client's net thread: one connection to one server. Every method returns at once;
/// dropping the handle closes the connection ([`super::reason::NORMAL`]) and joins the thread.
#[derive(Debug)]
pub struct ClientEndpoint {
    events: RingReceiver<ClientEvent>,
    commands: RingSender<ClientCommand>,
    control: RingSender<u8>,
    latest: Option<LatestReader<LatestFrame>>,
    local_addr: SocketAddr,
    server: SocketAddr,
    worker: Worker,
}

impl ClientEndpoint {
    /// Binds a UDP socket on a free port and starts the handshake with the server at `server` (direct connect by
    /// address, or a [`super::Found`] from a discovery scan).
    pub fn connect(server: SocketAddr, config: ClientEndpointConfig) -> io::Result<Self> {
        let bind = match server {
            SocketAddr::V4(_) => SocketAddr::from((Ipv4Addr::UNSPECIFIED, 0)),
            SocketAddr::V6(_) => SocketAddr::from((Ipv6Addr::UNSPECIFIED, 0)),
        };
        Self::spawn(UdpTransport::bind(bind)?, server, config)
    }

    /// Starts the handshake over a transport the caller made (the packet simulator in tests).
    pub fn spawn<T: Transport + 'static>(net: T, server: SocketAddr, config: ClientEndpointConfig) -> io::Result<Self> {
        let mut id = [0u8; 2];
        getrandom::fill(&mut id).map_err(|err| io::Error::other(err.to_string()))?;
        let conn_id = u16::from_le_bytes(id).max(1);
        let handshake = HandshakeClient::new(config.handshake.clone(), &config.identity, server, conn_id)
            .map_err(|err| io::Error::other(err.to_string()))?;
        Self::start(net, server, Start::Handshake(Box::new(handshake)), config.endpoint, config.latest_frame_type)
    }

    /// The host's own client on the in-memory link: a plain connection, already open.
    pub(crate) fn spawn_local(
        link: MemoryLink,
        server: SocketAddr,
        server_key: KeyId,
        config: EndpointConfig,
        latest_frame_type: Option<u8>,
    ) -> io::Result<Self> {
        let sealer: Box<dyn Sealer + Send> = Box::new(Plain);
        let conn = Connection::with_sealer(config.conn(LOCAL_CLIENT_ID, LOCAL_SERVER_ID), sealer);
        let peer = Box::new(PeerLink::new(conn, server, LOCAL_CLIENT_ID, None, false));
        Self::start(link, server, Start::Local { link: peer, server_key }, config, latest_frame_type)
    }

    fn start<T: Transport + 'static>(
        net: T,
        server: SocketAddr,
        start: Start,
        config: EndpointConfig,
        latest_frame_type: Option<u8>,
    ) -> io::Result<Self> {
        let (event_tx, events) = ring(config.event_capacity);
        let (commands, command_rx) = ring(config.command_capacity);
        let (control, control_rx) = ring(config.control_capacity);
        let (writer, reader) = latest(&LatestFrame::default());
        let latest_writer = latest_frame_type.map(|frame_type| (frame_type, writer));
        let local_addr = net.local_addr();
        let state = ClientWorker::new(config, net, server, start, event_tx, command_rx, control_rx, latest_writer);
        let worker = Worker::spawn("net-client", move |stop| state.run(&stop))?;
        let latest = latest_frame_type.map(|_| reader);
        Ok(Self { events, commands, control, latest, local_addr, server, worker })
    }

    /// The local socket's address.
    pub fn local_addr(&self) -> SocketAddr {
        self.local_addr
    }

    /// The server's address.
    pub fn server_addr(&self) -> SocketAddr {
        self.server
    }

    /// The next event, oldest first. Never blocks.
    pub fn poll_event(&mut self) -> Option<ClientEvent> {
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

    /// Commands dropped because the queue was full.
    pub fn dropped_commands(&self) -> u64 {
        self.commands.dropped() + self.control.dropped()
    }

    /// Queues a reliable message on `channel` (0 to 3). Sends wait in the queue until the connection is open.
    pub fn send_reliable(&mut self, channel: u8, bytes: Vec<u8>) {
        self.commands.push(ClientCommand::Reliable { channel, bytes });
    }

    /// Queues an unreliable frame. With a `tag`, [`ClientEvent::Delivered`] or [`ClientEvent::Lost`] reports the
    /// packet that carried it.
    pub fn send_unreliable(&mut self, frame_type: u8, bytes: Vec<u8>, tag: Option<u32>) {
        self.commands.push(ClientCommand::Unreliable { frame_type, bytes, tag });
    }

    /// Closes the connection with `reason`; [`ClientEvent::Disconnected`] follows and the thread ends.
    pub fn disconnect(&mut self, reason: u8) {
        self.control.push(reason);
    }

    /// The newest latest-value frame if one arrived since the last call.
    pub fn take_latest(&mut self) -> Option<&LatestFrame> {
        let reader = self.latest.as_mut()?;
        if !reader.has_update() {
            return None;
        }
        Some(reader.read())
    }

    /// The newest latest-value frame (seq 0 before any), new or not; `None` without a latest-value frame type.
    pub fn latest(&mut self) -> Option<&LatestFrame> {
        Some(self.latest.as_mut()?.read())
    }

    /// Whether the net thread is still running (it ends when the connection does).
    pub fn is_running(&self) -> bool {
        !self.worker.is_finished()
    }

    /// Closes the connection and joins the thread. The events stay readable. Idempotent; dropping does the same.
    pub fn shutdown(&mut self) {
        self.worker.stop();
    }

    /// A token that dies with the net thread.
    #[cfg(test)]
    pub(crate) fn probe(&self) -> std::sync::Weak<()> {
        self.worker.probe()
    }
}
