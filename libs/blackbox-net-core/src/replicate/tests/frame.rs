//! Frame order, value checks and lookups.

use super::{fixed_schema, random_schema, random_values};
use crate::replicate::{Frame, KindId, ReplicateError};
use crate::rng::SplitMix64;

#[test]
fn push_checks_order_kind_count_and_range() {
    let schema = fixed_schema();
    let mut frame = Frame::new();
    assert_eq!(frame.push(&schema, 5, KindId(0), &[1, 2, 3]), Ok(()));
    assert_eq!(frame.push(&schema, 5, KindId(0), &[1, 2, 3]), Err(ReplicateError::BadOrder));
    assert_eq!(frame.push(&schema, 4, KindId(0), &[1, 2, 3]), Err(ReplicateError::BadOrder));
    assert_eq!(frame.push(&schema, 6, KindId(9), &[]), Err(ReplicateError::UnknownKind));
    assert_eq!(frame.push(&schema, 6, KindId(0), &[1, 2]), Err(ReplicateError::BadSchema));
    assert_eq!(frame.push(&schema, 6, KindId(0), &[4096, 0, 0]), Err(ReplicateError::BadValue));
    assert_eq!(frame.push(&schema, 6, KindId(0), &[0, 128, 0]), Err(ReplicateError::BadValue));
    assert_eq!(frame.push(&schema, 6, KindId(1), &[2]), Err(ReplicateError::BadValue));
    assert_eq!(frame.len(), 1);
    assert_eq!(frame.push(&schema, 6, KindId(0), &[4095, 127, u32::MAX]), Ok(()));
    assert_eq!(frame.len(), 2);
}

#[test]
fn get_and_iter_follow_id_order() {
    let schema = fixed_schema();
    let mut frame = Frame::new();
    for (id, kind) in [(2u16, KindId(0)), (9, KindId(1)), (40, KindId(0))] {
        let values = match kind.0 {
            0 => vec![7, 1, 99],
            _ => vec![1],
        };
        assert_eq!(frame.push(&schema, id, kind, &values), Ok(()));
    }
    assert_eq!(frame.get(9), Some((KindId(1), &[1u32][..])));
    assert_eq!(frame.get(40), Some((KindId(0), &[7u32, 1, 99][..])));
    assert_eq!(frame.get(10), None);
    let ids: Vec<u16> = frame.iter().map(|(id, _, _)| id).collect();
    assert_eq!(ids, vec![2, 9, 40]);
    assert!(!frame.is_empty());
}

#[test]
fn lookups_agree_with_a_linear_scan_on_random_frames() {
    let mut rng = SplitMix64(3);
    let schema = random_schema(&mut rng);
    let mut frame = Frame::new();
    frame.clear(1);
    let mut present = Vec::new();
    for id in (0..1000u16).step_by(2) {
        if rng.below(3) == 0 {
            continue;
        }
        let kind = KindId(rng.below(schema.kind_count() as u64) as u8);
        let values = random_values(&schema, kind, &mut rng);
        assert_eq!(frame.push(&schema, id, kind, &values), Ok(()));
        present.push((id, kind, values));
    }
    for id in 0..1001u16 {
        let expected =
            present.iter().find(|(pid, _, _)| *pid == id).map(|(_, kind, values)| (*kind, values.as_slice()));
        assert_eq!(frame.get(id), expected);
    }
    assert_eq!(frame.len(), present.len());
}

#[test]
fn clear_empties_the_frame_and_sets_the_tick() {
    let schema = fixed_schema();
    let mut frame = Frame::new();
    assert_eq!(frame.tick(), 0);
    assert!(frame.is_empty());
    assert_eq!(frame.push(&schema, 1, KindId(1), &[0]), Ok(()));
    frame.clear(7);
    assert_eq!(frame.tick(), 7);
    assert_eq!(frame.len(), 0);
    assert_eq!(frame.get(1), None);
    assert_eq!(frame.push(&schema, 0, KindId(1), &[1]), Ok(()));
    assert_eq!(frame.get(0), Some((KindId(1), &[1u32][..])));
}
