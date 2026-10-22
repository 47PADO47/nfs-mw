//! Seeded garbage for every pre-authentication parser: random datagrams of every length, random datagrams with a valid
//! first byte, and real packets with random bits flipped. Nothing may panic; nothing may allocate a slot without a
//! valid cookie; nothing may answer with more bytes than it received; nothing may open without the key.

use super::harness::{addr, client, client_to_server, server};
use crate::MAX_DATAGRAM;
use crate::conn::{ConnConfig, Connection};
use crate::rng::SplitMix64;
use crate::secure::packet::{Challenge, Hello, HelloCookie, HsType, NoiseMsg, RejectReason};
use crate::secure::password::{PakeRole, start};
use crate::secure::token::{TokenKey, TokenKeyring, open_token};
use crate::secure::{ClientStatus, NoiseSealer, SealConfig, ServerAction, classify};

const ROUNDS: u64 = 50_000;

fn random_bytes(rng: &mut SplitMix64, len: usize) -> Vec<u8> {
    (0..len).map(|_| rng.next_u64() as u8).collect()
}

/// A random datagram: any length up to 1,300 bytes, or one of the exact handshake sizes, with a random or a valid
/// handshake first byte.
fn garbage(rng: &mut SplitMix64) -> Vec<u8> {
    let len = match rng.below(4) {
        0 => rng.below(1301) as usize,
        1 => 512,
        2 => [2usize, 36, 101, 69, 5][rng.below(5) as usize],
        _ => rng.below(64) as usize,
    };
    let mut bytes = random_bytes(rng, len);
    if let Some(first) = bytes.first_mut()
        && rng.below(2) == 0
    {
        *first = 0x80 | ((rng.below(6) as u8) << 4);
    }
    bytes
}

fn flip(rng: &mut SplitMix64, bytes: &mut [u8]) {
    for _ in 0..1 + rng.below(4) {
        let at = rng.below(bytes.len().max(1) as u64) as usize;
        if let Some(byte) = bytes.get_mut(at) {
            *byte ^= 1 << rng.below(8);
        }
    }
}

#[test]
fn the_packet_decoders_never_panic_on_garbage() {
    let mut rng = SplitMix64(0x5EC0);
    for _ in 0..ROUNDS {
        let bytes = garbage(&mut rng);
        let _ = classify(&bytes);
        let _ = Hello::decode(&bytes);
        let _ = Challenge::decode(&bytes);
        let _ = HelloCookie::decode(&bytes);
        let _ = NoiseMsg::decode(&bytes, HsType::NoiseMsg2);
        let _ = NoiseMsg::decode(&bytes, HsType::NoiseMsg3);
        let _ = RejectReason::decode(&bytes);
    }
}

#[test]
fn the_server_survives_garbage_without_state_or_amplification() {
    let mut rng = SplitMix64(0x5E5E);
    let mut srv = server(0);
    let mut out = vec![0u8; MAX_DATAGRAM];
    let (mut received, mut sent) = (0usize, 0usize);
    for i in 0..ROUNDS {
        let bytes = garbage(&mut rng);
        let from = addr(1000 + (i % 50) as u16);
        received += bytes.len();
        if let ServerAction::Reply(n) = srv.handle(i, from, &bytes, &mut out) {
            sent += n;
        }
        assert_eq!(srv.half_open(), 0, "no slot without a valid cookie");
    }
    assert!(sent * 3 <= received, "replies to garbage stay small: {sent} sent for {received} received");
    assert_eq!(srv.stats().established, 0);
}

#[test]
fn the_server_survives_mutated_real_handshake_packets() {
    let mut rng = SplitMix64(0xC00C);
    let mut srv = server(0);
    let mut cli = client(addr(9000), 3);
    let (_, challenge) = client_to_server(&mut cli, &mut srv, addr(4000), 0);
    cli.handle(addr(9000), &challenge);
    let mut cookie = vec![0u8; MAX_DATAGRAM];
    let len = cli.poll_transmit(0, &mut cookie).expect("HelloCookie");
    cookie.truncate(len);
    let hello = Hello { proto_major: 1, client_nonce: [4; 16], client_conn_id: 3 }.encode();
    let mut out = vec![0u8; MAX_DATAGRAM];
    for i in 0..ROUNDS / 5 {
        let mut bytes = match i % 2 {
            0 => cookie.clone(),
            _ => hello.to_vec(),
        };
        flip(&mut rng, &mut bytes);
        let _ = srv.handle(1, addr(4000), &bytes, &mut out);
        srv.expire(10_000 + i);
    }
    assert_eq!(srv.stats().established, 0);
}

