//! Clips and lists built byte by byte.

use crate::quat::angle_degrees;
use crate::{AnimationList, Clip, Error};

const IDENTITY: u32 = 0xDFF7_FDFF;
const TURNED: u32 = 0xEBD7_FDFF;

fn put(bytes: &mut [u8], at: usize, value: u32) {
    bytes[at..at + 4].copy_from_slice(&value.to_le_bytes());
}

/// A clip of 3 keys at 30 a second over 4 bones: a rotation track on bone 2 (identity, turned, identity) and a vector track
/// on bone 1 (a slide and a position per key).
fn clip(id: u32) -> Vec<u8> {
    let mut bytes = vec![0u8; 0x200];
    put(&mut bytes, 0x04, 3);
    put(&mut bytes, 0x08, 4);
    put(&mut bytes, 0x0C, 1);
    put(&mut bytes, 0x10, 1);
    put(&mut bytes, 0x18, 1);
    put(&mut bytes, 0x1C, 4);
    put(&mut bytes, 0x20, 16);
    put(&mut bytes, 0x28, 16);
    put(&mut bytes, 0x30, 30.0f32.to_bits());
    bytes[0x34..0x36].copy_from_slice(&1u16.to_le_bytes());
    bytes[0x36..0x38].copy_from_slice(&2u16.to_le_bytes());
    // Tables: slides 0x80, positions 0xB0, slide map 0xE0, rotations 0xF0, rotation map 0x100.
    for (table, at) in [(1, 0x80), (2, 0xB0), (4, 0xE0), (5, 0xF0), (7, 0x100)] {
        put(&mut bytes, 0x38 + 4 * table, at);
    }
    put(&mut bytes, 0x5C, id);
    for key in 0..3u32 {
        let slide = 0x80 + 16 * key as usize;
        put(&mut bytes, slide, (0.01 * key as f32).to_bits());
        let position = 0xB0 + 16 * key as usize;
        put(&mut bytes, position, (1.0 + key as f32).to_bits());
        put(&mut bytes, position + 4, 2.0f32.to_bits());
        put(&mut bytes, position + 8, 3.0f32.to_bits());
        put(&mut bytes, 0xF0 + 4 * key as usize, if key == 1 { TURNED } else { IDENTITY });
    }
    bytes[0xE0..0xE4].fill(0xFF);
    bytes[0xE1] = 0;
    bytes[0x100..0x104].fill(0xFF);
    bytes[0x102] = 0;
    bytes
}

#[test]
fn a_clip_reads_its_tracks_and_rate() {
    let clip = Clip::parse(&clip(0x1779c7)).unwrap();
    assert_eq!((clip.id, clip.keys, clip.bones, clip.reference), (0x1779c7, 3, 4, 2));
    assert_eq!(clip.rate, 30.0);
    assert!((clip.seconds() - 2.0 / 30.0).abs() < 1e-6);
    assert_eq!(clip.rotation_track, [None, None, Some(0), None]);
    assert_eq!(clip.slide_track, [None, Some(0), None, None]);
    assert_eq!(clip.positions[2][0], [3.0, 2.0, 3.0]);
}

#[test]
fn a_bone_turns_between_its_keys_and_a_bone_without_a_track_does_not() {
    let clip = Clip::parse(&clip(1)).unwrap();
    assert!(angle_degrees(clip.rotation(2, 0.0).unwrap()) < 0.5);
    assert!((angle_degrees(clip.rotation(2, 1.0 / 30.0).unwrap()) - 30.46).abs() < 0.1);
    let halfway = angle_degrees(clip.rotation(2, 0.5 / 30.0).unwrap());
    assert!((halfway - 15.2).abs() < 0.3, "{halfway}");
    // After the last key the clip holds.
    assert!(angle_degrees(clip.rotation(2, 10.0).unwrap()) < 0.5);
    assert_eq!(clip.rotation(0, 0.0), None);
    assert_eq!(clip.rotation(9, 0.0), None, "no such bone");
}

#[test]
fn a_position_track_blends_between_keys() {
    let clip = Clip::parse(&clip(1)).unwrap();
    let at = clip.position(1, 0.5 / 30.0).unwrap();
    assert!((at[0] - 1.5).abs() < 1e-5, "{at:?}");
    assert_eq!(clip.position(2, 0.0), None);
}

#[test]
fn a_clip_in_another_rotation_format_or_cut_short_is_refused() {
    let mut other = clip(1);
    other[0x34] = 2;
    assert!(matches!(Clip::parse(&other), Err(Error::Invalid { .. })));
    assert!(matches!(Clip::parse(&clip(1)[..0x90]), Err(Error::Truncated { .. })));
    assert!(Clip::parse(&[0u8; 0x80]).is_err(), "no keys");
}

#[test]
fn a_list_splits_its_clips_by_their_offsets() {
    let (first, second) = (clip(11), clip(22));
    let table = 0x20;
    let first_at = table + 8;
    let second_at = first_at + first.len();
    let total = second_at + second.len();
    let mut list = vec![0u8; first_at];
    list[2..4].copy_from_slice(&2u16.to_le_bytes());
    put(&mut list, 4, total as u32);
    put(&mut list, 8, table as u32);
    put(&mut list, table, first_at as u32);
    put(&mut list, table + 4, second_at as u32);
    list.extend_from_slice(&first);
    list.extend_from_slice(&second);
    let parsed = AnimationList::parse(&list).unwrap();
    assert_eq!(parsed.clips.len(), 2);
    assert_eq!(parsed.by_id(22).map(|c| c.keys), Some(3));
    assert_eq!(parsed.by_id(11).map(|c| c.id), Some(11));
    assert!(parsed.by_id(33).is_none());
}
