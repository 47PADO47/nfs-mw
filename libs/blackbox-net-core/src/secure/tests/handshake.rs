//! The handshake by direct calls: success for `XX` and `XK`, every refusal, the cookie rules, replays, timeouts and caps.

use std::net::SocketAddr;

use super::harness::{MAJOR, PROLOGUE, addr, client, client_to_server, direct_handshake, identity, server};
use crate::MAX_DATAGRAM;
use crate::conn::ConnConfig;
use crate::secure::packet::{Challenge, HelloCookie};
use crate::secure::{
    ClientConfig, ClientStatus, HandshakeClient, HandshakeError, HandshakeServer, Pattern, ServerAction, ServerConfig,
};

const SERVER: u16 = 9000;
const CLIENT: u16 = 4000;

#[test]
fn an_xx_handshake_authenticates_both_keys() {
    let mut srv = server(0);
    let mut cli = client(addr(SERVER), 77);
    let (client_end, server_end) = direct_handshake(&mut cli, &mut srv, addr(CLIENT), 0);
    assert_eq!(client_end.info.remote_static, *identity(1).public(), "the client learned the server's key");
    assert_eq!(server_end.info.remote_static, *identity(2).public(), "the server learned the client's key");
    assert_eq!(client_end.info.handshake_hash, server_end.info.handshake_hash, "one handshake, one hash");
    assert_eq!(client_end.info.local_conn_id, 77);
    assert_eq!(server_end.info.remote_conn_id, 77);
    assert_eq!(client_end.info.remote_conn_id, server_end.info.local_conn_id);
    assert_eq!(server_end.info.addr, addr(CLIENT));
    assert_eq!(client_end.info.remote_id(), identity(1).key_id());
    assert_eq!(srv.half_open(), 0);
    assert_eq!(srv.stats().established, 1);
}

#[test]
fn the_keys_carry_reliable_messages_both_ways() {
    let mut srv = server(0);
    let mut cli = client(addr(SERVER), 5);
    let (client_end, server_end) = direct_handshake(&mut cli, &mut srv, addr(CLIENT), 0);
    let (mut c, _) = client_end.into_connection(ConnConfig::default());
    let (mut s, _) = server_end.into_connection(ConnConfig::default());
    c.send_message(0, b"join please").expect("queued");
    s.send_message(1, b"welcome").expect("queued");
    let mut buf = vec![0u8; MAX_DATAGRAM];
    let sent = c.write_packet(10, &[], &mut buf).expect("sealed");
    s.read_packet(10, &mut buf[..sent.len], |_, _| {}).expect("opened by the server");
    let sent = s.write_packet(20, &[], &mut buf).expect("sealed");
    c.read_packet(20, &mut buf[..sent.len], |_, _| {}).expect("opened by the client");
    cli.confirm();
    assert_eq!(cli.status(), ClientStatus::Confirmed);
    let mut msg = Vec::new();
    assert_eq!(s.recv_message(&mut msg), Some(0));
    assert_eq!(msg, b"join please");
    assert_eq!(c.recv_message(&mut msg), Some(1));
    assert_eq!(msg, b"welcome");
}

#[test]
fn an_xk_handshake_pins_the_server_key() {
    let mut config = ServerConfig::lan(MAJOR, PROLOGUE);
    config.accept_xk = true;
    let mut srv = HandshakeServer::with_cookie_secret(config.clone(), identity(1), [3; 32], 0);
    let mut client_config = ClientConfig::lan(MAJOR, PROLOGUE);
    client_config.pattern = Pattern::Xk;
    client_config.server_key = Some(*identity(1).public());
    let mut cli = HandshakeClient::new(client_config.clone(), &identity(2), addr(SERVER), 9).expect("client");
    let (client_end, server_end) = direct_handshake(&mut cli, &mut srv, addr(CLIENT), 0);
    assert_eq!(client_end.info.remote_static, *identity(1).public());
    assert_eq!(server_end.info.remote_static, *identity(2).public());

    // The same client pinned to another key: the server cannot read message 1 and stays silent.
    client_config.server_key = Some(*identity(5).public());
    let mut wrong = HandshakeClient::new(client_config, &identity(2), addr(SERVER), 10).expect("client");
    let (_, challenge) = client_to_server(&mut wrong, &mut srv, addr(CLIENT), 0);
    wrong.handle(addr(SERVER), &challenge);
    let (action, _) = client_to_server(&mut wrong, &mut srv, addr(CLIENT), 0);
    assert!(matches!(action, ServerAction::None));
    assert_eq!(srv.stats().noise_failures, 1);
    assert_eq!(srv.half_open(), 0);
}

