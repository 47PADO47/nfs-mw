//! Track path zones from hand-built bytes.

use blackbox_chunk::ids;
use glam::{Vec2, Vec3};

use crate::{TrackZones, from_zone_space, to_zone_space};

fn chunk(id: u32, payload: &[u8]) -> Vec<u8> {
    let mut v = id.to_le_bytes().to_vec();
    v.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    v.extend_from_slice(payload);
    v
}

/// One zone record: the polygon's bounding box is computed from its points.
fn zone(kind: u32, data0: i32, polygon: &[(f32, f32)]) -> Vec<u8> {
    let min = polygon.iter().fold((f32::MAX, f32::MAX), |a, p| (a.0.min(p.0), a.1.min(p.1)));
    let max = polygon.iter().fold((f32::MIN, f32::MIN), |a, p| (a.0.max(p.0), a.1.max(p.1)));
    let mut v = kind.to_le_bytes().to_vec();
    for f in [min.0, min.1, 1.0, 0.0, 2.5] {
        v.extend_from_slice(&f.to_le_bytes());
    }
    v.extend_from_slice(&[0; 0x20 - 0x18]); // source, runtime fields
    for f in [min.0, min.1, max.0, max.1] {
        v.extend_from_slice(&f.to_le_bytes());
    }
    for d in [data0, 0, 0, 0] {
        v.extend_from_slice(&d.to_le_bytes());
    }
    v.extend_from_slice(&(polygon.len() as i16).to_le_bytes());
    v.extend_from_slice(&((0x44 + 8 * polygon.len()) as i16).to_le_bytes());
    for p in polygon {
        v.extend_from_slice(&p.0.to_le_bytes());
        v.extend_from_slice(&p.1.to_le_bytes());
    }
    v
}

const SQUARE: [(f32, f32); 4] = [(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)];
/// A concave L: the notch (5..10, 5..10) is outside.
const L_SHAPE: [(f32, f32); 6] = [(0.0, 0.0), (10.0, 0.0), (10.0, 5.0), (5.0, 5.0), (5.0, 10.0), (0.0, 10.0)];

fn file(records: &[Vec<u8>]) -> Vec<u8> {
    let payload: Vec<u8> = records.concat();
    let zones = chunk(ids::TRACK_PATH_ZONES, &payload);
    let other = chunk(0x0003_414D, &[0; 24]);
    chunk(ids::TRACK_PATH_MANAGER, &[other, zones].concat())
}

#[test]
fn the_records_are_read_one_after_the_other() {
    let data = file(&[zone(9, 77, &SQUARE), zone(3, -1, &L_SHAPE)]);
    let zones = TrackZones::read(&data).unwrap().unwrap();
    assert_eq!(zones.zones.len(), 2);
    let first = &zones.zones[0];
    assert_eq!((first.kind, first.data, first.elevation), (9, [77, 0, 0, 0], 2.5));
    assert_eq!((first.bbox_min, first.bbox_max), (Vec2::ZERO, Vec2::splat(10.0)));
    assert_eq!(first.polygon.len(), 4);
    assert_eq!(zones.zones[1].data[0], -1);
    assert_eq!(zones.zones[1].polygon.len(), 6);
}

#[test]
fn a_file_without_zones_has_none() {
    assert_eq!(TrackZones::read(&chunk(0x0003_4110, &[0; 8])).unwrap(), None);
    assert_eq!(TrackZones::read(&chunk(ids::TRACK_PATH_MANAGER, &[])).unwrap(), None);
}

#[test]
fn a_record_with_a_wrong_size_is_refused() {
    let mut bad = zone(9, 0, &SQUARE);
    bad[0x42] += 8;
    assert!(TrackZones::read(&file(&[bad])).is_err());
    let truncated = zone(9, 0, &SQUARE)[..0x50].to_vec();
    assert!(TrackZones::parse(&truncated).is_err());
}

#[test]
fn points_are_tested_against_the_polygon() {
    let zones = TrackZones::parse(&zone(9, 0, &L_SHAPE)).unwrap();
    let z = &zones.zones[0];
    assert!(z.contains(Vec2::new(2.0, 2.0)));
    assert!(z.contains(Vec2::new(8.0, 2.0)));
    assert!(z.contains(Vec2::new(2.0, 8.0)));
    assert!(!z.contains(Vec2::new(8.0, 8.0)), "in the notch of the L");
    assert!(!z.contains(Vec2::new(-1.0, 2.0)), "outside the bounding box");
    assert!(!z.contains(Vec2::new(12.0, 2.0)));
}

#[test]
fn the_first_zone_of_the_kind_in_file_order_wins() {
    let bytes = [zone(9, 1, &SQUARE), zone(4, 2, &SQUARE), zone(9, 3, &SQUARE), zone(9, 4, &L_SHAPE)].concat();
    let zones = TrackZones::parse(&bytes).unwrap();
    let inside = Vec2::new(2.0, 2.0);
    assert_eq!(zones.first_of_kind_at(9, inside).map(|z| z.data[0]), Some(1));
    assert_eq!(zones.first_of_kind_at(4, inside).map(|z| z.data[0]), Some(2));
    assert_eq!(zones.first_of_kind_at(5, inside), None);
    let all: Vec<i32> = zones.contains_point(9, inside).iter().map(|z| z.data[0]).collect();
    assert_eq!(all, [1, 3, 4]);
    assert!(zones.contains_point(9, Vec2::new(50.0, 50.0)).is_empty());
}

#[test]
fn the_frame_swaps_physics_axes() {
    // Physics x = -y2d and z = x2d.
    assert_eq!(to_zone_space(Vec3::new(-7.0, 99.0, 3.0)), Vec2::new(3.0, 7.0));
    assert_eq!(from_zone_space(Vec2::new(3.0, 7.0), 5.0), Vec3::new(-7.0, 5.0, 3.0));
    let p = Vec3::new(12.0, 0.0, -4.0);
    assert_eq!(from_zone_space(to_zone_space(p), 0.0), p);
}
