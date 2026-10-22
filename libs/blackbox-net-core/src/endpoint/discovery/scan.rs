//! The client's discovery scan (security-ops.md 2.3, rule 4): a few requests, each with a fresh random nonce, to the
//! broadcast address and any typed addresses, and the replies collected on a thread of its own so the game thread
//! never waits. A reply counts only if it echoes one of the nonces sent; each source is reported once. A reply is a
//! hint: the server's key is proved by the handshake.

use std::collections::HashSet;
use std::io;
use std::net::{Ipv4Addr, SocketAddr};
use std::time::{Duration, Instant};

use super::super::transport::{Transport, UdpTransport};
use super::super::worker::{Stop, Worker};
use crate::MAX_DATAGRAM;
use crate::discovery::{Reply, Request};
use crate::handoff::{RingReceiver, RingSender, ring};

/// Longest wait on the socket between two checks of the clock.
const SCAN_POLL: Duration = Duration::from_millis(5);

/// What to scan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScanConfig {
    /// Where requests go: the broadcast address and the discovery port, and typed or recent servers.
    pub targets: Vec<SocketAddr>,
    /// The protocol major version asked for; replies with another are ignored.
    pub proto_major: u8,
    /// How long the scan listens, in milliseconds.
    pub duration_ms: u64,
    /// Requests sent, spread evenly over the duration (each with a new nonce).
    pub requests: u32,
    /// Replies kept for the game before the oldest is dropped.
    pub capacity: usize,
}

impl ScanConfig {
    /// The design's scan: 3 requests in 2 seconds to `255.255.255.255` on `discovery_port`.
    pub fn broadcast(discovery_port: u16, proto_major: u8) -> Self {
        Self {
            targets: vec![SocketAddr::from((Ipv4Addr::BROADCAST, discovery_port))],
            proto_major,
            duration_ms: 2_000,
            requests: 3,
            capacity: 64,
        }
    }
}

/// A server that answered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Found {
    /// Where the reply came from (the server's discovery port).
    pub from: SocketAddr,
    /// The game port at the same address: what [`super::super::ClientEndpoint::connect`] takes.
    pub game_addr: SocketAddr,
    /// The reply.
    pub reply: Reply,
}

/// A running scan. Dropping it stops the thread.
#[derive(Debug)]
pub struct DiscoveryScan {
    results: RingReceiver<Found>,
    worker: Worker,
}

impl DiscoveryScan {
    /// Binds a broadcast-capable UDP socket on a free port and starts the scan.
    pub fn start(config: ScanConfig) -> io::Result<Self> {
        let transport = UdpTransport::bind_broadcast(SocketAddr::from((Ipv4Addr::UNSPECIFIED, 0)))?;
        Self::spawn(transport, config)
    }

    /// Starts the scan over a transport the caller made.
    pub fn spawn<T: Transport + 'static>(transport: T, config: ScanConfig) -> io::Result<Self> {
        let (tx, results) = ring(config.capacity);
        let worker = Worker::spawn("net-discovery", move |stop| scan(transport, &config, tx, &stop))?;
        Ok(Self { results, worker })
    }

    /// The next server found. Never blocks.
    pub fn poll(&mut self) -> Option<Found> {
        self.results.pop()
    }

    /// Whether the scan is over (its duration passed).
    pub fn is_finished(&self) -> bool {
        self.worker.is_finished()
    }
}

/// The scan thread.
fn scan<T: Transport>(mut transport: T, config: &ScanConfig, mut results: RingSender<Found>, stop: &Stop) {
    let start = Instant::now();
    let duration = Duration::from_millis(config.duration_ms);
    let gap = duration / config.requests.max(1);
    let mut nonces: Vec<u64> = Vec::new();
    let mut sent = 0u32;
    let mut seen: HashSet<SocketAddr> = HashSet::new();
    let mut buf = vec![0u8; MAX_DATAGRAM];
    while !stop.requested() && start.elapsed() < duration {
        if sent < config.requests && start.elapsed() >= gap * sent {
            sent += 1;
            send_request(&mut transport, config, &mut nonces);
        }
        let Ok(Some((from, len))) = transport.recv_from(&mut buf, SCAN_POLL) else {
            continue;
        };
        let Some(datagram) = buf.get(..len) else {
            continue;
        };
        let Ok(reply) = Reply::decode(datagram) else {
            continue;
        };
        let answers = nonces.iter().any(|&nonce| reply.answers(&Request { proto_major: config.proto_major, nonce }));
        if !answers || !seen.insert(from) {
            continue;
        }
        let game_addr = SocketAddr::new(from.ip(), reply.game_port);
        results.push(Found { from, game_addr, reply });
    }
}

/// Sends one request with a new nonce to every target. Without randomness the request is skipped.
fn send_request<T: Transport>(transport: &mut T, config: &ScanConfig, nonces: &mut Vec<u64>) {
    let mut bytes = [0u8; 8];
    if getrandom::fill(&mut bytes).is_err() {
        return;
    }
    let nonce = u64::from_le_bytes(bytes);
    nonces.push(nonce);
    let request = Request { proto_major: config.proto_major, nonce }.encode();
    for &target in &config.targets {
        // A target that cannot be reached is just not found.
        let _ = transport.send_to(target, &request);
    }
}
