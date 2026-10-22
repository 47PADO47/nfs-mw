//! Hostile traffic on loopback: thousands of garbage datagrams and a burst of real handshakes from one address, with
//! the LAN limits in force. The established client keeps its round trips, the handshakes beyond the per-address caps
//! go nowhere, and the counters say what was dropped.

use std::net::{SocketAddr, UdpSocket};
use std::thread;
use std::time::{Duration, Instant};

use super::support::{
    MAJOR, PROLOGUE, WAIT, client_config, client_connected, echo, identity, loopback, server_config, server_connected,
};
use crate::MAX_DATAGRAM;
use crate::endpoint::{ClientEndpoint, ClientEvent, EndpointConfig, ServerEndpoint, ServerEvent};
use crate::ratelimit::PreAuthLimits;
use crate::rng::SplitMix64;
use crate::secure::{ClientConfig, ClientStatus, HandshakeClient};

/// Garbage datagrams sent.
const GARBAGE: usize = 20_000;
/// Attacking handshakes started at once from 127.0.0.1.
const ATTACKERS: usize = 30;
/// Round trips the established client makes during the flood.
const ROUND_TRIPS: u8 = 40;

/// Sends `GARBAGE` seeded random datagrams of 1 to 1,200 bytes to `target`.
fn send_garbage(target: SocketAddr) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        let socket = UdpSocket::bind(loopback()).expect("bind");
        let mut rng = SplitMix64(0xF100D);
        let mut buf = vec![0u8; MAX_DATAGRAM];
        for _ in 0..GARBAGE {
            let len = 1 + rng.below(MAX_DATAGRAM as u64) as usize;
            for byte in buf.iter_mut().take(len) {
                *byte = rng.next_u64() as u8;
            }
            // A full socket buffer drops some of these; that is fine.
            let _ = socket.send_to(&buf[..len], target);
        }
    })
}

/// Runs `ATTACKERS` real handshakes against `target` for `for_ms` and returns how many got keys.
fn handshake_flood(target: SocketAddr, for_ms: u64) -> thread::JoinHandle<usize> {
    thread::spawn(move || {
        let mut config = ClientConfig::lan(MAJOR, PROLOGUE);
        config.retry_ms = vec![200; 8];
        let mut attackers: Vec<(UdpSocket, HandshakeClient)> = (0..ATTACKERS)
            .map(|index| {
                let socket = UdpSocket::bind(loopback()).expect("bind");
                socket.set_nonblocking(true).expect("non-blocking");
                let client =
                    HandshakeClient::new(config.clone(), &identity(100 + index as u8), target, 1000 + index as u16)
                        .expect("client");
                (socket, client)
            })
            .collect();
        let start = Instant::now();
        let mut buf = vec![0u8; MAX_DATAGRAM];
        while start.elapsed() < Duration::from_millis(for_ms) {
            let now = start.elapsed().as_millis() as u64;
            for (socket, client) in &mut attackers {
                if let Some(len) = client.poll_transmit(now, &mut buf) {
                    let _ = socket.send_to(&buf[..len], target);
                }
                while let Ok((len, from)) = socket.recv_from(&mut buf) {
                    client.handle(from, &buf[..len]);
                }
            }
            thread::sleep(Duration::from_millis(1));
        }
        attackers
            .iter()
            .filter(|(_, client)| !matches!(client.status(), ClientStatus::InProgress | ClientStatus::Failed(_)))
            .count()
    })
}

#[test]
fn garbage_and_handshake_floods_do_not_disturb_an_established_client() {
    let mut config = server_config();
    config.handshake.limits = PreAuthLimits::LAN;
    config.endpoint = EndpointConfig { stats_ms: 100, ..config.endpoint };
    let mut server = ServerEndpoint::bind(loopback(), None, config).expect("the server binds");
    let target = server.local_addr();
    let mut client = ClientEndpoint::connect(target, client_config(60)).expect("the client starts");
    client_connected(&mut client);
    server_connected(&mut server);

    let garbage = send_garbage(target);
    let flood = handshake_flood(target, 3_000);
    let mut stats = None;
    let mut connected = 0usize;
    let mut next = 0u8;
    let mut in_flight = false;
    let deadline = Instant::now() + WAIT;
    while next < ROUND_TRIPS {
        assert!(Instant::now() < deadline, "the client's round trips stall at {next}");
        for event in echo(&mut server) {
            match event {
                ServerEvent::Stats(latest) => stats = Some(latest),
                ServerEvent::Connected { .. } => connected += 1,
                _ => {}
            }
        }
        if !in_flight {
            client.send_reliable(1, vec![next]);
            in_flight = true;
        }
        while let Some(event) = client.poll_event() {
            match event {
                ClientEvent::Reliable { bytes, .. } => {
                    assert_eq!(bytes, vec![next]);
                    next += 1;
                    in_flight = false;
                }
                ClientEvent::Disconnected(reason) => panic!("the established client was dropped: {reason}"),
                _ => {}
            }
        }
        thread::sleep(Duration::from_millis(5));
    }
    garbage.join().expect("the garbage sender finishes");
    let keyed = flood.join().expect("the flood finishes");

    // The last stats after the flood.
    let settled = Instant::now() + Duration::from_millis(500);
    while Instant::now() < settled {
        for event in echo(&mut server) {
            match event {
                ServerEvent::Stats(latest) => stats = Some(latest),
                ServerEvent::Connected { .. } => connected += 1,
                _ => {}
            }
        }
        thread::sleep(Duration::from_millis(5));
    }
    let stats = stats.expect("stats events arrive");
    let limits = PreAuthLimits::LAN;
    assert!(stats.garbage > 0, "garbage is counted: {stats:?}");
    assert!(stats.handshake.limited > 0, "handshakes over the caps are dropped: {:?}", stats.handshake);
    // The established client spent one of the address's handshakes; the flood gets the rest of the budget at most.
    let budget = limits.handshakes_per_ip as usize - 1;
    assert!(connected <= budget, "at most the per-address handshake budget connects: {connected}");
    assert!(keyed <= budget, "only the per-address handshake budget gets keys: {keyed}");
    assert!(keyed < ATTACKERS && connected < ATTACKERS);
    assert!(stats.peers.iter().any(|peer| peer.bytes_received > 0), "the established client is still served");
}
