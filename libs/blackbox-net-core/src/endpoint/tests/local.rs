//! The listen host's own client (ADR 0009 section 3): the in-memory link, no socket and no handshake, but the same
//! events and calls as a remote client on both sides.

use std::thread;
use std::time::{Duration, Instant};

use super::support::{
    WAIT, client_config, client_connected, client_event, echo, identity, loopback, quiet, server_config,
    server_connected, server_event,
};
use crate::endpoint::{ClientEndpoint, ClientEvent, DisconnectReason, ServerEndpoint, ServerEvent, reason};

/// The snapshot-like frame type of the latest-value tests.
const SNAPSHOT: u8 = 0x02;

#[test]
fn the_host_client_and_a_remote_client_look_the_same_to_the_game() {
    let mut server = ServerEndpoint::bind(loopback(), None, server_config()).expect("the server binds");
    let host_key = identity(50).key_id();
    let mut host = server.connect_local(host_key, quiet(), None).expect("the host client attaches");
    let mut remote = ClientEndpoint::connect(server.local_addr(), client_config(51)).expect("the client starts");

    let mut seen = Vec::new();
    while seen.len() < 2 {
        let event = server_event(&mut server, |event| matches!(event, ServerEvent::Connected { .. }));
        let Some(ServerEvent::Connected { key, local, .. }) = event else {
            panic!("both clients connect");
        };
        seen.push((key, local));
    }
    seen.sort_by_key(|&(_, local)| local);
    assert_eq!(seen, vec![(identity(51).key_id(), false), (host_key, true)]);

    let event = client_event(&mut host, |event| matches!(event, ClientEvent::Connected { .. }));
    assert_eq!(event, Some(ClientEvent::Connected { server_key: server.key_id() }), "the host knows its own server");
    client_connected(&mut remote);

    host.send_reliable(2, b"host".to_vec());
    remote.send_reliable(2, b"remote".to_vec());
    let mut answers = [false; 2];
    let deadline = Instant::now() + WAIT;
    while answers != [true, true] {
        assert!(Instant::now() < deadline, "the echoes come back");
        echo(&mut server);
        for (index, client) in [&mut host, &mut remote].into_iter().enumerate() {
            while let Some(event) = client.poll_event() {
                if let ClientEvent::Reliable { channel: 2, bytes } = event {
                    let expected: &[u8] = [&b"host"[..], &b"remote"[..]][index];
                    assert_eq!(bytes, expected);
                    answers[index] = true;
                }
            }
        }
        thread::sleep(Duration::from_millis(1));
    }
}

#[test]
fn the_latest_value_cell_holds_the_newest_snapshot_frame() {
    let mut server = ServerEndpoint::bind(loopback(), None, server_config()).expect("the server binds");
    let mut host = server.connect_local(identity(50).key_id(), quiet(), Some(SNAPSHOT)).expect("attach");
    let peer = server_connected(&mut server);
    assert_eq!(host.take_latest(), None, "nothing yet");
    for tick in 0..200u32 {
        server.send_unreliable(peer, SNAPSHOT, tick.to_le_bytes().to_vec(), None);
    }
    server.send_unreliable(peer, 0x05, b"other".to_vec(), None);

    let newest = 199u32.to_le_bytes().to_vec();
    let got = super::support::wait_for(WAIT, || host.take_latest().filter(|frame| frame.bytes == newest).cloned());
    let frame = got.expect("the newest snapshot frame arrives");
    assert_eq!(frame.frame_type, SNAPSHOT);
    assert_eq!(frame.seq, 200, "the in-memory link loses nothing");
    assert_eq!(host.take_latest(), None, "taken once");
    assert_eq!(host.latest().map(|frame| frame.seq), Some(200), "still readable");

    let other = client_event(&mut host, |event| matches!(event, ClientEvent::Unreliable { .. }));
    assert_eq!(other, Some(ClientEvent::Unreliable { frame_type: 0x05, bytes: b"other".to_vec() }));
    while let Some(event) = host.poll_event() {
        assert!(!matches!(event, ClientEvent::Unreliable { frame_type: SNAPSHOT, .. }), "snapshots skip the queue");
    }
}

#[test]
fn tagged_frames_to_the_host_client_are_reported_delivered() {
    let mut server = ServerEndpoint::bind(loopback(), None, server_config()).expect("the server binds");
    let _host = server.connect_local(identity(50).key_id(), quiet(), None).expect("attach");
    let peer = server_connected(&mut server);
    server.send_unreliable(peer, SNAPSHOT, vec![1, 2, 3], Some(77));
    let event = server_event(&mut server, |event| matches!(event, ServerEvent::Delivered { .. }));
    assert_eq!(event, Some(ServerEvent::Delivered { peer, tag: 77 }));
}

#[test]
fn a_kicked_host_client_hears_the_reason() {
    let mut server = ServerEndpoint::bind(loopback(), None, server_config()).expect("the server binds");
    let mut host = server.connect_local(identity(50).key_id(), quiet(), None).expect("attach");
    let peer = server_connected(&mut server);
    server.disconnect(peer, reason::KICKED);
    let event = server_event(&mut server, |event| matches!(event, ServerEvent::Disconnected { .. }));
    assert_eq!(event, Some(ServerEvent::Disconnected { peer, reason: DisconnectReason::Local(reason::KICKED) }));
    let event = client_event(&mut host, |event| matches!(event, ClientEvent::Disconnected(_)));
    assert_eq!(event, Some(ClientEvent::Disconnected(DisconnectReason::Remote(reason::KICKED))));
    assert!(super::support::wait_for(WAIT, || (!host.is_running()).then_some(())).is_some(), "its thread ends");
}