#[test]
fn an_xk_client_needs_a_server_key_and_an_xx_server_refuses_xk() {
    let mut config = ClientConfig::lan(MAJOR, PROLOGUE);
    config.pattern = Pattern::Xk;
    assert_eq!(HandshakeClient::new(config.clone(), &identity(2), addr(SERVER), 1).err(), Some(HandshakeError::Config));
    config.server_key = Some(*identity(1).public());
    let mut cli = HandshakeClient::new(config, &identity(2), addr(SERVER), 1).expect("client");
    let mut srv = server(0);
    let (_, challenge) = client_to_server(&mut cli, &mut srv, addr(CLIENT), 0);
    cli.handle(addr(SERVER), &challenge);
    let (action, _) = client_to_server(&mut cli, &mut srv, addr(CLIENT), 0);
    assert!(matches!(action, ServerAction::None), "a LAN server takes XX only");
    assert_eq!(srv.half_open(), 0);
}

#[test]
fn a_prologue_mismatch_fails_before_any_data() {
    let mut srv = server(0);
    let config = ClientConfig::lan(MAJOR, b"test-proto/2");
    let mut cli = HandshakeClient::new(config, &identity(2), addr(SERVER), 1).expect("client");
    let (_, challenge) = client_to_server(&mut cli, &mut srv, addr(CLIENT), 0);
    cli.handle(addr(SERVER), &challenge);
    let (_, msg2) = client_to_server(&mut cli, &mut srv, addr(CLIENT), 0);
    assert_eq!(cli.handle(addr(SERVER), &msg2), ClientStatus::Failed(HandshakeError::Noise));
    assert!(cli.take_established().is_none());
}

#[test]
fn hello_costs_the_server_no_state_and_no_amplification() {
    let mut srv = server(0);
    let mut cli = client(addr(SERVER), 1);
    let (action, challenge) = client_to_server(&mut cli, &mut srv, addr(CLIENT), 0);
    assert!(matches!(action, ServerAction::Reply(36)));
    assert!(challenge.len() <= 512 / 3);
    assert_eq!(srv.half_open(), 0, "no slot before the cookie comes back");
    assert_eq!(srv.stats().challenges, 1);
}

/// Runs Hello and Challenge and returns the client's HelloCookie bytes.
fn hello_cookie(cli: &mut HandshakeClient, srv: &mut HandshakeServer, from: SocketAddr, now: u64) -> Vec<u8> {
    let (_, challenge) = client_to_server(cli, srv, from, now);
    cli.handle(addr(SERVER), &challenge);
    let mut buf = vec![0u8; MAX_DATAGRAM];
    let len = cli.poll_transmit(now, &mut buf).expect("HelloCookie");
    buf.truncate(len);
    buf
}

fn handle(srv: &mut HandshakeServer, now: u64, from: SocketAddr, datagram: &[u8]) -> ServerAction {
    let mut out = vec![0u8; MAX_DATAGRAM];
    srv.handle(now, from, datagram, &mut out)
}

#[test]
fn a_tampered_cookie_or_another_address_is_dropped_silently() {
    let mut srv = server(0);
    let mut cli = client(addr(SERVER), 1);
    let packet = hello_cookie(&mut cli, &mut srv, addr(CLIENT), 0);
    let mut tampered = packet.clone();
    tampered[30] ^= 1;
    assert!(matches!(handle(&mut srv, 0, addr(CLIENT), &tampered), ServerAction::None));
    assert!(matches!(handle(&mut srv, 0, addr(CLIENT + 1), &packet), ServerAction::None), "bound to the address");
    let mut other_id = packet.clone();
    other_id[22] ^= 1;
    assert!(matches!(handle(&mut srv, 0, addr(CLIENT), &other_id), ServerAction::None), "bound to the ids");
    assert_eq!(srv.stats().bad_cookies, 3);
    assert_eq!(srv.half_open(), 0);
    assert!(matches!(handle(&mut srv, 0, addr(CLIENT), &packet), ServerAction::Reply(_)), "the real one works");
    assert_eq!(srv.half_open(), 1);
}

#[test]
fn a_cookie_lives_for_the_current_and_the_previous_bucket() {
    let mut srv = server(0);
    let mut cli = client(addr(SERVER), 1);
    let packet = hello_cookie(&mut cli, &mut srv, addr(CLIENT), 14_999);
    assert!(matches!(handle(&mut srv, 29_999, addr(CLIENT), &packet), ServerAction::Reply(_)), "next bucket");

    let mut srv = server(0);
    let mut cli = client(addr(SERVER), 1);
    let packet = hello_cookie(&mut cli, &mut srv, addr(CLIENT), 14_999);
    assert!(matches!(handle(&mut srv, 30_000, addr(CLIENT), &packet), ServerAction::None), "two buckets later");
    assert_eq!(srv.stats().bad_cookies, 1);
}

