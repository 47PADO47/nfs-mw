use std::io;
use std::net::SocketAddr;
use std::time::Duration;

use super::{Link, LossConfig, LossyLink, MemoryLink, MemoryNet, UdpLink};
use crate::MAX_DATAGRAM;

fn addr(port: u16) -> SocketAddr {
    SocketAddr::from(([127, 0, 0, 1], port))
}

fn pair() -> (MemoryLink, MemoryLink) {
    let net = MemoryNet::new();
    (net.endpoint(addr(1000)), net.endpoint(addr(2000)))
}

/// A sender behind a [`LossyLink`] and a plain receiver, both on one in-memory network.
fn lossy_pair(config: LossConfig) -> (LossyLink<MemoryLink>, MemoryLink) {
    let net = MemoryNet::new();
    let sender = LossyLink::new(net.endpoint(addr(1000)), config);
    let receiver = net.endpoint(addr(2000));
    (sender, receiver)
}

/// Receives everything waiting on `link`, reading each payload as a little-endian `u32`.
fn drain(link: &mut impl Link) -> Vec<u32> {
    let mut buf = [0u8; 64];
    let mut out = Vec::new();
    while let Some((_, len)) = link.recv(&mut buf).unwrap() {
        out.push(u32::from_le_bytes(buf[..len].try_into().unwrap()));
    }
    out
}

