//! The same endpoints over the seeded packet simulator instead of sockets: 5 % loss, duplicates, reordering and a few
//! milliseconds of jitter on every link, the handshake included. Reliable messages arrive complete and in order;
//! tagged unreliable frames come back as deliveries or losses.

use std::net::SocketAddr;
use std::thread;
use std::time::{Duration, Instant};

use super::support::{WAIT, client_config, server_config, wait_for};
use crate::endpoint::{ClientEndpoint, ClientEvent, ServerEndpoint, ServerEvent};
use crate::link::{LossConfig, LossyLink, MemoryLink, MemoryNet};

const CLIENTS: usize = 16;
/// Reliable messages each client sends.
const MESSAGES: u8 = 20;

fn lossy(net: &MemoryNet, addr: SocketAddr, seed: u64) -> LossyLink<MemoryLink> {
    let config = LossConfig { loss: 0.05, latency_ms: 3, jitter_ms: 4, duplicate: 0.05, reorder: 0.05, seed };
    LossyLink::new(net.endpoint(addr), config)
}

#[test]
fn sixteen_clients_over_the_lossy_memory_link_get_every_reliable_message_in_order() {
    let net = MemoryNet::new();
    let server_addr = SocketAddr::from(([10, 0, 0, 1], 7000));
    let mut server =
        ServerEndpoint::spawn(lossy(&net, server_addr, 1), None, server_config()).expect("the server starts");
    let mut clients: Vec<ClientEndpoint> = (0..CLIENTS)
        .map(|index| {
            let addr = SocketAddr::from(([10, 0, 1, index as u8 + 1], 7001));
            let link = lossy(&net, addr, 100 + index as u64);
            ClientEndpoint::spawn(link, server_addr, client_config(10 + index as u8)).expect("the client starts")
        })
        .collect();
    for (index, client) in clients.iter_mut().enumerate() {
        for seq in 0..MESSAGES {
            client.send_reliable(1, vec![index as u8, seq]);
        }
    }

    let mut received: Vec<Vec<u8>> = vec![Vec::new(); CLIENTS];
    let mut deliveries = 0u32;
    let mut tag = 0u32;
    let deadline = Instant::now() + WAIT * 2;
    while received.iter().any(|got| got.len() < usize::from(MESSAGES)) {
        assert!(Instant::now() < deadline, "timed out: {:?}", received.iter().map(Vec::len).collect::<Vec<_>>());
        while let Some(event) = server.poll_event() {
            match event {
                ServerEvent::Reliable { peer, channel, bytes } => {
                    server.send_reliable(peer, channel, bytes.clone());
                    tag += 1;
                    server.send_unreliable(peer, 0x02, bytes, Some(tag));
                }
                ServerEvent::Delivered { .. } => deliveries += 1,
                ServerEvent::Disconnected { peer, reason } => panic!("{peer:?} disconnected: {reason}"),
                _ => {}
            }
        }
        for (index, client) in clients.iter_mut().enumerate() {
            while let Some(event) = client.poll_event() {
                match event {
                    ClientEvent::Reliable { channel, bytes } => {
                        assert_eq!(channel, 1);
                        assert_eq!(bytes.first(), Some(&(index as u8)), "a client gets only its own echoes");
                        received[index].extend(bytes.get(1).copied());
                    }
                    ClientEvent::Disconnected(reason) => panic!("client {index} disconnected: {reason}"),
                    _ => {}
                }
            }
        }
        thread::sleep(Duration::from_millis(1));
    }
    for (index, got) in received.iter().enumerate() {
        assert_eq!(got, &(0..MESSAGES).collect::<Vec<u8>>(), "client {index}: complete, in order, once each");
    }
    let delivered = wait_for(WAIT, || {
        while let Some(event) = server.poll_event() {
            if matches!(event, ServerEvent::Delivered { .. }) {
                deliveries += 1;
            }
        }
        (deliveries > 0).then_some(deliveries)
    });
    assert!(delivered.is_some(), "tagged frames are reported delivered");
}
