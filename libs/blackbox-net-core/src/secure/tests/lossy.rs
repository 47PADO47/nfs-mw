//! The whole path over the seeded lossy link: cookie round trip, Noise handshake, key confirmation and a reliable
//! message each way, under loss, latency, jitter, duplication and reordering, for many seeds. The listen host's own
//! client, on the plain in-memory link with no handshake, runs beside it.

use std::net::SocketAddr;

use super::harness::{MAJOR, PROLOGUE, addr, identity, server};
use crate::MAX_DATAGRAM;
use crate::conn::{ConnConfig, Connection};
use crate::link::{Link, LossConfig, LossyLink, MemoryLink, MemoryNet};
use crate::secure::{ClientConfig, ClientStatus, HandshakeClient, NoiseSealer, PacketKind, ServerAction, classify};

const STEP_MS: u64 = 10;
const LIMIT_MS: u64 = 20_000;

struct Run {
    client_done_ms: Option<u64>,
    server_got: bool,
    client_got: bool,
    amplification_ok: bool,
}

fn lossy(net: &MemoryNet, at: SocketAddr, seed: u64, loss: f64) -> LossyLink<MemoryLink> {
    let config = LossConfig { loss, latency_ms: 20, jitter_ms: 15, duplicate: 0.1, reorder: 0.1, seed };
    LossyLink::new(net.endpoint(at), config)
}

fn run(seed: u64, loss: f64) -> Run {
    let net = MemoryNet::new();
    let (server_addr, client_addr) = (addr(9000), addr(4000));
    let mut server_link = lossy(&net, server_addr, seed, loss);
    let mut client_link = lossy(&net, client_addr, seed ^ 0x5EED, loss);
    let mut srv = server(0);
    let mut config = ClientConfig::lan(MAJOR, PROLOGUE);
    config.retry_ms = vec![200; 40];
    let mut cli = HandshakeClient::new(config, &identity(2), server_addr, 31).expect("client");

    let mut client_conn: Option<Connection<NoiseSealer>> = None;
    let mut server_conn: Option<Connection<NoiseSealer>> = None;
    let mut run = Run { client_done_ms: None, server_got: false, client_got: false, amplification_ok: true };
    let (mut to_client, mut from_client) = (0usize, 0usize);
    let mut buf = vec![0u8; MAX_DATAGRAM];
    let mut out = vec![0u8; MAX_DATAGRAM];
    let mut msg = Vec::new();

    let mut now = 0;
    while now < LIMIT_MS {
        server_link.set_now(now);
        client_link.set_now(now);

        if let Some(len) = cli.poll_transmit(now, &mut out) {
            client_link.send(server_addr, &out[..len]).expect("send");
        }
        if let Some(conn) = client_conn.as_mut() {
            let sent = conn.write_packet(now, &[], &mut out).expect("client packet");
            client_link.send(server_addr, &out[..sent.len]).expect("send");
        }

        while let Some((from, len)) = server_link.recv(&mut buf).expect("recv") {
            let datagram = &mut buf[..len];
            if classify(datagram) == PacketKind::Data {
                let Some(conn) = server_conn.as_mut() else { continue };
                let _ = conn.read_packet(now, datagram, |_, _| {});
                continue;
            }
            if server_conn.is_none() {
                from_client += len;
            }
            match srv.handle(now, from, datagram, &mut out) {
                ServerAction::Reply(n) => {
                    to_client += n;
                    run.amplification_ok &= to_client <= 3 * from_client;
                    server_link.send(from, &out[..n]).expect("send");
                }
                ServerAction::Established(end) => {
                    let (mut conn, _) = end.into_connection(ConnConfig::default());
                    conn.send_message(1, b"from the server").expect("queued");
                    server_conn = Some(conn);
                }
                ServerAction::None => {}
            }
        }
        srv.expire(now);
        if let Some(conn) = server_conn.as_mut() {
            let sent = conn.write_packet(now, &[], &mut out).expect("server packet");
            server_link.send(client_addr, &out[..sent.len]).expect("send");
            run.server_got |= conn.recv_message(&mut msg).is_some_and(|_| msg == b"from the client");
        }

        while let Some((from, len)) = client_link.recv(&mut buf).expect("recv") {
            let datagram = &mut buf[..len];
            if classify(datagram) != PacketKind::Data {
                cli.handle(from, datagram);
                continue;
            }
            let Some(conn) = client_conn.as_mut() else { continue };
            if conn.read_packet(now, datagram, |_, _| {}).is_ok() {
                cli.confirm();
            }
        }
        if let Some(end) = cli.take_established() {
            let (mut conn, _) = end.into_connection(ConnConfig::default());
            conn.send_message(0, b"from the client").expect("queued");
            client_conn = Some(conn);
        }
        if let Some(conn) = client_conn.as_mut() {
            run.client_got |= conn.recv_message(&mut msg).is_some_and(|_| msg == b"from the server");
        }
        if cli.status() == ClientStatus::Confirmed && run.client_done_ms.is_none() {
            run.client_done_ms = Some(now);
        }
        if run.client_done_ms.is_some() && run.server_got && run.client_got {
            return run;
        }
        assert!(!matches!(cli.status(), ClientStatus::Failed(_)), "seed {seed}: client failed: {:?}", cli.status());
        now += STEP_MS;
    }
    run
}

