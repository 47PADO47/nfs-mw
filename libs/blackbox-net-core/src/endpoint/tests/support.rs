//! Shared setup for the endpoint tests: keys, configs with roomy limits and quiet stats, bounded waits that poll the
//! queues, and a transport that can be muted to simulate a peer that went silent.

use std::io;
use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{Duration, Instant};

use crate::endpoint::{
    ClientEndpoint, ClientEndpointConfig, ClientEvent, EndpointConfig, ServerEndpoint, ServerEndpointConfig,
    ServerEvent, SourcePolicy, Transport,
};
use crate::ratelimit::PreAuthLimits;
use crate::secure::{ClientConfig, IdentityKey, ServerConfig};
use crate::session::PeerId;

/// The prologue both sides use.
pub const PROLOGUE: &[u8] = b"test-net/1";
/// The protocol major version.
pub const MAJOR: u8 = 1;
/// The longest any test waits for one thing. Generous: the tests run beside other builds on a slow machine.
pub const WAIT: Duration = Duration::from_secs(20);

/// A fixed identity from one seed byte.
pub fn identity(seed: u8) -> IdentityKey {
    IdentityKey::from_secret(&[seed; 32]).expect("any 32 bytes are a key")
}

/// The default queues and timers without stats events.
pub fn quiet() -> EndpointConfig {
    EndpointConfig { stats_ms: 0, ..EndpointConfig::default() }
}

/// LAN limits loose enough for many clients from one address.
pub fn roomy_limits() -> PreAuthLimits {
    PreAuthLimits {
        conns_per_ip: 64,
        handshakes_per_ip: 64,
        half_open_soft: 64,
        new_handshakes_per_s: 64,
        max_connections: 64,
        ..PreAuthLimits::LAN
    }
}

/// A server with identity 1, roomy limits, quiet stats, and discovery answered on loopback.
pub fn server_config() -> ServerEndpointConfig {
    let mut handshake = ServerConfig::lan(MAJOR, PROLOGUE);
    handshake.limits = roomy_limits();
    ServerEndpointConfig {
        endpoint: quiet(),
        handshake,
        identity: identity(1),
        discovery_sources: SourcePolicy::loopback_only(),
    }
}

/// A client with identity `seed`, quiet stats and patient retries (16 sends 250 ms apart per stage).
pub fn client_config(seed: u8) -> ClientEndpointConfig {
    let mut handshake = ClientConfig::lan(MAJOR, PROLOGUE);
    handshake.retry_ms = vec![250; 16];
    ClientEndpointConfig { endpoint: quiet(), handshake, identity: identity(seed), latest_frame_type: None }
}

/// `127.0.0.1` with a port the system picks.
pub fn loopback() -> SocketAddr {
    SocketAddr::from(([127, 0, 0, 1], 0))
}

/// Polls `f` every millisecond until it gives a value or `timeout` passes.
pub fn wait_for<T>(timeout: Duration, mut f: impl FnMut() -> Option<T>) -> Option<T> {
    let deadline = Instant::now() + timeout;
    loop {
        if let Some(value) = f() {
            return Some(value);
        }
        if Instant::now() >= deadline {
            return None;
        }
        thread::sleep(Duration::from_millis(1));
    }
}

/// The first server event `pick` accepts; the events before it are discarded.
pub fn server_event(server: &mut ServerEndpoint, mut pick: impl FnMut(&ServerEvent) -> bool) -> Option<ServerEvent> {
    wait_for(WAIT, || {
        while let Some(event) = server.poll_event() {
            if pick(&event) {
                return Some(event);
            }
        }
        None
    })
}

/// The first client event `pick` accepts; the events before it are discarded.
pub fn client_event(client: &mut ClientEndpoint, mut pick: impl FnMut(&ClientEvent) -> bool) -> Option<ClientEvent> {
    wait_for(WAIT, || {
        while let Some(event) = client.poll_event() {
            if pick(&event) {
                return Some(event);
            }
        }
        None
    })
}

/// Waits for the client's `Connected`.
pub fn client_connected(client: &mut ClientEndpoint) {
    let event = client_event(client, |event| matches!(event, ClientEvent::Connected { .. }));
    assert!(event.is_some(), "the client connects");
}

/// Waits for the server's next `Connected` and returns the peer.
pub fn server_connected(server: &mut ServerEndpoint) -> PeerId {
    let event = server_event(server, |event| matches!(event, ServerEvent::Connected { .. }));
    let Some(ServerEvent::Connected { peer, .. }) = event else {
        panic!("the server sees a connection");
    };
    peer
}

/// Sends every reliable message and unreliable frame back to its sender; returns the events it did not echo.
pub fn echo(server: &mut ServerEndpoint) -> Vec<ServerEvent> {
    let mut others = Vec::new();
    while let Some(event) = server.poll_event() {
        match event {
            ServerEvent::Reliable { peer, channel, bytes } => server.send_reliable(peer, channel, bytes),
            ServerEvent::Unreliable { peer, frame_type, bytes } => {
                server.send_unreliable(peer, frame_type, bytes, None);
            }
            other => others.push(other),
        }
    }
    others
}

/// A transport whose sends can be switched off: the peer goes silent without closing.
pub struct MuteTransport<T> {
    inner: T,
    mute: Arc<AtomicBool>,
}

impl<T: Transport> MuteTransport<T> {
    /// Wraps `inner`; returns the transport and its switch.
    pub fn new(inner: T) -> (Self, Arc<AtomicBool>) {
        let mute = Arc::new(AtomicBool::new(false));
        (Self { inner, mute: Arc::clone(&mute) }, mute)
    }
}

impl<T: Transport> Transport for MuteTransport<T> {
    fn send_to(&mut self, to: SocketAddr, bytes: &[u8]) -> io::Result<()> {
        if self.mute.load(Ordering::Relaxed) {
            return Ok(());
        }
        self.inner.send_to(to, bytes)
    }

    fn recv_from(&mut self, buf: &mut [u8], wait: Duration) -> io::Result<Option<(SocketAddr, usize)>> {
        self.inner.recv_from(buf, wait)
    }

    fn local_addr(&self) -> SocketAddr {
        self.inner.local_addr()
    }

    fn set_now(&mut self, now_ms: u64) {
        self.inner.set_now(now_ms);
    }
}
