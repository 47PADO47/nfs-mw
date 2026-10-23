//! Sealed connections: tamper and replay rejection, rekeying by count and by time, the old key's grace period, and
//! the nonce guard.

use crate::MAX_DATAGRAM;
use crate::conn::{ConnConfig, ConnError, Connection, FRAME_SPACE, Sealer};
use crate::secure::sealer::expand;
use crate::secure::{NoiseSealer, SealConfig, TAG_LEN};

fn keys() -> ([u8; 32], [u8; 32]) {
    ([0x11; 32], [0x22; 32])
}

/// Two sealed connections with matching keys and `config` on both sides.
fn pair(config: SealConfig) -> (Connection<NoiseSealer>, Connection<NoiseSealer>) {
    let (ab, ba) = keys();
    let a = Connection::with_sealer(
        ConnConfig { local_id: 1, remote_id: 2, ..ConnConfig::default() },
        NoiseSealer::new(ab, ba, config),
    );
    let b = Connection::with_sealer(
        ConnConfig { local_id: 2, remote_id: 1, ..ConnConfig::default() },
        NoiseSealer::new(ba, ab, config),
    );
    (a, b)
}

fn packet(conn: &mut Connection<NoiseSealer>, now: u64, frames: &[(u8, &[u8])]) -> Vec<u8> {
    let mut out = vec![0u8; MAX_DATAGRAM];
    let sent = conn.write_packet(now, frames, &mut out).expect("sealed");
    out.truncate(sent.len);
    out
}

fn read(conn: &mut Connection<NoiseSealer>, now: u64, datagram: &[u8]) -> Result<Vec<(u8, Vec<u8>)>, ConnError> {
    let mut copy = datagram.to_vec();
    let mut frames = Vec::new();
    conn.read_packet(now, &mut copy, |kind, payload| frames.push((kind, payload.to_vec())))?;
    Ok(frames)
}

#[test]
fn a_sealed_packet_hides_everything_after_the_clear_header() {
    let (mut a, mut b) = pair(SealConfig::default());
    let datagram = packet(&mut a, 0, &[(0x40, b"secret position")]);
    assert_eq!(datagram.len(), 15 + 2 + 15 + TAG_LEN);
    assert!(!datagram.windows(6).any(|w| w == b"secret"), "the frames are encrypted");
    assert_eq!(&datagram[1..3], &2u16.to_le_bytes(), "the connection id stays readable");
    assert_eq!(read(&mut b, 0, &datagram), Ok(vec![(0x40, b"secret position".to_vec())]));
}

#[test]
fn a_full_packet_with_its_tag_fits_in_one_datagram() {
    let (mut a, mut b) = pair(SealConfig::default());
    let payload = vec![7u8; FRAME_SPACE - 3];
    let datagram = packet(&mut a, 0, &[(0x40, &payload)]);
    assert_eq!(datagram.len(), MAX_DATAGRAM);
    assert!(read(&mut b, 0, &datagram).is_ok());
}

#[test]
fn every_single_bit_flip_is_refused_and_changes_nothing() {
    let (mut a, mut b) = pair(SealConfig::default());
    let datagram = packet(&mut a, 0, &[(0x40, b"hello")]);
    for byte in 0..datagram.len() {
        for bit in 0..8 {
            let mut forged = datagram.clone();
            forged[byte] ^= 1 << bit;
            assert!(read(&mut b, 0, &forged).is_err(), "flip of byte {byte} bit {bit} accepted");
        }
    }
    assert!(b.sealer().stats().bad_tags > 0);
    assert_eq!(read(&mut b, 0, &datagram), Ok(vec![(0x40, b"hello".to_vec())]), "the real packet still opens");
}

#[test]
fn a_replayed_packet_is_a_duplicate_and_an_ancient_one_too_old() {
    let (mut a, mut b) = pair(SealConfig::default());
    let first = packet(&mut a, 0, &[]);
    assert!(read(&mut b, 0, &first).is_ok());
    assert_eq!(read(&mut b, 1, &first), Err(ConnError::Duplicate));
    for now in 1..=1100 {
        let next = packet(&mut a, now, &[]);
        assert!(read(&mut b, now, &next).is_ok());
    }
    assert_eq!(read(&mut b, 2000, &first), Err(ConnError::TooOld));
}

#[test]
fn a_packet_for_the_other_direction_does_not_open() {
    let (mut a, _) = pair(SealConfig::default());
    let datagram = packet(&mut a, 0, &[]);
    let (ab, ba) = keys();
    // The sender's own keys: it receives under the other direction's key, so its own packets do not open.
    let mut wrong = Connection::with_sealer(
        ConnConfig { local_id: 2, remote_id: 1, ..ConnConfig::default() },
        NoiseSealer::new(ab, ba, SealConfig::default()),
    );
    assert_eq!(read(&mut wrong, 0, &datagram), Err(ConnError::BadTag));
}

