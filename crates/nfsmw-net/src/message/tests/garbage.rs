//! Seeded garbage: decode never panics, and whatever it accepts re-encodes to the same bytes.

use blackbox_net_core::rng::SplitMix64;

use super::{bytes_of, samples};
use crate::message::*;

/// Decodes `bytes`; an accepted message at its known format must re-encode to exactly `bytes`.
fn check(bytes: &[u8]) {
    let message = match decode(bytes) {
        Ok(message) => message,
        Err(_) => return,
    };
    if bytes.get(2).copied() != Some(message.format()) {
        return;
    }
    assert_eq!(bytes_of(&message), bytes, "accepted a non-canonical {:#06x}", message.type_id());
}

#[test]
fn random_bodies_behind_every_known_header() {
    let mut rng = SplitMix64(0x5EED_0001);
    for message in samples::all() {
        let header = bytes_of(&message)[..HEADER_LEN].to_vec();
        for _ in 0..300 {
            let len = rng.below(96) as usize;
            let mut bytes = header.clone();
            bytes.extend((0..len).map(|_| rng.next_u64() as u8));
            check(&bytes);
        }
    }
}

#[test]
fn random_datagrams() {
    let mut rng = SplitMix64(0x5EED_0002);
    for _ in 0..20_000 {
        let len = rng.below(64) as usize;
        let bytes: Vec<u8> = (0..len).map(|_| rng.next_u64() as u8).collect();
        check(&bytes);
    }
}

#[test]
fn mutated_samples() {
    let mut rng = SplitMix64(0x5EED_0003);
    let samples: Vec<Vec<u8>> = samples::all().iter().map(bytes_of).collect();
    for _ in 0..40_000 {
        let mut bytes = samples[rng.below(samples.len() as u64) as usize].clone();
        let edits = 1 + rng.below(4);
        for _ in 0..edits {
            let at = rng.below(bytes.len() as u64) as usize;
            match rng.below(4) {
                0 => bytes[at] ^= 1 << rng.below(8),
                1 => bytes[at] = rng.next_u64() as u8,
                2 => bytes.insert(at, rng.next_u64() as u8),
                _ => {
                    bytes.remove(at);
                }
            }
            if bytes.is_empty() {
                break;
            }
        }
        check(&bytes);
    }
}

#[test]
fn huge_counts_do_not_allocate_or_panic() {
    // A list count of u64::MAX behind the LobbyState header, and a string length past the end behind a ChatSend.
    let lobby = [0x10, 0x00, 1, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x01];
    assert_eq!(decode(&lobby), Err(DecodeError::BadCount));
    let chat = [0x00, 0x02, 1, 0, 0, 0xFF, 0xFF, 0x03, b'h', b'i'];
    assert_eq!(decode(&chat), Err(DecodeError::BadText));
    let bulk = [0x01, 0x03, 1, 1, 0, 0xFF, 0x7F];
    assert_eq!(decode(&bulk), Err(DecodeError::BadCount));
}