#[test]
fn the_client_survives_garbage_in_every_stage() {
    let mut rng = SplitMix64(0xC11E);
    let mut srv = server(0);
    for stage in 0..3 {
        let mut cli = client(addr(9000), 3);
        if stage >= 1 {
            let (_, challenge) = client_to_server(&mut cli, &mut srv, addr(4000), 0);
            cli.handle(addr(9000), &challenge);
        }
        if stage >= 2 {
            let (_, msg2) = client_to_server(&mut cli, &mut srv, addr(4000), 0);
            cli.handle(addr(9000), &msg2);
            assert_eq!(cli.status(), ClientStatus::Established);
        }
        for _ in 0..ROUNDS / 5 {
            let bytes = garbage(&mut rng);
            let status = cli.handle(addr(9000), &bytes);
            if let ClientStatus::Failed(_) = status {
                break;
            }
        }
    }
}

#[test]
fn a_sealed_connection_survives_garbage_and_opens_none_of_it() {
    let mut rng = SplitMix64(0x5EA1);
    let mut conn = Connection::with_sealer(
        ConnConfig { local_id: 2, remote_id: 1, ..ConnConfig::default() },
        NoiseSealer::new([1; 32], [2; 32], SealConfig::default()),
    );
    let mut sender = Connection::with_sealer(
        ConnConfig { local_id: 1, remote_id: 2, ..ConnConfig::default() },
        NoiseSealer::new([2; 32], [1; 32], SealConfig::default()),
    );
    let mut real = vec![0u8; MAX_DATAGRAM];
    let mut opened = 0;
    for i in 0..ROUNDS {
        let mut bytes = match i % 2 {
            0 => {
                let mut bytes = garbage(&mut rng);
                if let Some(head) = bytes.get_mut(..3) {
                    head.copy_from_slice(&[0, 2, 0]);
                }
                bytes
            }
            _ => {
                let msg_len = rng.below(40) as usize;
                let msg = random_bytes(&mut rng, msg_len);
                let sent = sender.write_packet(i, &[(0x40, &msg)], &mut real).expect("sealed");
                let mut bytes = real[..sent.len].to_vec();
                flip(&mut rng, &mut bytes);
                if bytes[..] == real[..sent.len] {
                    // Two flips of one bit cancel out; make sure the packet really differs.
                    bytes[sent.len - 1] ^= 1;
                }
                bytes
            }
        };
        if conn.read_packet(i, &mut bytes, |_, _| {}).is_ok() {
            opened += 1;
        }
        while conn.poll_event().is_some() {}
        while sender.poll_event().is_some() {}
    }
    assert_eq!(opened, 0, "garbage and flipped packets never open");
}

#[test]
fn tokens_and_pake_messages_never_panic_on_garbage() {
    let mut rng = SplitMix64(0x70CE);
    let mut ring = TokenKeyring::new();
    ring.install(TokenKey::new(1, [9; 32]));
    for _ in 0..ROUNDS / 10 {
        let len = match rng.below(2) {
            0 => crate::secure::token::TOKEN_LEN,
            _ => rng.below(600) as usize,
        };
        let mut bytes = random_bytes(&mut rng, len);
        if let Some(head) = bytes.get_mut(..5) {
            head.copy_from_slice(&[1, 1, 0, 0, 0]);
        }
        assert!(open_token(&ring, &bytes).is_err());
    }
    for _ in 0..2_000 {
        let (state, _) = start(b"pw", &[0; 32], PakeRole::Host).expect("start");
        let len = match rng.below(2) {
            0 => 33,
            _ => rng.below(80) as usize,
        };
        let peer = random_bytes(&mut rng, len);
        if let Ok(confirm) = state.finish(&peer) {
            assert!(!confirm.verify(&random_bytes(&mut rng, 32)));
        }
    }
}
