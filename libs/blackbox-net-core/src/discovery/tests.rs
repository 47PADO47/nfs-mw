use super::*;
use crate::rng::SplitMix64;

const NONCE: u64 = 0x0123_4567_89AB_CDEF;

fn sample_request() -> Request {
    Request { proto_major: 3, nonce: NONCE }
}

fn sample_reply(name: &str) -> Reply {
    Reply {
        proto_major: 3,
        nonce: NONCE,
        proto_minor: 7,
        key_hash: [1, 2, 3, 4, 5, 6, 7, 8],
        game_port: 30_000,
        players: 4,
        max_players: 8,
        flags: FLAG_PASSWORD | FLAG_DEDICATED,
        mode: 2,
        data_id: [9, 8, 7, 6],
        name: name.to_owned(),
    }
}

/// Every value other than the original at byte `at`, each as a whole datagram.
fn variants(encoded: [u8; DATAGRAM_LEN], at: usize) -> impl Iterator<Item = [u8; DATAGRAM_LEN]> {
    let original = encoded[at];
    (0..=255u8).filter(move |value| *value != original).map(move |value| {
        let mut bad = encoded;
        bad[at] = value;
        bad
    })
}

/// A 64-byte datagram with random bytes, a valid reply header, a name of random ASCII letters and zero padding.
fn random_valid_reply(rng: &mut SplitMix64) -> [u8; DATAGRAM_LEN] {
    let mut bytes = [0u8; DATAGRAM_LEN];
    for byte in bytes.iter_mut() {
        *byte = rng.next_u64() as u8;
    }
    bytes[0..4].copy_from_slice(b"NMDR");
    bytes[5] = FORMAT;
    bytes[33..57].fill(0);
    let len = rng.below(25) as usize;
    for byte in &mut bytes[33..33 + len] {
        *byte = b'a' + rng.below(26) as u8;
    }
    bytes[57..].fill(0);
    bytes
}

#[test]
fn request_round_trips_at_the_spec_offsets() {
    let encoded = sample_request().encode();
    assert_eq!(&encoded[0..4], b"NMDQ");
    assert_eq!(encoded[4], 3);
    assert_eq!(encoded[5], FORMAT);
    assert_eq!(&encoded[6..14], &NONCE.to_le_bytes());
    assert!(encoded[14..].iter().all(|b| *b == 0));
    assert_eq!(Request::decode(&encoded), Ok(sample_request()));
}

#[test]
fn reply_round_trips_at_the_spec_offsets() {
    let reply = sample_reply("Alpha");
    let encoded = reply.encode();
    assert_eq!(&encoded[0..4], b"NMDR");
    assert_eq!(encoded[4], 3);
    assert_eq!(encoded[5], FORMAT);
    assert_eq!(&encoded[6..14], &NONCE.to_le_bytes());
    assert_eq!(encoded[14], 7);
    assert_eq!(&encoded[15..23], &[1, 2, 3, 4, 5, 6, 7, 8]);
    assert_eq!(&encoded[23..25], &30_000u16.to_le_bytes());
    assert_eq!(encoded[25], 4);
    assert_eq!(encoded[26], 8);
    assert_eq!(encoded[27], 17);
    assert_eq!(encoded[28], 2);
    assert_eq!(&encoded[29..33], &[9, 8, 7, 6]);
    assert_eq!(&encoded[33..38], b"Alpha");
    assert!(encoded[38..].iter().all(|b| *b == 0));
    assert_eq!(Reply::decode(&encoded), Ok(reply));
}

#[test]
fn a_long_name_is_cut_on_a_character_boundary() {
    // One ASCII byte, then eight 3-byte characters: 25 bytes. Only seven characters fit after the 'a'.
    let name = format!("a{}", "日".repeat(8));
    let encoded = sample_reply(&name).encode();
    let decoded = Reply::decode(&encoded).unwrap();
    assert_eq!(decoded.name, format!("a{}", "日".repeat(7)));
    assert_eq!(encoded[33 + 22], 0);
}

#[test]
fn a_name_of_exactly_24_bytes_is_kept_whole() {
    let name = "é".repeat(12);
    assert_eq!(name.len(), 24);
    let decoded = Reply::decode(&sample_reply(&name).encode()).unwrap();
    assert_eq!(decoded.name, name);
}

#[test]
fn three_byte_characters_fill_eight_slots() {
    let name = "日".repeat(9);
    let decoded = Reply::decode(&sample_reply(&name).encode()).unwrap();
    assert_eq!(decoded.name, "日".repeat(8));
}

#[test]
fn an_empty_name_round_trips() {
    let decoded = Reply::decode(&sample_reply("").encode()).unwrap();
    assert_eq!(decoded.name, "");
}

