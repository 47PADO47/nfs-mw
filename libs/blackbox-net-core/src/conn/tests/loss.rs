//! Reliable delivery over the simulated wires: every message arrives once, in order, on its own channel, the queues
//! drain, and every packet is reported exactly once.

use super::harness::{Harness, Side};
use crate::conn::{CHANNELS, ConnError};
use crate::rng::SplitMix64;

/// Messages each loss case sends.
const MESSAGES: usize = 2000;

/// A message of random size: mostly small, some up to the 16,382-byte limit.
fn random_message(rng: &mut SplitMix64) -> Vec<u8> {
    let len: u64 = match rng.below(100) {
        0..=79 => rng.below(200),
        80..=94 => 200 + rng.below(1800),
        _ => 2000 + rng.below(14_383),
    };
    (0..len).map(|_| rng.next_u64() as u8).collect()
}

/// Checks the outcome events: none is counted twice, and every packet sent at least 100 rounds before the end has
/// exactly one (it is older than the 1 s age limit and every acknowledgement has had time to arrive).
fn check_outcomes(side: &Side, end_round: u64) {
    for (number, (&count, &round)) in side.outcomes.iter().zip(&side.sent_round).enumerate() {
        assert!(count <= 1, "packet {number} reported {count} times");
        if round + 100 <= end_round {
            assert_eq!(count, 1, "packet {number} sent in round {round} has no outcome");
        }
    }
}

/// Checks a finished run: no read errors but duplicates, empty send queues, no extra messages, and the outcomes.
pub fn verify(mut h: Harness) {
    h.settle(150);
    assert!(h.errors.is_empty(), "unexpected read errors: {:?}", h.errors);
    assert_eq!(h.a.conn.pending_messages(), 0, "send queues drained");
    let mut buf = Vec::new();
    assert_eq!(h.b.conn.recv_message(&mut buf), None, "no extra messages");
    check_outcomes(&h.a, h.round);
    check_outcomes(&h.b, h.round);
}

/// Sends `MESSAGES` random messages over the four channels, keeping each channel under its limit, until all of them
/// have arrived and been acknowledged. Returns the finished harness.
fn run_messages(loss: f64, seed: u64) -> Harness {
    let mut rng = SplitMix64(seed);
    let mut plan: [Vec<Vec<u8>>; CHANNELS] = std::array::from_fn(|_| Vec::new());
    for _ in 0..MESSAGES {
        let channel = rng.below(CHANNELS as u64) as usize;
        plan[channel].push(random_message(&mut rng));
    }
    let mut h = Harness::new(seed ^ 0x5EED, loss, loss / 2.0);
    let mut sent = [0usize; CHANNELS];
    let mut got = [0usize; CHANNELS];
    let mut buf = Vec::new();
    loop {
        let all_arrived = got.iter().zip(&plan).all(|(done, msgs)| *done == msgs.len());
        if all_arrived && h.a.conn.pending_messages() == 0 {
            return h;
        }
        assert!(h.round < 200_000, "not delivered after 200,000 rounds");
        for channel in 0..CHANNELS {
            while let Some(msg) = plan[channel].get(sent[channel]) {
                match h.a.conn.send_message(channel as u8, msg) {
                    Ok(()) => sent[channel] += 1,
                    Err(ConnError::QueueFull) => break,
                    Err(error) => panic!("send_message: {error}"),
                }
            }
        }
        h.step(&[]);
        while let Some(channel) = h.b.conn.recv_message(&mut buf) {
            let ch = usize::from(channel);
            let index = got[ch];
            let expected = plan[ch].get(index).expect("more messages than were sent");
            assert_eq!(&buf, expected, "channel {ch}, message {index}");
            got[ch] += 1;
        }
    }
}

#[test]
fn messages_arrive_in_order_without_loss() {
    verify(run_messages(0.0, 1));
}

#[test]
fn messages_arrive_in_order_with_two_percent_loss() {
    verify(run_messages(0.02, 2));
}

#[test]
fn messages_arrive_in_order_with_five_percent_loss() {
    verify(run_messages(0.05, 3));
}

#[test]
fn messages_arrive_in_order_with_ten_percent_loss() {
    verify(run_messages(0.10, 4));
}

#[test]
fn message_ids_wrap_past_u16() {
    const COUNT: usize = 70_000;
    let mut h = Harness::new(9, 0.0, 0.0);
    let mut sent = 0usize;
    let mut got = 0usize;
    let mut buf = Vec::new();
    while got < COUNT {
        while sent < COUNT {
            let msg = [(sent % 251) as u8];
            match h.a.conn.send_message(2, &msg) {
                Ok(()) => sent += 1,
                Err(ConnError::QueueFull) => break,
                Err(error) => panic!("send_message: {error}"),
            }
        }
        h.step(&[]);
        while let Some(channel) = h.b.conn.recv_message(&mut buf) {
            assert_eq!(channel, 2);
            assert_eq!(buf, [(got % 251) as u8], "message {got}");
            got += 1;
        }
        assert!(h.round < 100_000, "stalled at message {got}");
    }
    verify(h);
}

#[test]
fn app_frames_keep_their_order_within_a_packet_and_never_repeat() {
    // Each round A sends two app frames in one packet. The wire reorders datagrams, so packets may arrive in any
    // order, but the two frames of one packet come out together and in order, and no packet is handed out twice.
    let mut h = Harness::new(3, 0.05, 0.02);
    let mut buf = Vec::new();
    for round in 0..3000u32 {
        let bytes = round.to_le_bytes();
        let payload: &[u8] = &bytes;
        h.step(&[(0x20, payload), (0x21, payload)]);
        if round % 10 == 0 {
            // A full queue is fine here: the test only needs some reliable traffic alongside the app frames.
            let _ = h.a.conn.send_message(0, &round.to_le_bytes());
        }
        while h.b.conn.recv_message(&mut buf).is_some() {}
    }
    let mut seen = std::collections::HashSet::new();
    for pair in h.app_received.chunks(2) {
        let [(first_type, first), (second_type, second)] = pair else {
            panic!("app frames come in pairs, got {pair:?}");
        };
        assert_eq!((*first_type, *second_type), (0x20, 0x21), "pair order");
        assert_eq!(first, second, "both frames of a packet came from the same round");
        let value = u32::from_le_bytes(first[..4].try_into().expect("four bytes"));
        assert!(seen.insert(value), "round {value} handed out twice");
    }
    assert!(h.app_received.len() > 4000, "most app frames arrive: {}", h.app_received.len());
    verify(h);
}

#[test]
fn outcomes_follow_every_packet_on_both_sides() {
    let h = run_messages(0.05, 77);
    assert!(h.a.conn.loss() > 0.0, "loss is measured under loss: {}", h.a.conn.loss());
    assert!(h.a.conn.rtt_ms() > 0.0 && h.a.conn.rtt_ms() < 1000.0, "rtt estimate is sane: {}", h.a.conn.rtt_ms());
    verify(h);
}