#[test]
fn the_handshake_completes_over_a_lossy_reordering_duplicating_link() {
    for seed in 0..40u64 {
        let run = run(seed, 0.2);
        assert!(run.client_done_ms.is_some(), "seed {seed}: the keys were never confirmed");
        assert!(run.server_got && run.client_got, "seed {seed}: a message did not arrive");
        assert!(run.amplification_ok, "seed {seed}: the server sent more than 3x before the handshake ended");
    }
}

#[test]
fn the_handshake_completes_over_a_clean_link_in_a_few_round_trips() {
    let run = run(7, 0.0);
    let done = run.client_done_ms.expect("confirmed");
    assert!(done <= 500, "about three round trips of 40 to 70 ms each, took {done} ms");
}

#[test]
fn the_listen_hosts_own_client_needs_no_handshake() {
    let net = MemoryNet::new();
    let (host_addr, own_addr) = (addr(9100), addr(9101));
    let mut host_link = net.endpoint(host_addr);
    let mut own_link = net.endpoint(own_addr);
    let mut host = Connection::new(ConnConfig { local_id: 1, remote_id: 2, ..ConnConfig::default() });
    let mut own = Connection::new(ConnConfig { local_id: 2, remote_id: 1, ..ConnConfig::default() });
    own.send_message(0, b"join").expect("queued");
    host.send_message(1, b"welcome").expect("queued");
    let mut buf = vec![0u8; MAX_DATAGRAM];
    let mut msg = Vec::new();
    for now in [0u64, 16] {
        let sent = own.write_packet(now, &[], &mut buf).expect("plain packet");
        own_link.send(host_addr, &buf[..sent.len]).expect("send");
        let sent = host.write_packet(now, &[], &mut buf).expect("plain packet");
        host_link.send(own_addr, &buf[..sent.len]).expect("send");
        while let Some((_, len)) = host_link.recv(&mut buf).expect("recv") {
            host.read_packet(now, &mut buf[..len], |_, _| {}).expect("host reads");
        }
        while let Some((_, len)) = own_link.recv(&mut buf).expect("recv") {
            own.read_packet(now, &mut buf[..len], |_, _| {}).expect("own client reads");
        }
    }
    assert_eq!(host.recv_message(&mut msg), Some(0));
    assert_eq!(msg, b"join");
    assert_eq!(own.recv_message(&mut msg), Some(1));
    assert_eq!(msg, b"welcome");
}