#[test]
fn rekeying_by_count_keeps_every_packet_flowing() {
    let config = SealConfig { rekey_packets: 10, ..SealConfig::default() };
    let (mut a, mut b) = pair(config);
    for now in 0..100u64 {
        let datagram = packet(&mut a, now, &[(0x40, &now.to_le_bytes())]);
        assert!(read(&mut b, now, &datagram).is_ok(), "packet {now}");
        let reply = packet(&mut b, now, &[]);
        assert!(read(&mut a, now, &reply).is_ok(), "reply {now}");
    }
    assert_eq!(a.sealer().stats().rekeys_sent, 9);
    assert_eq!(b.sealer().stats().rekeys_received, 9);
    assert_eq!(a.sealer().send_phase(), 1, "nine phases later, phase 9 mod 4");
}

#[test]
fn rekeying_by_time_happens_after_the_interval() {
    let config = SealConfig { rekey_ms: 1_000, ..SealConfig::default() };
    let (mut a, mut b) = pair(config);
    let early = packet(&mut a, 0, &[]);
    assert!(read(&mut b, 0, &early).is_ok());
    let late = packet(&mut a, 1_000, &[]);
    assert_eq!(a.sealer().send_phase(), 1);
    assert!(read(&mut b, 1_000, &late).is_ok());
    assert_eq!(b.sealer().recv_phase(), 1);
}

#[test]
fn the_old_key_opens_reordered_packets_for_three_seconds_only() {
    let config = SealConfig { rekey_packets: 4, ..SealConfig::default() };
    let (mut a, mut b) = pair(config);
    let old: Vec<Vec<u8>> = (0..4).map(|i| packet(&mut a, i, &[])).collect();
    let new: Vec<Vec<u8>> = (4..8).map(|i| packet(&mut a, i, &[])).collect();
    assert!(read(&mut b, 10, &new[0]).is_ok(), "the next phase is followed");
    assert!(read(&mut b, 1_000, &old[0]).is_ok(), "a reordered old packet within 3 s");
    assert!(read(&mut b, 3_009, &old[1]).is_ok(), "still within 3 s of the switch");
    assert_eq!(read(&mut b, 3_010, &old[2]), Err(ConnError::BadTag), "the old key is erased");
    assert_eq!(b.sealer().stats().unknown_phase, 1);
    assert!(read(&mut b, 3_010, &new[1]).is_ok());
}

#[test]
fn a_forged_next_phase_packet_does_not_move_the_key() {
    let (mut a, mut b) = pair(SealConfig::default());
    let mut forged = packet(&mut a, 0, &[]);
    forged[0] |= 1 << 5;
    assert_eq!(read(&mut b, 0, &forged), Err(ConnError::BadTag));
    assert_eq!(b.sealer().recv_phase(), 0);
    let two_ahead = {
        let mut p = packet(&mut a, 1, &[]);
        p[0] |= 2 << 5;
        p
    };
    assert_eq!(read(&mut b, 1, &two_ahead), Err(ConnError::BadTag));
    assert_eq!(b.sealer().stats().unknown_phase, 1);
}

#[test]
fn the_sealer_never_seals_one_number_twice() {
    let (ab, ba) = keys();
    let mut sealer = NoiseSealer::new(ab, ba, SealConfig::default());
    let mut buf = [0u8; 64];
    buf[3..7].copy_from_slice(&5u32.to_le_bytes());
    let plain = buf;
    assert_eq!(sealer.seal(0, &mut buf, 20), Ok(20 + TAG_LEN));
    let mut again = plain;
    assert_eq!(sealer.seal(0, &mut again, 20), Err(ConnError::SealFailed), "nonce reuse refused");
    let mut short = [0u8; 20];
    assert_eq!(sealer.seal(0, &mut short, 20), Err(ConnError::SealFailed), "no room for the tag");
}

#[test]
fn packet_numbers_expand_across_the_32_bit_wrap() {
    assert_eq!(expand(0, 0), 0);
    assert_eq!(expand(5, 3), 3);
    assert_eq!(expand(0xFFFF_FFFF, 0), 0x1_0000_0000);
    assert_eq!(expand(0x1_0000_0002, 0xFFFF_FFF0), 0xFFFF_FFF0);
    assert_eq!(expand(0x2_0000_0000, 0x10), 0x2_0000_0010);
    assert_eq!(expand(0x1_8000_0000, 0x7FFF_FFFF), 0x1_7FFF_FFFF, "the nearer of the two candidates");
    assert_eq!(expand(0x1_8000_0001, 0), 0x2_0000_0000);
}

#[test]
fn a_sealed_connection_keeps_working_past_the_32_bit_wrap() {
    let (ab, ba) = keys();
    let mut tx = NoiseSealer::new(ab, ba, SealConfig::default());
    let mut rx = NoiseSealer::new(ba, ab, SealConfig::default());
    for number in [0xFFFF_FFFDu32, 0xFFFF_FFFE, 0xFFFF_FFFF, 0, 1, 2] {
        let mut buf = [0u8; 64];
        buf[1..3].copy_from_slice(&1u16.to_le_bytes());
        buf[3..7].copy_from_slice(&number.to_le_bytes());
        buf[7..20].copy_from_slice(b"payload bytes");
        let len = tx.seal(0, &mut buf, 20).expect("sealed");
        assert_eq!(rx.open(0, &mut buf[..len]), Ok(20), "number {number:#x}");
        assert_eq!(&buf[7..20], b"payload bytes");
    }
}
