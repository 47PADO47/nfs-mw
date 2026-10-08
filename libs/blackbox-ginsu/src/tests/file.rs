//! Parsing the header and tables of a `.gin` built in memory.

use super::chirp::standard;
use crate::{Error, GinsuTables, HEADER_LEN};

/// Serialises tables the way a file stores them, followed by `payload` bytes.
fn file_bytes(t: &GinsuTables, payload: usize) -> Vec<u8> {
    let mut b = Vec::new();
    b.extend_from_slice(b"Gnsu20\0\0");
    b.extend_from_slice(&t.min_frequency().to_le_bytes());
    b.extend_from_slice(&t.max_frequency().to_le_bytes());
    b.extend_from_slice(&(t.seg_count() as u32).to_le_bytes());
    b.extend_from_slice(&(t.cycle_count() as u32).to_le_bytes());
    b.extend_from_slice(&t.sample_count().to_le_bytes());
    b.extend_from_slice(&t.sample_rate().to_le_bytes());
    for v in t.freq_pos().iter().chain(t.cycle_pos()) {
        b.extend_from_slice(&v.to_le_bytes());
    }
    b.resize(b.len() + payload, 0xAB);
    b
}

#[test]
fn a_file_round_trips() {
    let chirp = standard();
    let t = chirp.data.tables();
    let bytes = file_bytes(t, t.xas_payload_len());
    let (parsed, payload_at) = GinsuTables::parse(&bytes).unwrap();
    assert_eq!(&parsed, t);
    assert_eq!(payload_at, HEADER_LEN + 4 * (51 + t.cycle_count() + 1));
    assert_eq!(bytes.len() - payload_at, t.xas_payload_len());
}

#[test]
fn the_xas_payload_is_19_bytes_per_32_samples() {
    let t = GinsuTables::new(1.0, 2.0, 24_000, 33, vec![0], vec![0, 10]).unwrap();
    assert_eq!(t.xas_payload_len(), 2 * 0x13);
    let t = GinsuTables::new(1.0, 2.0, 24_000, 64, vec![0], vec![0, 10]).unwrap();
    assert_eq!(t.xas_payload_len(), 2 * 0x13);
}

#[test]
fn damaged_files_are_refused() {
    let chirp = standard();
    let good = file_bytes(chirp.data.tables(), 0);
    assert!(matches!(GinsuTables::parse(&good[..10]), Err(Error::Truncated { .. })));
    assert!(matches!(GinsuTables::parse(&good[..good.len() - 1]), Err(Error::Truncated { .. })));

    let mut bad = good.clone();
    bad[0] = b'X';
    assert_eq!(GinsuTables::parse(&bad), Err(Error::BadMagic));

    let mut bad = good.clone();
    bad[4] = b'3';
    assert_eq!(GinsuTables::parse(&bad), Err(Error::UnsupportedVersion(b'3')));

    // A cycle count far beyond the data must not allocate or panic.
    let mut bad = good.clone();
    bad[0x14..0x18].copy_from_slice(&u32::MAX.to_le_bytes());
    assert!(matches!(GinsuTables::parse(&bad), Err(Error::Truncated { .. })));

    // Cycle positions that do not increase.
    let mut bad = good.clone();
    let cycle_table = HEADER_LEN + 4 * 51;
    bad[cycle_table + 8..cycle_table + 12].copy_from_slice(&0u32.to_le_bytes());
    assert!(matches!(GinsuTables::parse(&bad), Err(Error::InvalidTables(_))));
}