/// Polls `link` for up to about a second, for loopback UDP that may take a moment to arrive.
fn wait_recv(link: &mut impl Link, buf: &mut [u8]) -> Option<(SocketAddr, usize)> {
    for _ in 0..200 {
        if let Some(got) = link.recv(buf).unwrap() {
            return Some(got);
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    None
}

#[test]
fn memory_pair_delivers_both_ways_with_source() {
    let (mut a, mut b) = pair();
    assert_eq!(a.local_addr(), addr(1000));
    assert_eq!(b.local_addr(), addr(2000));

    a.send(addr(2000), b"ping").unwrap();
    b.send(addr(1000), b"pong!").unwrap();

    let mut buf = [0u8; 64];
    let (from, len) = b.recv(&mut buf).unwrap().unwrap();
    assert_eq!(from, addr(1000));
    assert_eq!(&buf[..len], b"ping");

    let (from, len) = a.recv(&mut buf).unwrap().unwrap();
    assert_eq!(from, addr(2000));
    assert_eq!(&buf[..len], b"pong!");

    assert_eq!(a.recv(&mut buf).unwrap(), None);
    assert_eq!(b.recv(&mut buf).unwrap(), None);
}

#[test]
fn memory_unknown_address_is_dropped() {
    let (mut a, mut b) = pair();
    a.send(addr(9999), b"lost").unwrap();
    let mut buf = [0u8; 16];
    assert_eq!(b.recv(&mut buf).unwrap(), None);
    assert_eq!(a.recv(&mut buf).unwrap(), None);
}

#[test]
fn memory_oversize_is_refused() {
    let (mut a, mut b) = pair();
    let big = vec![0u8; MAX_DATAGRAM + 1];
    let err = a.send(addr(2000), &big).unwrap_err();
    assert_eq!(err.kind(), io::ErrorKind::InvalidInput);

    let max = vec![7u8; MAX_DATAGRAM];
    a.send(addr(2000), &max).unwrap();
    let mut buf = [0u8; MAX_DATAGRAM + 10];
    let (_, len) = b.recv(&mut buf).unwrap().unwrap();
    assert_eq!(len, MAX_DATAGRAM);
    // The refused datagram was never queued.
    assert_eq!(b.recv(&mut buf).unwrap(), None);
}

#[test]
fn memory_queue_overflow_drops_newest() {
    let (mut a, mut b) = pair();
    for i in 0..4097u32 {
        a.send(addr(2000), &i.to_le_bytes()).unwrap();
    }
    let got = drain(&mut b);
    assert_eq!(got.len(), 4096);
    assert_eq!(got.last(), Some(&4095));
}

#[test]
fn memory_recv_truncates_to_buffer() {
    let (mut a, mut b) = pair();
    a.send(addr(2000), b"0123456789").unwrap();
    let mut small = [0u8; 4];
    let (from, len) = b.recv(&mut small).unwrap().unwrap();
    assert_eq!((from, len), (addr(1000), 4));
    assert_eq!(&small, b"0123");
}

#[test]
fn udp_loopback_round_trip() {
    let mut a = UdpLink::bind(addr(0)).unwrap();
    let mut b = UdpLink::bind(addr(0)).unwrap();
    assert_ne!(a.local_addr().port(), 0);

    let mut buf = [0u8; 64];
    assert_eq!(a.recv(&mut buf).unwrap(), None);

    a.send(b.local_addr(), b"hello udp").unwrap();
    let (from, len) = wait_recv(&mut b, &mut buf).expect("datagram arrives on loopback");
    assert_eq!(from, a.local_addr());
    assert_eq!(&buf[..len], b"hello udp");
}

#[test]
fn loss_rate_matches_config() {
    let config = LossConfig { loss: 0.1, seed: 1, ..LossConfig::default() };
    let (mut tx, mut rx) = lossy_pair(config);
    let mut received = 0u64;
    for i in 0..10_000u32 {
        tx.send(addr(2000), &i.to_le_bytes()).unwrap();
        received += drain(&mut rx).len() as u64;
    }
    assert_eq!(tx.sent(), 10_000);
    let rate = tx.dropped() as f64 / tx.sent() as f64;
    assert!((0.08..=0.12).contains(&rate), "loss rate {rate}");
    assert_eq!(received, 10_000 - tx.dropped());
}

#[test]
fn duplicate_rate_matches_config() {
    let config = LossConfig { duplicate: 0.05, seed: 2, ..LossConfig::default() };
    let (mut tx, mut rx) = lossy_pair(config);
    let mut received = 0u64;
    for i in 0..10_000u32 {
        tx.send(addr(2000), &i.to_le_bytes()).unwrap();
        received += drain(&mut rx).len() as u64;
    }
    let rate = tx.duplicated() as f64 / tx.sent() as f64;
    assert!((0.03..=0.07).contains(&rate), "duplicate rate {rate}");
    assert_eq!(received, 10_000 + tx.duplicated());
}

#[test]
fn latency_and_jitter_bound_delivery_delay() {
    let config = LossConfig { latency_ms: 50, jitter_ms: 20, seed: 3, ..LossConfig::default() };
    let (mut tx, mut rx) = lossy_pair(config);
    let mut delays = Vec::new();
    for now in 0..300u64 {
        tx.set_now(now);
        if now < 100 {
            tx.send(addr(2000), &(now as u32).to_le_bytes()).unwrap();
        }
        for sent_at in drain(&mut rx) {
            delays.push(now - u64::from(sent_at));
        }
    }
    assert_eq!(delays.len(), 100);
    assert!(delays.iter().all(|d| (50..=70).contains(d)), "delays out of range: {delays:?}");
    assert!(delays.iter().any(|&d| d != 50), "jitter had no effect");
}

#[test]
fn reorder_lets_later_datagrams_overtake() {
    let config = LossConfig { latency_ms: 10, reorder: 0.5, seed: 9, ..LossConfig::default() };
    let (mut tx, mut rx) = lossy_pair(config);
    let mut got = Vec::new();
    for now in 0..200u64 {
        tx.set_now(now);
        if now < 100 {
            tx.send(addr(2000), &(now as u32).to_le_bytes()).unwrap();
        }
        got.extend(drain(&mut rx));
    }
    assert_eq!(got.len(), 100);
    assert!(got.windows(2).any(|w| w[0] > w[1]), "no datagram overtook another");
}

/// Runs an impaired link for 300 sends and returns every payload received, in arrival order.
fn impaired_run(config: LossConfig) -> Vec<u32> {
    let (mut tx, mut rx) = lossy_pair(config);
    let mut got = Vec::new();
    for now in 0..500u64 {
        tx.set_now(now);
        if now < 300 {
            tx.send(addr(2000), &(now as u32).to_le_bytes()).unwrap();
        }
        got.extend(drain(&mut rx));
    }
    got
}

#[test]
fn same_seed_gives_same_delivery_sequence() {
    let config = LossConfig { loss: 0.2, latency_ms: 10, jitter_ms: 5, duplicate: 0.1, reorder: 0.3, seed: 42 };
    let first = impaired_run(config);
    assert!(!first.is_empty());
    assert_eq!(first, impaired_run(config));

    let other_seed = LossConfig { seed: 43, ..config };
    assert_ne!(first, impaired_run(other_seed));
}

#[test]
fn perfect_link_delivers_in_send_order_immediately() {
    let (mut tx, mut rx) = lossy_pair(LossConfig::default());
    assert_eq!(tx.local_addr(), addr(1000));
    for i in 0..100u32 {
        tx.send(addr(2000), &i.to_le_bytes()).unwrap();
    }
    // No set_now call: a zero-delay send is forwarded at once.
    assert_eq!(drain(&mut rx), (0..100).collect::<Vec<u32>>());
    assert_eq!((tx.sent(), tx.dropped(), tx.duplicated()), (100, 0, 0));
}
