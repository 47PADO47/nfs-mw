//! The placement rule on authored numbers: no game data.

use blackbox_modding::content::{FitLimits, SwapFit};
use blackbox_solid::Vertex;

use super::*;

fn vertex(x: f32, y: f32, z: f32) -> Vertex {
    Vertex { position: [x, y, z], normal: [0.0, 0.0, 1.0], color_bgra: [128; 4], uv: [0.0, 0.0] }
}

/// A model 10 metres wide and 20 tall whose crown begins 4 metres up.
const MODEL: Measure = Measure { width: 10.0, height: 20.0, vertices: 100 };

fn stock(vertices: &[Vertex]) -> Stock<'_> {
    let first = vertices[0].position;
    let (mut low, mut high) = (first, first);
    for vertex in vertices {
        for axis in 0..3 {
            low[axis] = low[axis].min(vertex.position[axis]);
            high[axis] = high[axis].max(vertex.position[axis]);
        }
    }
    Stock { vertices, bounds: (low, high) }
}

/// A box of vertices: `columns` by `rows` points on a plan of `width` by `depth`, low at `floor` and high `thick` above it.
fn block(width: f32, depth: f32, floor: f32, thick: f32) -> Vec<Vertex> {
    let mut vertices = Vec::new();
    for column in 0..=10 {
        for row in 0..=10 {
            let (x, y) = (width * column as f32 / 10.0, depth * row as f32 / 10.0);
            vertices.push(vertex(x, y, floor));
            vertices.push(vertex(x, y, floor + thick));
        }
    }
    vertices
}

#[test]
fn a_single_tree_stands_on_the_origin_with_the_height_of_the_solid() {
    let points = block(10.0, 10.0, 2.0, 20.0);
    let one = single(&MODEL, &stock(&points), &FitLimits::default()).unwrap();
    assert_eq!((one.up, one.across, one.yaw), (1.1, 1.1, 0.0));
    assert_eq!(one.offset, [0.0, 0.0, 0.0], "the origin is the ground");
}

#[test]
fn a_tree_wider_than_the_footprint_is_squeezed_to_the_spread() {
    let points = block(6.0, 6.0, 0.0, 20.0);
    let limits = FitLimits::default();
    let one = single(&MODEL, &stock(&points), &limits).unwrap();
    assert_eq!(one.up, 1.0, "as tall as the solid");
    assert!((one.across * MODEL.width - limits.spread * 6.0).abs() < 1e-4, "as wide as the spread allows: {one:?}");
}

#[test]
fn a_tree_still_too_wide_is_made_smaller_and_one_too_small_is_refused() {
    let limits = FitLimits::default();
    // 3 metres wide: 3.75 metres of crown is 0.375 of the model; its height would be 0.75 of the wanted 1.0.
    let narrow = block(3.0, 3.0, 0.0, 20.0);
    let one = single(&MODEL, &stock(&narrow), &limits).unwrap();
    assert!((one.across - 0.375).abs() < 1e-4 && (one.up - 0.75).abs() < 1e-4, "{one:?}");
    // 2 metres wide: it would be half the height, under the least share.
    let thin = block(2.0, 2.0, 0.0, 20.0);
    assert!(single(&MODEL, &stock(&thin), &limits).is_none());
    let loose = FitLimits { min_height: 0.0, ..limits };
    assert!(single(&MODEL, &stock(&thin), &loose).is_some());
}

#[test]
fn a_crown_that_begins_above_the_origin_still_stands_on_it() {
    // The old far-tree solid is a crown from 7 to 23 metres: the tree grows from the ground, up to the crown's top.
    let points = block(12.0, 12.0, 7.0, 16.0);
    let one = single(&MODEL, &stock(&points), &FitLimits::default()).unwrap();
    assert!((one.up - 1.15).abs() < 1e-4, "{one:?}");
    assert_eq!(one.offset[2], 0.0);
}

