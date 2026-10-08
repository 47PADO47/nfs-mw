use super::synth::*;
use crate::carp::tag4;
use crate::{CollisionPack, CollisionWorld, Error, Grid, InstanceRef, RayOptions};

const ASPHALT: u32 = 0x19DB_2F1E;

/// One node: index, then per kind the element values.
fn node(index: u16, kinds: [&[u32]; 4]) -> Vec<u8> {
    let mut v = Vec::new();
    push_u32(&mut v, 0); // runtime list pointer
    push_u16(&mut v, index);
    push_u16(&mut v, 0);
    v.extend(kinds.iter().map(|k| k.len() as u8));
    let mut off = 0u16;
    for k in kinds {
        push_u16(&mut v, off);
        off += 4 * k.len() as u16;
    }
    for k in kinds {
        for e in k {
            push_u32(&mut v, *e);
        }
    }
    v
}

/// A 4 x 3 grid (rows x cols) of 64 m cells whose corner is (-100, 0, -50).
fn grid_payload(nodes: &[Vec<u8>]) -> Vec<u8> {
    let mut head = Vec::new();
    for c in [-100.0f32, 0.0, -50.0, 1.0, 64.0, 1.0 / 64.0] {
        push_f32(&mut head, c);
    }
    push_u32(&mut head, 4);
    push_u32(&mut head, 3);
    push_u32(&mut head, 0xCAFE_0000);
    let all: Vec<u8> = nodes.concat();
    carp(tag4(b"CDat"), &[(tag4(b"CGcn"), nodes.len() as u32, all), (tag4(b"CGrd"), 1, head)])
}

#[test]
fn parses_grid_and_nodes() {
    let blob = grid_payload(&[
        node(1, [&[(7 << 16) | 2, (8 << 16)], &[], &[], &[40]]),
        node(11, [&[(7 << 16) | 2], &[3], &[4], &[]]),
    ]);
    let g = Grid::parse(&blob).unwrap();
    assert_eq!((g.rows, g.cols, g.edge, g.min), (4, 3, 64.0, [-100.0, 0.0, -50.0]));
    let n = g.node(1).unwrap();
    assert_eq!(n.instances, [InstanceRef::new(7, 2), InstanceRef::new(8, 0)]);
    assert_eq!(n.road_segments, [40]);
    let n = g.node(11).unwrap();
    assert_eq!((n.triggers.as_slice(), n.objects.as_slice()), (&[3][..], &[4][..]));
    assert!(g.node(0).is_none() && g.node(12).is_none());
    assert_eq!(InstanceRef::new(7, 2).section(), 7);
    assert_eq!(InstanceRef::new(7, 2).index(), 2);
}

#[test]
fn cells_and_segments() {
    let g = Grid::parse(&grid_payload(&[
        node(0, [&[1], &[], &[], &[]]),
        node(4, [&[2], &[], &[], &[]]),
        node(11, [&[3], &[], &[], &[]]),
    ]))
    .unwrap();
    // x = -100 + 64 * col, z = -50 + 64 * row; indices are row * 3 + col.
    assert_eq!(g.cell_of(-99.0, -49.0), (0, 0));
    assert_eq!(g.cell_of(-30.0, 20.0), (1, 1));
    assert_eq!(g.cell_of(1e9, -1e9), (0, 2), "clamped to the grid");
    assert_eq!(g.node_index(1, 1), 4);
    // A segment through cells 0 and 4 only (not the empty cells between).
    let refs = g.instances_along([-99.0, 0.0, -49.0], [-30.0, 0.0, 20.0]);
    assert_eq!(refs, [InstanceRef(1), InstanceRef(2)]);
    // A segment inside one cell.
    assert_eq!(g.instances_along([-90.0, 0.0, -40.0], [-80.0, 0.0, -45.0]), [InstanceRef(1)]);
    // A vertical segment stays in its cell.
    assert_eq!(g.instances_along([-30.0, 9.0, 20.0], [-30.0, -9.0, 20.0]), [InstanceRef(2)]);
}

#[test]
fn rejects_bad_grids() {
    let blob = grid_payload(&[node(12, [&[], &[], &[], &[]])]);
    assert!(matches!(Grid::parse(&blob), Err(Error::Malformed { .. })), "node outside the grid");
    assert!(matches!(Grid::parse(&carp(tag4(b"Zzzz"), &[])), Err(Error::Missing(_))));
}

#[test]
fn world_uses_the_grid_to_pick_candidates() {
    // Two tiles: one in cell 4 and one far away that the grid never lists for this segment.
    let tile = article(&[ground_strip(0)], &[], &[ASPHALT]);
    let near = InstanceSpec::upright([-20.0, 0.0, 30.0], [4.0, 1.0, 4.0], 0);
    let far = InstanceSpec::upright([60.0, 0.0, 140.0], [4.0, 1.0, 4.0], 1);
    let pack = CollisionPack::parse(&pack(7, &[tile.clone(), tile], &[near, far])).unwrap();
    let grid =
        Grid::parse(&grid_payload(&[node(4, [&[7 << 16], &[], &[], &[]]), node(8, [&[(7 << 16) | 1], &[], &[], &[]])]))
            .unwrap();
    let mut w = CollisionWorld::new(Some(grid));
    w.insert(pack);
    assert_eq!(w.pack_count(), 1);
    let opts = RayOptions::default();
    assert_eq!(w.ray_cast([-20.0, 5.0, 30.0], [-20.0, -5.0, 30.0], &opts).unwrap().instance, 0);
    assert_eq!(w.ray_cast([60.0, 5.0, 140.0], [60.0, -5.0, 140.0], &opts).unwrap().instance, 1);

    // The same far tile is invisible if the grid does not list it.
    let tile = article(&[ground_strip(0)], &[], &[ASPHALT]);
    let lonely = CollisionPack::parse(&pack_of_one(tile)).unwrap();
    let mut w = CollisionWorld::new(Some(Grid::parse(&grid_payload(&[])).unwrap()));
    w.insert(lonely);
    assert!(w.ray_cast([60.0, 5.0, 140.0], [60.0, -5.0, 140.0], &opts).is_none());
    assert!(w.remove(7).is_some() && w.remove(7).is_none());
}

fn pack_of_one(tile: Vec<u8>) -> Vec<u8> {
    pack(7, &[tile], &[InstanceSpec::upright([60.0, 0.0, 140.0], [4.0, 1.0, 4.0], 0)])
}
