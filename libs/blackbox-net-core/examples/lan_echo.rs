//! A manual soak of the net thread: one server and N clients in one process on loopback, each with a real Noise
//! handshake and sealed packets. Every client streams unreliable frames and reliable messages; the server echoes
//! everything; the example prints the echo rate once a second and the totals at the end.
//!
//! Run with `cargo run --release -p blackbox-net-core --features netcode --example lan_echo -- [clients] [seconds]`
//! (defaults: 8 clients, 10 seconds).

use std::net::SocketAddr;
use std::thread;
use std::time::{Duration, Instant};

use blackbox_net_core::endpoint::{
    ClientEndpoint, ClientEndpointConfig, ClientEvent, EndpointConfig, ServerEndpoint, ServerEndpointConfig,
    ServerEvent, SourcePolicy,
};
use blackbox_net_core::ratelimit::PreAuthLimits;
use blackbox_net_core::secure::{ClientConfig, IdentityKey, ServerConfig};

const PROLOGUE: &[u8] = b"lan-echo/1";
const MAJOR: u8 = 1;
/// Unreliable frame type used by the example.
const FRAME: u8 = 0x01;
/// Bytes of each unreliable frame and each reliable message.
const FRAME_BYTES: usize = 200;
const MESSAGE_BYTES: usize = 1_000;

#[derive(Debug, Default, Clone, Copy)]
struct Totals {
    frames: u64,
    messages: u64,
    bytes: u64,
}

fn main() -> std::io::Result<()> {
    let mut args = std::env::args().skip(1);
    let clients: usize = args.next().and_then(|arg| arg.parse().ok()).unwrap_or(8);
    let seconds: u64 = args.next().and_then(|arg| arg.parse().ok()).unwrap_or(10);
    let identity =
        |seed: u8| IdentityKey::from_secret(&[seed; 32]).map_err(|err| std::io::Error::other(err.to_string()));

    let limits = PreAuthLimits {
        conns_per_ip: 1024,
        handshakes_per_ip: 1024,
        half_open_soft: 1024,
        half_open_total: 1024,
        new_handshakes_per_s: 1024,
        max_connections: 1024,
        ..PreAuthLimits::LAN
    };
    let mut handshake = ServerConfig::lan(MAJOR, PROLOGUE);
    handshake.limits = limits;
    let config = ServerEndpointConfig {
        endpoint: EndpointConfig::default(),
        handshake,
        identity: identity(1)?,
        discovery_sources: SourcePolicy::loopback_only(),
    };
    let mut server = ServerEndpoint::bind(SocketAddr::from(([127, 0, 0, 1], 0)), None, config)?;
    let addr = server.local_addr();
    println!("server on {addr}, {clients} clients, {seconds} s");

    let mut endpoints = Vec::with_capacity(clients);
    for index in 0..clients {
        let mut handshake = ClientConfig::lan(MAJOR, PROLOGUE);
        handshake.retry_ms = vec![250; 16];
        let config = ClientEndpointConfig::new(handshake, identity(2 + (index % 250) as u8)?);
        endpoints.push(ClientEndpoint::connect(addr, config)?);
    }

    let start = Instant::now();
    let mut connected = 0usize;
    let mut totals = Totals::default();
    let mut last = Totals::default();
    let mut next_report = Duration::from_secs(1);
    let frame = vec![0xA5u8; FRAME_BYTES];
    let message = vec![0x5Au8; MESSAGE_BYTES];
    let mut pass = 0u64;
    while start.elapsed() < Duration::from_secs(seconds) {
        pass += 1;
        while let Some(event) = server.poll_event() {
            match event {
                ServerEvent::Reliable { peer, channel, bytes } => server.send_reliable(peer, channel, bytes),
                ServerEvent::Unreliable { peer, frame_type, bytes } => {
                    server.send_unreliable(peer, frame_type, bytes, None);
                }
                ServerEvent::Disconnected { peer, reason } => println!("server: {peer:?} left: {reason}"),
                _ => {}
            }
        }
        for client in &mut endpoints {
            client.send_unreliable(FRAME, frame.clone(), None);
            if pass % 10 == 0 {
                client.send_reliable(1, message.clone());
            }
            while let Some(event) = client.poll_event() {
                match event {
                    ClientEvent::Connected { .. } => connected += 1,
                    ClientEvent::Unreliable { bytes, .. } => {
                        totals.frames += 1;
                        totals.bytes += bytes.len() as u64;
                    }
                    ClientEvent::Reliable { bytes, .. } => {
                        totals.messages += 1;
                        totals.bytes += bytes.len() as u64;
                    }
                    ClientEvent::Disconnected(reason) => println!("client: disconnected: {reason}"),
                    _ => {}
                }
            }
        }
        if start.elapsed() >= next_report {
            next_report += Duration::from_secs(1);
            println!(
                "{:>3} s: {connected}/{clients} connected, {} frames/s, {} messages/s, {:.1} KiB/s echoed",
                next_report.as_secs() - 1,
                totals.frames - last.frames,
                totals.messages - last.messages,
                (totals.bytes - last.bytes) as f64 / 1024.0,
            );
            last = totals;
        }
        thread::sleep(Duration::from_millis(1));
    }
    let elapsed = start.elapsed().as_secs_f64();
    println!(
        "total: {} frames, {} messages, {:.1} MiB echoed in {elapsed:.1} s ({:.1} KiB/s); dropped events {} (server), commands {}",
        totals.frames,
        totals.messages,
        totals.bytes as f64 / (1024.0 * 1024.0),
        totals.bytes as f64 / 1024.0 / elapsed,
        server.dropped_events(),
        server.dropped_commands(),
    );
    Ok(())
}
