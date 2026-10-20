use glam::Vec2;

use super::*;
use crate::tests::synth::straight_road;

fn record(a: [f32; 2], b: [f32; 2], bytes: [u8; 4], hash: u32) -> Vec<u8> {
    let mut r = Vec::new();
    for v in [a[0], a[1], b[0], b[1]] {
        r.extend_from_slice(&v.to_le_bytes());
    }
    r.extend_from_slice(&bytes);
    r.extend_from_slice(&hash.to_le_bytes());
    r
}

#[test]
fn records_are_two_points_four_bytes_and_a_hash() {
    let payload = [
        record([2778.0, 624.2], [2779.4, 596.0], [0, 0, 0, 0], 0xB112_B337),
        record([1.0, 2.0], [3.0, 4.0], [1, 0, 1, 1], 7),
    ]
    .concat();
    let barriers = parse_track_barriers(&payload).unwrap();
    assert_eq!(barriers.len(), 2);
    assert_eq!(barriers[0].a, Vec2::new(2778.0, 624.2));
    assert_eq!(barriers[0].b, Vec2::new(2779.4, 596.0));
    assert!(!barriers[0].enabled && !barriers[0].drive_through && !barriers[0].left_handed);
    assert_eq!(barriers[0].group_hash, 0xB112_B337);
    assert!(barriers[1].enabled && barriers[1].drive_through && barriers[1].left_handed);
    assert!(parse_track_barriers(&[0; 25]).is_err());
}

/// The straight road runs along physics +z from 0 to 100: in the 2D frame from (0, 0) to (100, 0).
fn across(x: f32, drive_through: bool) -> ActiveBarrier {
    ActiveBarrier { a: Vec2::new(x, -10.0), b: Vec2::new(x, 10.0), drive_through }
}

#[test]
fn a_barrier_across_a_road_flags_its_segment() {
    let mut net = straight_road();
    let index = SegmentIndex::build(&net);
    assert_eq!(mark_barrier_crossings(&mut net, &index, &[across(50.0, false)], &[]), 1);
    assert!(net.segments[0].has(flags::CROSSES_BARRIER));
    assert!(!net.segments[0].has(flags::CROSSES_DRIVE_THROUGH_BARRIER));

    // A drive-through barrier sets the other flag; the earlier flags are cleared first.
    mark_barrier_crossings(&mut net, &index, &[across(50.0, true)], &[]);
    assert!(!net.segments[0].has(flags::CROSSES_BARRIER));
    assert!(net.segments[0].has(flags::CROSSES_DRIVE_THROUGH_BARRIER));

    // No barrier: both flags are gone.
    assert_eq!(mark_barrier_crossings(&mut net, &index, &[], &[]), 0);
    assert!(!net.segments[0].has(flags::CROSSES_BARRIER | flags::CROSSES_DRIVE_THROUGH_BARRIER));
}

#[test]
fn a_barrier_beside_or_past_the_road_flags_nothing() {
    let mut net = straight_road();
    let index = SegmentIndex::build(&net);
    let beside = ActiveBarrier { a: Vec2::new(0.0, 20.0), b: Vec2::new(100.0, 20.0), drive_through: false };
    assert_eq!(mark_barrier_crossings(&mut net, &index, &[beside, across(150.0, false)], &[]), 0);
    assert_eq!(net.segments[0].flags, 0);
}

#[test]
fn segments_of_an_exempt_road_are_skipped() {
    let mut net = straight_road();
    let index = SegmentIndex::build(&net);
    assert_eq!(mark_barrier_crossings(&mut net, &index, &[across(50.0, false)], &[0]), 0);
    assert!(!net.segments[0].has(flags::CROSSES_BARRIER));
}

#[test]
fn line_crossing_handles_touching_and_parallel_lines() {
    let p = |x: f32, y: f32| Vec2::new(x, y);
    assert!(lines_cross(p(0.0, 0.0), p(2.0, 2.0), p(0.0, 2.0), p(2.0, 0.0)));
    assert!(lines_cross(p(0.0, 0.0), p(2.0, 0.0), p(2.0, 0.0), p(3.0, 1.0)), "an end touching counts");
    assert!(!lines_cross(p(0.0, 0.0), p(2.0, 0.0), p(0.0, 1.0), p(2.0, 1.0)), "parallel lines do not");
    assert!(lines_cross(p(0.0, 0.0), p(2.0, 0.0), p(1.0, 0.0), p(3.0, 0.0)), "overlapping collinear lines do");
    assert!(!lines_cross(p(0.0, 0.0), p(1.0, 0.0), p(2.0, 0.0), p(3.0, 0.0)), "collinear but apart");
}

#[test]
#[ignore = "needs a game install; set NFSMW_GAME_DIR"]
fn real_install_barriers_ship_switched_off() {
    let Some(dir) = std::env::var_os("NFSMW_GAME_DIR") else { return };
    let raw = std::fs::read(std::path::Path::new(&dir).join("TRACKS/L2RA.BUN")).unwrap();
    let data = ea_compress::unwrap(&raw).unwrap();
    let barriers = read_track_barriers(&data).unwrap();
    assert_eq!(barriers.len(), 1376);
    assert!(barriers.iter().all(|b| !b.enabled && !b.drive_through));
    // Gate 10 (`BARRIER_LOCKED_10`), docs/specs/career-regions.md section 1.3.
    let gate = barriers.iter().find(|b| (b.a - Vec2::new(2778.0, 624.2)).length() < 0.1).expect("gate 10 barrier");
    assert!((gate.b - Vec2::new(2779.4, 596.0)).length() < 0.1);
}
