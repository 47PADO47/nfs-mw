//! Full queues: the producer never waits, the oldest entries go, and every one is counted.

use std::net::UdpSocket;
use std::thread;
use std::time::Duration;

use super::support::{WAIT, client_config, client_connected, loopback, server_config, server_connected, wait_for};
use crate::endpoint::{ClientEndpoint, EndpointConfig, ServerEndpoint, ServerEvent};

#[test]
fn a_full_event_queue_drops_the_oldest_and_counts_them() {
    let mut config = server_config();
    config.endpoint = EndpointConfig { event_capacity: 8, ..config.endpoint };
    let mut server = ServerEndpoint::bind(loopback(), None, config).expect("the server binds");
    let mut client = ClientEndpoint::connect(server.local_addr(), client_config(70)).expect("the client starts");
    client_connected(&mut client);
    for seq in 0..100u8 {
        client.send_reliable(1, vec![seq]);
    }
    // Connected plus 100 messages, and nothing read in between.
    let total = 101;
    let settled = wait_for(WAIT, || (server.pending_events() as u64 + server.dropped_events() >= total).then_some(()));
    assert!(settled.is_some(), "every event is queued or counted");
    thread::sleep(Duration::from_millis(100));
    assert_eq!(server.pending_events(), 8, "the queue holds its capacity");
    assert_eq!(server.dropped_events(), total - 8, "the rest are counted");
    let kept: Vec<u8> = std::iter::from_fn(|| server.poll_event())
        .map(|event| match event {
            ServerEvent::Reliable { bytes, .. } => bytes.first().copied().unwrap_or_default(),
            other => panic!("only the newest messages are left, got {other:?}"),
        })
        .collect();
    assert_eq!(kept, (92..100).collect::<Vec<u8>>(), "the newest survive, in order");
}

#[test]
fn a_full_command_queue_drops_the_oldest_and_counts_them() {
    // Nothing answers, so the connection never opens and the client takes no sends off its queue.
    let silent = UdpSocket::bind(loopback()).expect("bind");
    let mut config = client_config(71);
    config.endpoint = EndpointConfig { command_capacity: 4, ..config.endpoint };
    let mut client = ClientEndpoint::connect(silent.local_addr().expect("addr"), config).expect("the client starts");
    for seq in 0..10u8 {
        client.send_reliable(0, vec![seq]);
    }
    assert_eq!(client.dropped_commands(), 6, "the sender never waits: six of ten dropped");
}

#[test]
fn a_server_command_flood_never_blocks_the_game_thread() {
    let mut config = server_config();
    config.endpoint = EndpointConfig { command_capacity: 2, ..config.endpoint };
    let mut server = ServerEndpoint::bind(loopback(), None, config).expect("the server binds");
    let mut client = ClientEndpoint::connect(server.local_addr(), client_config(72)).expect("the client starts");
    client_connected(&mut client);
    let peer = server_connected(&mut server);
    let started = std::time::Instant::now();
    for _ in 0..100_000 {
        server.send_unreliable(peer, 0x01, vec![0; 16], None);
    }
    assert!(started.elapsed() < WAIT, "pushing never waits on the net thread");
    assert!(server.dropped_commands() > 0, "with two slots, most of a tight loop of sends is dropped and counted");
}
