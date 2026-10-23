//! Connections ending: a kick, a leave, a server shutdown, a peer gone silent, a handshake nobody answers, and threads
//! joined when their handles go.

use std::net::UdpSocket;
use std::sync::atomic::Ordering;

use super::support::{
    MuteTransport, WAIT, client_config, client_connected, client_event, identity, loopback, quiet, server_config,
    server_connected, server_event, wait_for,
};
use crate::endpoint::{
    ClientEndpoint, ClientEvent, DisconnectReason, EndpointConfig, ServerEndpoint, ServerEvent, UdpTransport, reason,
};
use crate::secure::HandshakeError;

fn connected_pair() -> (ServerEndpoint, ClientEndpoint, crate::session::PeerId) {
    let mut server = ServerEndpoint::bind(loopback(), None, server_config()).expect("the server binds");
    let mut client = ClientEndpoint::connect(server.local_addr(), client_config(20)).expect("the client starts");
    client_connected(&mut client);
    let peer = server_connected(&mut server);
    (server, client, peer)
}

#[test]
fn a_kicked_client_hears_the_reason_and_stops() {
    let (mut server, mut client, peer) = connected_pair();
    server.disconnect(peer, reason::KICKED);
    let event = server_event(&mut server, |event| matches!(event, ServerEvent::Disconnected { .. }));
    assert_eq!(event, Some(ServerEvent::Disconnected { peer, reason: DisconnectReason::Local(reason::KICKED) }));
    let event = client_event(&mut client, |event| matches!(event, ClientEvent::Disconnected(_)));
    assert_eq!(event, Some(ClientEvent::Disconnected(DisconnectReason::Remote(reason::KICKED))));
    assert!(wait_for(WAIT, || (!client.is_running()).then_some(())).is_some(), "the client thread ends");
}

#[test]
fn a_client_that_leaves_is_reported_by_the_server() {
    let (mut server, mut client, peer) = connected_pair();
    client.disconnect(reason::NORMAL);
    let event = client_event(&mut client, |event| matches!(event, ClientEvent::Disconnected(_)));
    assert_eq!(event, Some(ClientEvent::Disconnected(DisconnectReason::Local(reason::NORMAL))));
    let event = server_event(&mut server, |event| matches!(event, ServerEvent::Disconnected { .. }));
    assert_eq!(event, Some(ServerEvent::Disconnected { peer, reason: DisconnectReason::Remote(reason::NORMAL) }));
}

#[test]
fn a_server_shutdown_notifies_every_client() {
    let mut server = ServerEndpoint::bind(loopback(), None, server_config()).expect("the server binds");
    let mut clients: Vec<ClientEndpoint> = (0..3)
        .map(|index| ClientEndpoint::connect(server.local_addr(), client_config(30 + index)).expect("client"))
        .collect();
    let mut host = server.connect_local(identity(39).key_id(), quiet(), None).expect("attach");
    for client in &mut clients {
        client_connected(client);
    }
    client_connected(&mut host);
    for _ in 0..4 {
        server_connected(&mut server);
    }
    server.shutdown();
    assert!(!server.is_running(), "shutdown joins the thread");
    let mut shut = 0;
    while let Some(event) = server.poll_event() {
        assert!(matches!(event, ServerEvent::Disconnected { reason: DisconnectReason::Shutdown, .. }), "{event:?}");
        shut += 1;
    }
    assert_eq!(shut, 4, "every peer is reported");
    for client in clients.iter_mut().chain(std::iter::once(&mut host)) {
        let event = client_event(client, |event| matches!(event, ClientEvent::Disconnected(_)));
        assert_eq!(event, Some(ClientEvent::Disconnected(DisconnectReason::Remote(reason::SHUTTING_DOWN))));
    }
}

#[test]
fn a_silent_peer_is_removed_after_the_idle_timeout() {
    let mut config = server_config();
    config.endpoint = EndpointConfig { idle_timeout_ms: 500, keepalive_ms: 100, ..config.endpoint };
    let mut server = ServerEndpoint::bind(loopback(), None, config).expect("the server binds");
    let (transport, mute) = MuteTransport::new(UdpTransport::bind(loopback()).expect("bind"));
    let mut client_cfg = client_config(21);
    client_cfg.endpoint.idle_timeout_ms = 60_000;
    let mut client = ClientEndpoint::spawn(transport, server.local_addr(), client_cfg).expect("the client starts");
    client_connected(&mut client);
    let peer = server_connected(&mut server);
    mute.store(true, Ordering::Relaxed);
    let event = server_event(&mut server, |event| matches!(event, ServerEvent::Disconnected { .. }));
    assert_eq!(event, Some(ServerEvent::Disconnected { peer, reason: DisconnectReason::Timeout }));
    let event = client_event(&mut client, |event| matches!(event, ClientEvent::Disconnected(_)));
    assert_eq!(
        event,
        Some(ClientEvent::Disconnected(DisconnectReason::Remote(reason::TIMEOUT))),
        "told on its way out"
    );
}

#[test]
fn a_handshake_nobody_answers_fails_with_no_answer() {
    let silent = UdpSocket::bind(loopback()).expect("bind");
    let addr = silent.local_addr().expect("addr");
    let mut config = client_config(22);
    config.handshake.retry_ms = vec![50; 4];
    let mut client = ClientEndpoint::connect(addr, config).expect("the client starts");
    let event = client_event(&mut client, |event| matches!(event, ClientEvent::Disconnected(_)));
    assert_eq!(event, Some(ClientEvent::Disconnected(DisconnectReason::Handshake(HandshakeError::NoAnswer))));
    assert!(wait_for(WAIT, || (!client.is_running()).then_some(())).is_some(), "the client thread ends");
}

#[test]
fn dropping_the_handles_joins_every_thread() {
    let (mut server, client, _peer) = connected_pair();
    let host = server.connect_local(identity(40).key_id(), quiet(), None).expect("attach");
    let probes = [server.probe(), client.probe(), host.probe()];
    assert!(probes.iter().all(|probe| probe.upgrade().is_some()), "all three threads run");
    drop(client);
    drop(host);
    drop(server);
    assert!(probes.iter().all(|probe| probe.upgrade().is_none()), "no thread outlives its handle");
}