#[test]
fn a_row_is_one_copy_for_each_stretch_each_on_the_floor_of_its_stretch() {
    // 100 metres long, 10 wide and 8 thick, rising 20 metres.
    let mut vertices = Vec::new();
    for n in 0..=100 {
        let x = n as f32;
        for y in [0.0, 5.0, 10.0] {
            vertices.push(vertex(x, y, 0.2 * x));
            vertices.push(vertex(x, y, 0.2 * x + 8.0));
        }
    }
    let at = copies(&MODEL, &stock(&vertices), SwapFit::Row, &FitLimits::default());
    assert!((4..=MAX_COPIES).contains(&at.len()), "{} copies", at.len());
    assert!(at.windows(2).all(|pair| pair[1].offset[0] > pair[0].offset[0]), "in order along the row");
    for copy in &at {
        let floor = copy.offset[2];
        let ground = 0.2 * copy.offset[0];
        assert!((floor - ground).abs() < 0.2 * 12.0, "the foot follows the old floor: {floor} against {ground}");
        assert!(copy.across * MODEL.width <= FitLimits::default().spread * 24.0, "no wider than a stretch: {copy:?}");
    }
    assert!(at.windows(2).all(|pair| pair[0].yaw != pair[1].yaw), "no two copies are alike");
}

#[test]
fn a_row_follows_a_bend() {
    let mut vertices = Vec::new();
    for n in 0..=60 {
        vertices.push(vertex(n as f32, 0.0, 0.0));
        vertices.push(vertex(n as f32, 6.0, 12.0));
    }
    for n in 1..=60 {
        vertices.push(vertex(60.0, n as f32, 0.0));
        vertices.push(vertex(66.0, n as f32, 12.0));
    }
    let at = copies(&MODEL, &stock(&vertices), SwapFit::Row, &FitLimits::default());
    assert!(at.iter().any(|copy| copy.offset[0] < 20.0 && copy.offset[1] < 10.0), "on the first leg");
    assert!(at.iter().any(|copy| copy.offset[1] > 50.0 && copy.offset[0] > 55.0), "on the second leg");
    assert!(!at.iter().any(|copy| copy.offset[0] < 40.0 && copy.offset[1] > 30.0), "none cuts across the corner");
}

#[test]
fn a_single_tree_copy_of_the_world_is_one_cell() {
    let points = block(10.0, 10.0, 5.0, 20.0);
    let at = copies(&MODEL, &stock(&points), SwapFit::Height, &FitLimits::default());
    assert_eq!(at.len(), 1, "{at:?}");
    assert!((at[0].offset[0] - 5.0).abs() < 1e-4 && (at[0].offset[1] - 5.0).abs() < 1e-4, "{:?}", at[0].offset);
    assert!((at[0].offset[2] - 5.0).abs() < 1e-4);
}

#[test]
fn a_solid_too_big_for_the_most_copies_gets_larger_stretches_or_keeps_its_model() {
    let mut vertices = Vec::new();
    for column in 0..=40 {
        for row in 0..=40 {
            vertices.push(vertex(column as f32 * 10.0, row as f32 * 10.0, 0.0));
            vertices.push(vertex(column as f32 * 10.0, row as f32 * 10.0, 20.0));
        }
    }
    let at = copies(&MODEL, &stock(&vertices), SwapFit::Row, &FitLimits::default());
    assert!(at.len() <= MAX_COPIES, "{} copies", at.len());
    let huge: Vec<Vertex> =
        (0..=400).flat_map(|n| (0..3).map(move |level| vertex(n as f32 * 100.0, 0.0, level as f32 * 10.0))).collect();
    assert!(copies(&MODEL, &stock(&huge), SwapFit::Row, &FitLimits::default()).is_empty(), "40 km of trees");
}

#[test]
fn stray_vertices_hold_no_copy() {
    let vertices = [vertex(0.0, 0.0, 0.0), vertex(1.0, 0.0, 10.0)];
    assert!(copies(&MODEL, &stock(&vertices), SwapFit::Row, &FitLimits::default()).is_empty());
}

#[test]
fn the_vertices_of_all_the_copies_stay_under_the_writers_limit() {
    let heavy = Measure { vertices: 4000, ..MODEL };
    let mut vertices = Vec::new();
    for n in 0..=100 {
        vertices.push(vertex(n as f32, 0.0, 0.0));
        vertices.push(vertex(n as f32, 5.0, 10.0));
    }
    let at = copies(&heavy, &stock(&vertices), SwapFit::Row, &FitLimits::default());
    assert!(at.len() * heavy.vertices <= MAX_VERTICES, "{} copies", at.len());
}
