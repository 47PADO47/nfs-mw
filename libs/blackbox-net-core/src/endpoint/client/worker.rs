//! The client's net thread. One pass: the leave command, the sends (once the connection is open), the handshake's
//! due datagram, the datagrams from the server, the connection's sends, then the idle timeout and stats.
//!
//! The thread ends when the connection does (leave, DISCONNECT from the server, idle timeout, failed handshake, full
//! reliable queue) after reporting [`ClientEvent::Disconnected`].

use std::net::SocketAddr;
use std::time::Duration;

use super::super::clock::Clock;
use super::super::command::ClientCommand;
use super::super::config::EndpointConfig;
use super::super::event::{ClientEvent, DisconnectReason};
use super::super::link::{LinkEvent, PeerLink};
use super::super::reason;
use super::super::stats::EndpointStats;
use super::super::transport::Transport;
use super::super::wire::Wire;
use super::super::worker::Stop;
use super::LatestFrame;
use crate::MAX_DATAGRAM;
use crate::conn::ConnError;
use crate::handoff::{LatestWriter, RingReceiver, RingSender};
use crate::secure::{ClientStatus, HandshakeClient, KeyId};
use crate::session::PeerId;

/// How the connection starts.
pub(crate) enum Start {
    /// A Noise handshake over the network.
    Handshake(Box<HandshakeClient>),
    /// The host's own client: already open.
    Local { link: Box<PeerLink>, server_key: KeyId },
}

/// The latest-value frame type and the writing half of its cell.
pub(super) type LatestSlot = Option<(u8, LatestWriter<LatestFrame>)>;

/// Everything the client's net thread owns.
pub(crate) struct ClientWorker<T: Transport> {
    pub(super) config: EndpointConfig,
    pub(super) wire: Wire<T>,
    pub(super) server: SocketAddr,
    pub(super) handshake: Option<HandshakeClient>,
    pub(super) link: Option<PeerLink>,
    pub(super) connected: bool,
    pub(super) server_key: Option<KeyId>,
    /// The Noise handshake hash of this connection (all zero for the in-memory link).
    pub(super) handshake_hash: [u8; 32],
    pub(super) events: RingSender<ClientEvent>,
    commands: RingReceiver<ClientCommand>,
    control: RingReceiver<u8>,
    pub(super) latest: LatestSlot,
    pub(super) clock: Clock,
    pub(super) counters: EndpointStats,
    pub(super) out: Vec<u8>,
    next_stats_ms: u64,
}

