//! Seeded fuzzing of `read_packet`: random datagrams, and real packets with random bytes flipped. A panic fails the test;
//! the results are ignored.

use crate::MAX_DATAGRAM;
use crate::conn::{ConnConfig, Connection, HEADER_LEN};
use crate::rng::SplitMix64;

/// Drains everything the connection has queued, so the fuzzed connection's queues stay small.
fn drain(conn: &mut Connection) {
    let mut buf = Vec::new();
    while conn.poll_event().is_some() {}
    while conn.recv_message(&mut buf).is_some() {}
}

#[test]
fn random_datagrams_never_panic() {
    let mut rng = SplitMix64(0xF022);
    let mut conn = Connection::new(ConnConfig { local_id: 2, remote_id: 1, ..ConnConfig::default() });
    for i in 0..100_000u64 {
        let len = rng.below(1301) as usize;
        let mut datagram: Vec<u8> = (0..len).map(|_| rng.next_u64() as u8).collect();
        // Half of them get a valid header for this connection, so the frame parser sees most of the input.
        if i % 2 == 0 && len >= HEADER_LEN {
            datagram[0] &= 0x01;
            datagram[1..3].copy_from_slice(&2u16.to_le_bytes());
        }
        let _ = conn.read_packet(i * 3, &datagram, |_, _| {});
        if i % 50 == 0 {
            let mut out = vec![0u8; MAX_DATAGRAM];
            let _ = conn.write_packet(i * 3, &[], &mut out);
            let _ = conn.send_message(rng.below(4) as u8, &datagram[..len.min(64)]);
        }
        drain(&mut conn);
    }
}

#[test]
fn mutated_real_packets_never_panic() {
    let mut rng = SplitMix64(0xB17F);
    let mut sender = Connection::new(ConnConfig { local_id: 1, remote_id: 2, ..ConnConfig::default() });
    let mut receiver = Connection::new(ConnConfig { local_id: 2, remote_id: 1, ..ConnConfig::default() });

    // A pool of real packets carrying reliable fragments of every size class and acknowledgements.
    let mut pool: Vec<Vec<u8>> = Vec::new();
    let mut out = vec![0u8; MAX_DATAGRAM];
    for round in 0..500u64 {
        let len = match rng.below(3) {
            0 => rng.below(64),
            1 => rng.below(1100),
            _ => rng.below(3000),
        } as usize;
        let msg: Vec<u8> = (0..len).map(|_| rng.next_u64() as u8).collect();
        // A full channel just means this round sends no new message.
        let _ = sender.send_message(rng.below(4) as u8, &msg);
        let sent = sender.write_packet(round * 16, &[], &mut out).expect("write_packet");
        pool.push(out[..sent.len].to_vec());
        // Keep the sender's acknowledgement state moving so ack blocks vary across the pool.
        let _ = receiver.read_packet(round * 16, &pool[pool.len() - 1], |_, _| {});
        drain(&mut receiver);
        drain(&mut sender);
    }

    let mut target = Connection::new(ConnConfig { local_id: 2, remote_id: 1, ..ConnConfig::default() });
    for i in 0..100_000u64 {
        let mut datagram = pool[rng.below(pool.len() as u64) as usize].clone();
        let flips = 1 + rng.below(4);
        for _ in 0..flips {
            // Mostly past the header, so the frames are what changes; sometimes anywhere.
            let start = match i % 4 {
                0 => 0,
                _ => HEADER_LEN,
            };
            let span = datagram.len().saturating_sub(start).max(1);
            let at = start + rng.below(span as u64) as usize;
            if let Some(byte) = datagram.get_mut(at) {
                *byte ^= 1u8 << rng.below(8);
            }
        }
        let _ = target.read_packet(i * 2, &datagram, |_, _| {});
        drain(&mut target);
    }
}
