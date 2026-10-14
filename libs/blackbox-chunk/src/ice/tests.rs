use super::*;
use crate::ids::{ICE_NIS_CAMERAS, ICE_SHAKE_TRACKS};
use crate::{Chunk, chunks};

/// Wraps a payload in an 8-byte chunk header.
fn chunk_bytes(id: u32, payload: &[u8]) -> Vec<u8> {
    let mut v = id.to_le_bytes().to_vec();
    v.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    v.extend_from_slice(payload);
    v
}

fn first_chunk(data: &[u8]) -> Chunk<'_> {
    chunks(data).next().unwrap().unwrap()
}

/// A 0x28-byte camera header with the given element count at +0x14.
fn camera_header(count: i16) -> Vec<u8> {
    let mut h = vec![0u8; CAMERA_RECORD_HEADER_LEN];
    h[0x14..0x16].copy_from_slice(&count.to_le_bytes());
    h
}

/// One 0x84-byte camera element: flag at +0, time key at +0x10.
fn camera_element(enabled: bool, time: f32) -> Vec<u8> {
    let mut e = vec![0u8; CAMERA_ELEMENT_LEN];
    e[0] = u8::from(enabled);
    e[0x10..0x14].copy_from_slice(&time.to_le_bytes());
    e
}

/// A camera set: key, record count, then the records.
fn camera_set(key: u32, records: &[Vec<u8>]) -> Vec<u8> {
    let mut v = key.to_le_bytes().to_vec();
    v.extend_from_slice(&(records.len() as u32).to_le_bytes());
    for r in records {
        v.extend_from_slice(r);
    }
    v
}

#[test]
fn camera_pack_reads_sets_records_and_elements() {
    let mut rec = camera_header(2);
    rec.extend(camera_element(true, 0.0));
    rec.extend(camera_element(false, 1.5));
    let sets =
        [chunk_bytes(0x0003_0001, &camera_set(0x0ABC, &[rec])), chunk_bytes(0x0003_0001, &camera_set(7, &[]))].concat();
    let data = chunk_bytes(ICE_NIS_CAMERAS, &sets);
    let pack = parse_camera_pack(&first_chunk(&data)).unwrap();
    assert_eq!(pack.kind, 0);
    assert_eq!(pack.sets.len(), 2);
    assert_eq!(pack.sets[0].key, 0x0ABC);
    assert!(pack.sets[1].records.is_empty());
    let record = &pack.sets[0].records[0];
    assert_eq!(record.header.len(), CAMERA_RECORD_HEADER_LEN);
    assert_eq!(record.elements.len(), 2);
    assert_eq!(element_enabled(record.elements[0]), Some(true));
    assert_eq!(element_enabled(record.elements[1]), Some(false));
    assert_eq!(element_time(record.elements[1]), Some(1.5));
}

#[test]
fn camera_pack_kind_follows_id() {
    let data = chunk_bytes(ICE_NIS_CAMERAS + 3, &[]);
    let pack = parse_camera_pack(&first_chunk(&data)).unwrap();
    assert_eq!(pack.kind, 3);
    assert!(pack.sets.is_empty());
}

#[test]
fn camera_pack_rejects_other_ids() {
    let data = chunk_bytes(ICE_SHAKE_TRACKS, &[]);
    assert_eq!(parse_camera_pack(&first_chunk(&data)).unwrap_err(), IceError::WrongId(ICE_SHAKE_TRACKS));
}

#[test]
fn camera_truncated_element_is_error() {
    let rec = camera_header(3);
    let data = chunk_bytes(ICE_NIS_CAMERAS, &chunk_bytes(0x0003_0001, &camera_set(1, &[rec])));
    assert!(matches!(parse_camera_pack(&first_chunk(&data)), Err(IceError::Truncated(_))));
}

#[test]
fn camera_negative_count_is_error() {
    let rec = camera_header(-1);
    let data = chunk_bytes(ICE_NIS_CAMERAS, &chunk_bytes(0x0003_0001, &camera_set(1, &[rec])));
    assert!(matches!(parse_camera_pack(&first_chunk(&data)), Err(IceError::NegativeCount(_, -1))));
}

#[test]
fn shake_tracks_read_records_and_elements() {
    let mut a = vec![0u8; SHAKE_RECORD_HEADER_LEN];
    a[0xC..0xE].copy_from_slice(&1i16.to_le_bytes());
    a.extend(vec![0xAA; SHAKE_ELEMENT_LEN]);
    let mut b = vec![0u8; SHAKE_RECORD_HEADER_LEN];
    b[0xC..0xE].copy_from_slice(&0i16.to_le_bytes());
    let mut payload = 2u32.to_le_bytes().to_vec();
    payload.extend(a);
    payload.extend(b);
    let data = chunk_bytes(ICE_SHAKE_TRACKS, &payload);
    let tracks = parse_shake_tracks(&first_chunk(&data)).unwrap();
    assert_eq!(tracks.records.len(), 2);
    assert_eq!(tracks.records[0].header.len(), SHAKE_RECORD_HEADER_LEN);
    assert_eq!(tracks.records[0].elements, vec![&[0xAAu8; SHAKE_ELEMENT_LEN][..]]);
    assert!(tracks.records[1].elements.is_empty());
}

#[test]
fn shake_rejects_other_ids_and_truncation() {
    let data = chunk_bytes(ICE_NIS_CAMERAS, &[]);
    assert!(matches!(parse_shake_tracks(&first_chunk(&data)), Err(IceError::WrongId(_))));
    let data = chunk_bytes(ICE_SHAKE_TRACKS, &1u32.to_le_bytes());
    assert!(matches!(parse_shake_tracks(&first_chunk(&data)), Err(IceError::Truncated(_))));
}