#[test]
fn a_cookie_survives_one_secret_rotation_but_not_two() {
    let mut srv = server(0);
    let mut cli = client(addr(SERVER), 1);
    let packet = hello_cookie(&mut cli, &mut srv, addr(CLIENT), 0);
    srv.rotate_cookie_secret([8; 32], 1);
    assert!(matches!(handle(&mut srv, 2, addr(CLIENT), &packet), ServerAction::Reply(_)));

    let mut srv = server(0);
    let mut cli = client(addr(SERVER), 1);
    let packet = hello_cookie(&mut cli, &mut srv, addr(CLIENT), 0);
    srv.rotate_cookie_secret([8; 32], 1);
    srv.rotate_cookie_secret([9; 32], 2);
    assert!(matches!(handle(&mut srv, 3, addr(CLIENT), &packet), ServerAction::None));
}

#[test]
fn rejects_go_only_to_a_valid_cookie() {
    let mut srv = server(0);
    let config = ClientConfig::lan(MAJOR + 1, PROLOGUE);
    let mut cli = HandshakeClient::new(config, &identity(2), addr(SERVER), 1).expect("client");
    let packet = hello_cookie(&mut cli, &mut srv, addr(CLIENT), 0);
    let mut out = vec![0u8; MAX_DATAGRAM];
    let action = srv.handle(0, addr(CLIENT), &packet, &mut out);
    assert!(matches!(action, ServerAction::Reply(2)));
    assert_eq!(cli.handle(addr(SERVER), &out[..2]), ClientStatus::Failed(HandshakeError::Rejected(2)));

    let mut forged = packet.clone();
    forged[40] ^= 0xFF;
    assert!(matches!(handle(&mut srv, 0, addr(CLIENT), &forged), ServerAction::None), "no Reject without a cookie");

    let mut srv = server(0);
    srv.set_shutting_down(true);
    let mut cli = client(addr(SERVER), 1);
    let packet = hello_cookie(&mut cli, &mut srv, addr(CLIENT), 0);
    let action = srv.handle(0, addr(CLIENT), &packet, &mut out);
    assert!(matches!(action, ServerAction::Reply(2)));
    assert_eq!(cli.handle(addr(SERVER), &out[..2]), ClientStatus::Failed(HandshakeError::Rejected(3)));
}

#[test]
fn a_full_server_rejects_with_reason_one() {
    let mut config = ServerConfig::lan(MAJOR, PROLOGUE);
    config.limits.max_connections = 1;
    let mut srv = HandshakeServer::with_cookie_secret(config, identity(1), [7; 32], 0);
    let mut first = client(addr(SERVER), 1);
    direct_handshake(&mut first, &mut srv, addr(CLIENT), 0);
    let mut second = client(addr(SERVER), 2);
    let packet = hello_cookie(&mut second, &mut srv, addr(CLIENT + 1), 0);
    let mut out = vec![0u8; MAX_DATAGRAM];
    assert!(matches!(srv.handle(0, addr(CLIENT + 1), &packet, &mut out), ServerAction::Reply(2)));
    assert_eq!(second.handle(addr(SERVER), &out[..2]), ClientStatus::Failed(HandshakeError::Rejected(1)));
    srv.connection_closed(1, addr(CLIENT));
    assert_eq!(srv.stats().rejects, 1);
}

#[test]
fn a_repeated_hello_cookie_gets_the_same_message_two_and_replays_are_ignored() {
    let mut srv = server(0);
    let mut cli = client(addr(SERVER), 1);
    let packet = hello_cookie(&mut cli, &mut srv, addr(CLIENT), 0);
    let mut first = vec![0u8; MAX_DATAGRAM];
    let ServerAction::Reply(n1) = srv.handle(0, addr(CLIENT), &packet, &mut first) else { panic!("msg2") };
    let mut second = vec![0u8; MAX_DATAGRAM];
    let ServerAction::Reply(n2) = srv.handle(10, addr(CLIENT), &packet, &mut second) else { panic!("msg2 again") };
    assert_eq!(first[..n1], second[..n2], "a resend, not a second handshake");
    assert_eq!(srv.half_open(), 1);

    cli.handle(addr(SERVER), &first[..n1]);
    let mut msg3 = vec![0u8; MAX_DATAGRAM];
    let len = cli.poll_transmit(20, &mut msg3).expect("msg3");
    assert!(matches!(handle(&mut srv, 20, addr(CLIENT), &msg3[..len]), ServerAction::Established(_)));
    assert!(matches!(handle(&mut srv, 30, addr(CLIENT), &msg3[..len]), ServerAction::None), "msg3 replayed");
    assert!(matches!(handle(&mut srv, 40, addr(CLIENT), &packet), ServerAction::None), "HelloCookie replayed");
    assert_eq!(srv.half_open(), 0);
    assert_eq!(srv.stats().established, 1);
}

