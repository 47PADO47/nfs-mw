//! Sixteen clients on real UDP sockets on loopback, each with a full Noise handshake, ping-ponging reliable messages
//! and unreliable frames through a server that echoes them.

use std::collections::HashSet;
use std::thread;
use std::time::{Duration, Instant};

use super::support::{WAIT, client_config, echo, loopback, server_config};
use crate::endpoint::{ClientEndpoint, ClientEvent, ServerEndpoint, ServerEvent};

const CLIENTS: usize = 16;
/// Unreliable frame type of the test.
const FRAME: u8 = 0x01;

#[test]
fn sixteen_clients_ping_pong_reliable_and_unreliable_data_over_loopback_udp() {
    let mut server = ServerEndpoint::bind(loopback(), None, server_config()).expect("the server binds");
    let addr = server.local_addr();
    let mut clients: Vec<ClientEndpoint> = (0..CLIENTS)
        .map(|index| ClientEndpoint::connect(addr, client_config(10 + index as u8)).expect("the client starts"))
        .collect();
    for (index, client) in clients.iter_mut().enumerate() {
        // Queued before the connection opens: it waits in the queue.
        client.send_reliable(1, format!("ping {index}").into_bytes());
    }

    let mut peers = HashSet::new();
    let mut reliable = [false; CLIENTS];
    let mut unreliable = [false; CLIENTS];
    let mut connected = [false; CLIENTS];
    let mut next_frame = Instant::now();
    let deadline = Instant::now() + WAIT;
    while !(reliable.iter().all(|&done| done) && unreliable.iter().all(|&done| done)) {
        assert!(Instant::now() < deadline, "timed out: reliable {reliable:?}, unreliable {unreliable:?}");
        for event in echo(&mut server) {
            if let ServerEvent::Connected { peer, local, .. } = event {
                assert!(!local);
                peers.insert(peer);
            }
        }
        // Unreliable frames may be lost, so they are resent every 20 ms until one comes back.
        let send_frames = Instant::now() >= next_frame;
        if send_frames {
            next_frame = Instant::now() + Duration::from_millis(20);
        }
        for (index, client) in clients.iter_mut().enumerate() {
            if send_frames && connected[index] && !unreliable[index] {
                client.send_unreliable(FRAME, format!("frame {index}").into_bytes(), None);
            }
            while let Some(event) = client.poll_event() {
                match event {
                    ClientEvent::Connected { server_key } => {
                        assert_eq!(server_key, server.key_id(), "the client authenticated the server's key");
                        connected[index] = true;
                    }
                    ClientEvent::Reliable { channel, bytes } => {
                        assert_eq!((channel, bytes), (1, format!("ping {index}").into_bytes()));
                        reliable[index] = true;
                    }
                    ClientEvent::Unreliable { frame_type, bytes } => {
                        assert_eq!((frame_type, bytes), (FRAME, format!("frame {index}").into_bytes()));
                        unreliable[index] = true;
                    }
                    ClientEvent::Disconnected(reason) => panic!("client {index} disconnected: {reason}"),
                    _ => {}
                }
            }
        }
        thread::sleep(Duration::from_millis(1));
    }
    assert_eq!(peers.len(), CLIENTS, "every client is its own peer");
    assert_eq!(server.dropped_events(), 0);
}