#[test]
fn a_reply_answers_only_the_same_major_and_nonce() {
    let request = sample_request();
    let reply = sample_reply("x");
    assert!(reply.answers(&request));
    let other_nonce = Request { proto_major: 3, nonce: NONCE + 1 };
    assert!(!reply.answers(&other_nonce));
    let other_major = Request { proto_major: 4, nonce: NONCE };
    assert!(!reply.answers(&other_major));
}

#[test]
fn a_datagram_of_the_wrong_length_is_rejected() {
    for len in [0, 1, 63, 65, 128] {
        let bytes = vec![0u8; len];
        assert_eq!(Request::decode(&bytes), Err(DiscoveryError::Length), "len {len}");
        assert_eq!(Reply::decode(&bytes), Err(DiscoveryError::Length), "len {len}");
    }
}

#[test]
fn every_single_byte_corruption_of_a_request_is_rejected() {
    let encoded = sample_request().encode();
    let positions = (0..4).chain(std::iter::once(5)).chain(14..DATAGRAM_LEN);
    for at in positions {
        for bad in variants(encoded, at) {
            let expected = match at {
                0..=3 => DiscoveryError::Magic,
                5 => DiscoveryError::Format,
                _ => DiscoveryError::Padding,
            };
            assert_eq!(Request::decode(&bad), Err(expected), "byte {at}");
        }
    }
}

#[test]
fn every_single_byte_corruption_of_a_reply_header_or_padding_is_rejected() {
    let encoded = sample_reply("Alpha").encode();
    let positions = (0..4).chain(std::iter::once(5)).chain(57..DATAGRAM_LEN);
    for at in positions {
        for bad in variants(encoded, at) {
            let expected = match at {
                0..=3 => DiscoveryError::Magic,
                5 => DiscoveryError::Format,
                _ => DiscoveryError::Padding,
            };
            assert_eq!(Reply::decode(&bad), Err(expected), "byte {at}");
        }
    }
}

#[test]
fn nonzero_bytes_after_the_name_are_padding_errors() {
    // "ab" ends at byte 35, the first zero of the name field. A nonzero byte after that zero is padding; one before it
    // would be part of the name text.
    let encoded = sample_reply("ab").encode();
    for at in 36..57 {
        let mut bad = encoded;
        bad[at] = 1;
        assert_eq!(Reply::decode(&bad), Err(DiscoveryError::Padding), "byte {at}");
    }
}

#[test]
fn a_name_that_is_not_utf8_is_a_name_error() {
    let encoded = sample_reply("ab").encode();
    for at in [33, 34] {
        let mut bad = encoded;
        bad[at] = 0xFF;
        assert_eq!(Reply::decode(&bad), Err(DiscoveryError::Name), "byte {at}");
    }
}

#[test]
fn random_bytes_never_panic_the_decoders() {
    let mut rng = SplitMix64(0xD15C_0BE7);
    for _ in 0..20_000 {
        let len = rng.below(130) as usize;
        let bytes: Vec<u8> = (0..len).map(|_| rng.next_u64() as u8).collect();
        let _ = Request::decode(&bytes);
        let _ = Reply::decode(&bytes);
    }
}

#[test]
fn random_64_byte_datagrams_with_a_valid_header_round_trip() {
    let mut rng = SplitMix64(42);
    for _ in 0..5_000 {
        let mut bytes = [0u8; DATAGRAM_LEN];
        for byte in bytes.iter_mut() {
            *byte = rng.next_u64() as u8;
        }
        bytes[0..4].copy_from_slice(b"NMDQ");
        bytes[5] = FORMAT;
        bytes[14..].fill(0);
        let request = Request::decode(&bytes).unwrap();
        assert_eq!(request.encode(), bytes);
    }
}

#[test]
fn random_valid_replies_decode_and_encode_back_to_the_same_bytes() {
    let mut rng = SplitMix64(7);
    for _ in 0..5_000 {
        let bytes = random_valid_reply(&mut rng);
        let reply = Reply::decode(&bytes).unwrap();
        assert_eq!(reply.encode(), bytes);
    }
}

#[test]
fn flag_bits_are_distinct_powers_of_two() {
    let flags = [FLAG_PASSWORD, FLAG_IN_PROGRESS, FLAG_JOIN_IN_PROGRESS, FLAG_MODS, FLAG_DEDICATED];
    assert_eq!(flags, [1, 2, 4, 8, 16]);
}

#[test]
fn every_error_has_a_message() {
    let errors = [
        DiscoveryError::Length,
        DiscoveryError::Magic,
        DiscoveryError::Format,
        DiscoveryError::Padding,
        DiscoveryError::Name,
    ];
    for error in errors {
        assert!(!error.to_string().is_empty());
        let _: &dyn std::error::Error = &error;
    }
}