#[test]
fn a_half_open_handshake_is_freed_after_five_seconds() {
    let mut srv = server(0);
    let mut cli = client(addr(SERVER), 1);
    let packet = hello_cookie(&mut cli, &mut srv, addr(CLIENT), 0);
    let mut out = vec![0u8; MAX_DATAGRAM];
    let ServerAction::Reply(n) = srv.handle(0, addr(CLIENT), &packet, &mut out) else { panic!("msg2") };
    cli.handle(addr(SERVER), &out[..n]);
    assert_eq!(srv.expire(4_999), 0);
    assert_eq!(srv.expire(5_000), 1);
    assert_eq!(srv.stats().timeouts, 1);
    let len = cli.poll_transmit(5_000, &mut out).expect("msg3");
    assert!(matches!(handle(&mut srv, 5_000, addr(CLIENT), &out[..len]), ServerAction::None));
    assert_eq!(srv.stats().unknown, 1);
}

#[test]
fn message_three_from_another_address_is_ignored() {
    let mut srv = server(0);
    let mut cli = client(addr(SERVER), 1);
    let packet = hello_cookie(&mut cli, &mut srv, addr(CLIENT), 0);
    let mut out = vec![0u8; MAX_DATAGRAM];
    let ServerAction::Reply(n) = srv.handle(0, addr(CLIENT), &packet, &mut out) else { panic!("msg2") };
    cli.handle(addr(SERVER), &out[..n]);
    let len = cli.poll_transmit(0, &mut out).expect("msg3");
    assert!(matches!(handle(&mut srv, 0, addr(CLIENT + 7), &out[..len]), ServerAction::None));
    assert_eq!(srv.half_open(), 1, "the slot survives a spoofed message 3");
    assert!(matches!(handle(&mut srv, 0, addr(CLIENT), &out[..len]), ServerAction::Established(_)));
}

#[test]
fn the_client_retries_on_schedule_then_reports_no_answer() {
    let mut cli = client(addr(SERVER), 1);
    let mut buf = vec![0u8; MAX_DATAGRAM];
    let mut sends = Vec::new();
    for now in (0..=5_000).step_by(50) {
        if cli.poll_transmit(now, &mut buf).is_some() {
            sends.push(now);
        }
    }
    assert_eq!(sends, vec![0, 500, 1_000, 2_000]);
    assert_eq!(cli.status(), ClientStatus::Failed(HandshakeError::NoAnswer));
}

#[test]
fn the_client_ignores_strangers_and_stray_packets() {
    let mut srv = server(0);
    let mut cli = client(addr(SERVER), 1);
    let (_, challenge) = client_to_server(&mut cli, &mut srv, addr(CLIENT), 0);
    assert_eq!(cli.handle(addr(SERVER + 1), &challenge), ClientStatus::InProgress, "not from the server");
    let fake = Challenge { server_conn_id: 1, bucket: 0, cookie: [0; 32] }.encode();
    assert_eq!(cli.handle(addr(SERVER), &fake[..30]), ClientStatus::InProgress, "malformed: ignored");
    assert_eq!(cli.handle(addr(SERVER), &[0xD0, 1]), ClientStatus::InProgress, "a Reject before the cookie");
    assert_eq!(cli.handle(addr(SERVER), &challenge), ClientStatus::InProgress);
    let mut buf = vec![0u8; MAX_DATAGRAM];
    let len = cli.poll_transmit(0, &mut buf).expect("HelloCookie");
    assert!(HelloCookie::decode(&buf[..len]).is_ok());
    assert_eq!(cli.handle(addr(SERVER), &challenge), ClientStatus::InProgress, "a late Challenge is ignored");
}

#[test]
fn per_address_caps_and_ignored_addresses_hold() {
    let mut srv = server(0);
    let ip_ports: Vec<SocketAddr> = (0..6).map(|i| SocketAddr::from(([10, 9, 9, 9], 5000 + i))).collect();
    let mut opened = 0;
    for (i, from) in ip_ports.iter().enumerate() {
        let mut cli = client(addr(SERVER), i as u16 + 1);
        let packet = hello_cookie(&mut cli, &mut srv, *from, 0);
        if matches!(handle(&mut srv, 0, *from, &packet), ServerAction::Reply(_)) {
            opened += 1;
        }
    }
    assert_eq!(opened, 5, "5 new handshakes per 10 s from one address");

    let stranger = addr(CLIENT);
    for _ in 0..10 {
        srv.auth_failed(0, stranger);
    }
    let mut cli = client(addr(SERVER), 99);
    let mut buf = vec![0u8; MAX_DATAGRAM];
    let len = cli.poll_transmit(0, &mut buf).expect("Hello");
    assert!(matches!(handle(&mut srv, 0, stranger, &buf[..len]), ServerAction::None), "ignored after auth failures");
}
