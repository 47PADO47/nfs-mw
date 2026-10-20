use blackbox_chunk::ids;

use crate::group::record_size;
use crate::{Error, SceneryGroup, override_group_numbers, parse_groups, read_scenery_groups};

/// One group record as the track bundle stores it: two junk words, the hash, the number, the count, the flag bytes,
/// then the override indexes, padded to 4 bytes.
fn record(hash: u32, number: u16, drive_through: u8, overrides: &[u16]) -> Vec<u8> {
    let mut r = vec![0u8; record_size(overrides.len())];
    r[0..4].copy_from_slice(&0x0B00_0000u32.to_le_bytes());
    r[4..8].copy_from_slice(&0x0B00_0000u32.to_le_bytes());
    r[0x08..0x0C].copy_from_slice(&hash.to_le_bytes());
    r[0x0C..0x0E].copy_from_slice(&number.to_le_bytes());
    r[0x0E..0x10].copy_from_slice(&(overrides.len() as u16).to_le_bytes());
    r[0x10] = 1;
    r[0x11] = drive_through;
    for (i, index) in overrides.iter().enumerate() {
        r[0x14 + i * 2..0x16 + i * 2].copy_from_slice(&index.to_le_bytes());
    }
    r
}

fn chunk(id: u32, payload: &[u8]) -> Vec<u8> {
    let mut v = id.to_le_bytes().to_vec();
    v.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    v.extend_from_slice(payload);
    v
}

#[test]
fn record_size_follows_the_spec_formula() {
    assert_eq!(record_size(0), 0x14);
    assert_eq!(record_size(3), 28);
    assert_eq!(record_size(5), 32);
    assert_eq!(record_size(7), 36);
}

#[test]
fn parses_records_back_to_back() {
    let mut payload = record(0xB112_B337, 624, 0, &[41, 40, 39]);
    payload.extend(record(0x454C_22D2, 1, 1, &[]));
    payload.extend(record(0xB112_B33A, 200, 0, &[14519, 14520, 14521, 14522, 14523]));

    let groups = parse_groups(&payload).unwrap();

    assert_eq!(groups.len(), 3);
    assert_eq!(
        groups[0],
        SceneryGroup {
            hash: 0xB112_B337,
            number: 624,
            flag: 1,
            drive_through: false,
            override_indexes: vec![41, 40, 39],
        }
    );
    assert_eq!(groups[1].number, 1);
    assert!(groups[1].drive_through);
    assert!(groups[1].override_indexes.is_empty());
    assert_eq!(groups[2].number, 200);
    assert_eq!(groups[2].override_indexes, [14519, 14520, 14521, 14522, 14523]);
}

#[test]
fn empty_payload_has_no_groups() {
    assert!(parse_groups(&[]).unwrap().is_empty());
}

#[test]
fn truncated_record_is_an_error() {
    let mut payload = record(1, 2, 0, &[5, 6, 7]);
    payload.truncate(payload.len() - 4);
    let err = parse_groups(&payload).unwrap_err();
    assert!(matches!(err, Error::Groups { offset: 0, .. }), "{err}");

    let short = parse_groups(&[0u8; 0x10]).unwrap_err();
    assert!(matches!(short, Error::Groups { offset: 0, .. }), "{short}");
}

#[test]
fn error_offset_points_at_the_bad_record() {
    let mut payload = record(1, 2, 0, &[]);
    let second = payload.len();
    payload.extend(record(3, 4, 0, &[9, 9, 9, 9]));
    payload.truncate(payload.len() - 2);
    let err = parse_groups(&payload).unwrap_err();
    assert!(matches!(err, Error::Groups { offset, .. } if offset == second), "{err}");
}

#[test]
fn bundle_without_the_chunk_has_no_groups() {
    let bundle = chunk(ids::SCENERY_PRECULLER_INFOS, &[0u8; 8]);
    assert!(read_scenery_groups(&bundle).unwrap().is_empty());
}

#[test]
fn reads_the_group_chunk_of_a_bundle() {
    let bundle = chunk(ids::SCENERY_GROUPS, &record(7, 356, 0, &[12244, 12243]));
    let groups = read_scenery_groups(&bundle).unwrap();
    assert_eq!(groups.len(), 1);
    assert_eq!(groups[0].number, 356);
    assert_eq!(groups[0].override_indexes, [12244, 12243]);
}

#[test]
fn override_numbers_skip_group_zero() {
    let groups = vec![
        SceneryGroup { hash: 0, number: 624, flag: 1, drive_through: false, override_indexes: vec![41, 40] },
        SceneryGroup { hash: 0, number: 0, flag: 1, drive_through: false, override_indexes: vec![99] },
        SceneryGroup { hash: 0, number: 389, flag: 1, drive_through: false, override_indexes: vec![13601] },
    ];
    let map = override_group_numbers(&groups);
    assert_eq!(map.len(), 3);
    assert_eq!(map.get(&41), Some(&624));
    assert_eq!(map.get(&40), Some(&624));
    assert_eq!(map.get(&13601), Some(&389));
    assert_eq!(map.get(&99), None);
}

#[test]
fn a_group_is_found_by_its_name_hash() {
    let groups = parse_groups(&[record(0x2D9C_192E, 2, 0, &[5]), record(0x87DE_8337, 71, 0, &[7])].concat()).unwrap();
    assert_eq!(crate::find_group(&groups, 0x87DE_8337).map(|g| g.number), Some(71));
    assert!(crate::find_group(&groups, 0x1234_5678).is_none());
}
