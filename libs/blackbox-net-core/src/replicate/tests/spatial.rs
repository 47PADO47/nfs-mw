//! The spatial hash against a brute-force scan.

use crate::replicate::SpatialHash;
use crate::rng::SplitMix64;

/// A random coordinate in -500..500.
fn coord(rng: &mut SplitMix64) -> f32 {
    (rng.next_f64() as f32) * 1000.0 - 500.0
}

/// The ids within `radius` of `(x, z)` by a full scan, sorted.
fn brute(points: &[(u16, f32, f32)], x: f32, z: f32, radius: f32) -> Vec<u16> {
    let r = radius.max(0.0);
    let mut ids: Vec<u16> = points
        .iter()
        .filter(|(_, px, pz)| {
            let dx = px - x;
            let dz = pz - z;
            dx * dx + dz * dz <= r * r
        })
        .map(|(id, _, _)| *id)
        .collect();
    ids.sort_unstable();
    ids
}

#[test]
fn query_matches_a_brute_force_scan_on_2000_points() {
    let mut rng = SplitMix64(99);
    for cell in [1.0f32, 7.5, 50.0, 400.0] {
        let mut hash = SpatialHash::new(cell);
        let points: Vec<(u16, f32, f32)> = (0..2_000u16).map(|id| (id, coord(&mut rng), coord(&mut rng))).collect();
        for &(id, x, z) in &points {
            hash.insert(id, x, z);
        }
        let mut out = Vec::new();
        for _ in 0..200 {
            let (x, z) = (coord(&mut rng), coord(&mut rng));
            let radius = (rng.next_f64() as f32) * 300.0;
            hash.query(x, z, radius, &mut out);
            assert_eq!(out, brute(&points, x, z, radius), "cell {cell}, query at ({x}, {z}) radius {radius}");
        }
    }
}

#[test]
fn a_radius_covering_everything_returns_every_id() {
    let mut rng = SplitMix64(4);
    let mut hash = SpatialHash::new(3.0);
    let points: Vec<(u16, f32, f32)> = (0..500u16).map(|id| (id, coord(&mut rng), coord(&mut rng))).collect();
    for &(id, x, z) in &points {
        hash.insert(id, x, z);
    }
    let mut out = Vec::new();
    hash.query(0.0, 0.0, 1.0e6, &mut out);
    assert_eq!(out, brute(&points, 0.0, 0.0, 1.0e6));
    assert_eq!(out.len(), 500);
}

#[test]
fn clear_empties_the_hash_and_it_can_be_refilled() {
    let mut hash = SpatialHash::new(10.0);
    hash.insert(1, 5.0, 5.0);
    hash.insert(2, 6.0, 5.0);
    let mut out = Vec::new();
    hash.query(5.0, 5.0, 10.0, &mut out);
    assert_eq!(out, vec![1, 2]);
    hash.clear();
    hash.query(5.0, 5.0, 10.0, &mut out);
    assert!(out.is_empty());
    hash.insert(2, 5.0, 5.0);
    hash.query(5.0, 5.0, 1.0, &mut out);
    assert_eq!(out, vec![2]);
}

#[test]
fn the_radius_is_inclusive_and_exact() {
    let mut hash = SpatialHash::new(2.0);
    hash.insert(1, 3.0, 4.0);
    let mut out = Vec::new();
    hash.query(0.0, 0.0, 5.0, &mut out);
    assert_eq!(out, vec![1]);
    hash.query(0.0, 0.0, 4.99, &mut out);
    assert!(out.is_empty());
}

#[test]
fn negative_radius_acts_as_zero_and_duplicates_are_dropped() {
    let mut hash = SpatialHash::new(1.0);
    hash.insert(2, 0.0, 0.0);
    hash.insert(2, 0.0, 0.0);
    hash.insert(7, 0.5, 0.0);
    let mut out = Vec::new();
    hash.query(0.0, 0.0, -1.0, &mut out);
    assert_eq!(out, vec![2]);
}

#[test]
fn a_bad_cell_size_falls_back_to_one() {
    let mut hash = SpatialHash::new(f32::NAN);
    hash.insert(3, 0.25, 0.25);
    let mut out = Vec::new();
    hash.query(0.25, 0.25, 0.1, &mut out);
    assert_eq!(out, vec![3]);
    let mut zero = SpatialHash::new(0.0);
    zero.insert(4, -2.0, 2.0);
    zero.query(-2.0, 2.0, 0.0, &mut out);
    assert_eq!(out, vec![4]);
}