impl<T: Transport> ClientWorker<T> {
    /// The thread's state, before it starts.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        config: EndpointConfig,
        net: T,
        server: SocketAddr,
        start: Start,
        events: RingSender<ClientEvent>,
        commands: RingReceiver<ClientCommand>,
        control: RingReceiver<u8>,
        latest: LatestSlot,
    ) -> Self {
        let (handshake, link, server_key) = match start {
            Start::Handshake(handshake) => (Some(*handshake), None, None),
            Start::Local { link, server_key } => (None, Some(*link), Some(server_key)),
        };
        Self {
            config,
            wire: Wire::new(net, None),
            server,
            handshake,
            link,
            connected: false,
            server_key,
            handshake_hash: [0; 32],
            events,
            commands,
            control,
            latest,
            clock: Clock::new(),
            counters: EndpointStats::default(),
            out: vec![0u8; MAX_DATAGRAM],
            next_stats_ms: config.stats_ms,
        }
    }

    /// Runs until the connection ends or the handle asks it to stop.
    pub(crate) fn run(mut self, stop: &Stop) {
        let mut buf = vec![0u8; MAX_DATAGRAM + 1];
        let mut idle = false;
        if self.link.is_some() {
            self.open();
        }
        while !stop.requested() {
            let now = self.clock.now_ms();
            self.wire.net.set_now(now);
            let wait = Duration::from_millis(self.config.poll_ms.max(1) * u64::from(idle));
            match self.step(now, &mut buf, wait) {
                Ok(work) => idle = !work,
                Err(reason) => {
                    self.events.push(ClientEvent::Disconnected(reason));
                    return;
                }
            }
        }
        let now = self.clock.now_ms();
        self.close(now, reason::NORMAL);
        self.events.push(ClientEvent::Disconnected(DisconnectReason::Shutdown));
    }

    /// The connection is confirmed: sends may flow and the game hears about it.
    pub(super) fn open(&mut self) {
        self.connected = true;
        if let Some(handshake) = self.handshake.as_mut() {
            handshake.confirm();
        }
        self.handshake = None;
        let Some(server_key) = self.server_key else {
            return;
        };
        self.events.push(ClientEvent::Connected { server_key, handshake_hash: self.handshake_hash });
    }

    /// Sends DISCONNECT with `byte` if the connection is open, and forgets it.
    pub(super) fn close(&mut self, now_ms: u64, byte: u8) {
        let Some(mut link) = self.link.take() else {
            return;
        };
        if !self.connected {
            return;
        }
        let (wire, server) = (&mut self.wire, self.server);
        link.close_now(now_ms, byte, &mut self.out, |bytes| wire.send(false, server, bytes));
    }

    fn step(&mut self, now_ms: u64, buf: &mut [u8], wait: Duration) -> Result<bool, DisconnectReason> {
        if let Some(byte) = self.control.pop() {
            self.close(now_ms, byte);
            return Err(DisconnectReason::Local(byte));
        }
        self.apply_commands(now_ms)?;
        let mut work = self.transmit_handshake(now_ms)?;
        work |= self.receive(buf, wait)?;
        let now_ms = self.clock.now_ms();
        work |= self.flush(now_ms);
        self.timers(now_ms)?;
        Ok(work)
    }

    /// Takes the waiting sends once the connection is open; until then they wait in the queue.
    fn apply_commands(&mut self, now_ms: u64) -> Result<(), DisconnectReason> {
        if !self.connected {
            return Ok(());
        }
        for _ in 0..self.commands.capacity() {
            let Some(link) = self.link.as_mut() else {
                return Ok(());
            };
            let Some(command) = self.commands.pop() else {
                return Ok(());
            };
            let sent = match command {
                ClientCommand::Reliable { channel, bytes } => link.send_reliable(channel, &bytes),
                ClientCommand::Unreliable { frame_type, bytes, tag } => {
                    match link.queue_unreliable(frame_type, bytes, tag) {
                        true => Ok(()),
                        false => Err(ConnError::FrameTooLarge),
                    }
                }
            };
            match sent {
                Ok(()) => {}
                Err(ConnError::QueueFull) => {
                    self.close(now_ms, reason::PROTOCOL);
                    return Err(DisconnectReason::Overflow);
                }
                Err(_) => self.counters.bad_commands += 1,
            }
        }
        Ok(())
    }

    /// Sends the handshake datagram that is due; a failed handshake ends the thread.
    fn transmit_handshake(&mut self, now_ms: u64) -> Result<bool, DisconnectReason> {
        let Some(handshake) = self.handshake.as_mut() else {
            return Ok(false);
        };
        let sent = handshake.poll_transmit(now_ms, &mut self.out);
        if let ClientStatus::Failed(err) = handshake.status() {
            return Err(DisconnectReason::Handshake(err));
        }
        let Some(len) = sent else {
            return Ok(false);
        };
        if let Some(bytes) = self.out.get(..len) {
            self.wire.send(false, self.server, bytes);
        }
        Ok(true)
    }

    /// Sends what the open connection has due.
    fn flush(&mut self, now_ms: u64) -> bool {
        if !self.connected {
            return false;
        }
        let Some(link) = self.link.as_mut() else {
            return false;
        };
        let (wire, server) = (&mut self.wire, self.server);
        link.flush(now_ms, &mut self.out, &mut self.counters.bad_commands, |bytes| wire.send(false, server, bytes))
    }

    /// Reports the connection's messages and events; a DISCONNECT from the server ends the connection.
    pub(super) fn drain(&mut self) -> Result<(), DisconnectReason> {
        let Some(link) = self.link.as_mut() else {
            return Ok(());
        };
        while let Some((channel, bytes)) = link.take_message() {
            self.events.push(ClientEvent::Reliable { channel, bytes });
        }
        let mut closed = None;
        let events = &mut self.events;
        link.drain_events(|event| match event {
            LinkEvent::Delivered(tag) => {
                events.push(ClientEvent::Delivered(tag));
            }
            LinkEvent::Lost(tag) => {
                events.push(ClientEvent::Lost(tag));
            }
            LinkEvent::Closed(byte) => closed = Some(byte),
        });
        let Some(byte) = closed else {
            return Ok(());
        };
        self.link = None;
        Err(DisconnectReason::Remote(byte))
    }

    /// Events from the sends, the idle timeout and the stats event.
    fn timers(&mut self, now_ms: u64) -> Result<(), DisconnectReason> {
        self.drain()?;
        let timed_out = self.connected && self.link.as_ref().is_some_and(|link| link.is_timed_out(now_ms));
        if timed_out {
            self.close(now_ms, reason::TIMEOUT);
            return Err(DisconnectReason::Timeout);
        }
        if self.config.stats_ms == 0 || now_ms < self.next_stats_ms {
            return Ok(());
        }
        self.next_stats_ms = now_ms + self.config.stats_ms;
        let mut stats = self.counters.clone();
        stats.now_ms = now_ms;
        stats.datagrams_out = self.wire.stats.datagrams;
        stats.bytes_out = self.wire.stats.bytes;
        stats.send_errors += self.wire.stats.errors;
        stats.events_dropped = self.events.dropped();
        stats.peers = self.link.iter().map(|link| link.stats(PeerId(0))).collect();
        self.events.push(ClientEvent::Stats(Box::new(stats)));
        Ok(())
    }
}
