//! The collision grid (`CDat/CGrd` and `CDat/CGcn` in the `0x3B800` world map tree): a regular
//! xz grid whose cells list the instances and road segments that touch them.
//! Spec: `docs/formats/collision.md` ("The collision grid").

use blackbox_chunk::{Chunk, ids};

use crate::bytes::{Reader, malformed};
use crate::carp::{Carp, tag4};
use crate::math::Vec3;
use crate::{Error, Result};

const PACK_ALIGN: usize = 16;
const GRID_HEADER_LEN: usize = 0x24;
const NODE_HEADER_LEN: usize = 0x14;

/// A reference to an instance: the section of its pack and its index inside it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct InstanceRef(pub u32);

impl InstanceRef {
    pub fn new(section: u16, index: u16) -> Self {
        Self(u32::from(section) << 16 | u32::from(index))
    }

    pub fn section(self) -> u32 {
        self.0 >> 16
    }

    pub fn index(self) -> usize {
        (self.0 & 0xFFFF) as usize
    }
}

/// One non-empty grid cell.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct GridNode {
    /// `row * cols + col`.
    pub index: u16,
    pub instances: Vec<InstanceRef>,
    pub triggers: Vec<u32>,
    pub objects: Vec<u32>,
    /// Indices into the road network's segment table.
    pub road_segments: Vec<u32>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Grid {
    /// World position of the grid's minimum corner (only x and z matter).
    pub min: Vec3,
    /// Cell edge length in metres.
    pub edge: f32,
    pub rows: u32,
    pub cols: u32,
    nodes: Vec<Option<GridNode>>,
}

impl Grid {
    /// Parses the payload of a `0x3B800` chunk (as returned by `Chunk::aligned_payload(16)`).
    pub fn parse(payload: &[u8]) -> Result<Self> {
        let root = Carp::new(payload).root()?;
        let cdat = root.group(tag4(b"CDat"))?.ok_or(Error::Missing("CDat group"))?;
        let head = cdat.require(tag4(b"CGrd"), "CGrd record")?;
        if head.data.len() < GRID_HEADER_LEN {
            return Err(malformed("collision grid", format!("{} bytes of header", head.data.len())));
        }
        let h = Reader::new(head.data, "collision grid header");
        let (rows, cols) = (h.u32(0x18)?, h.u32(0x1C)?);
        let cells = rows
            .checked_mul(cols)
            .filter(|&n| n <= 1 << 20)
            .ok_or_else(|| malformed("collision grid", format!("{rows} x {cols} cells")))?;
        let mut grid = Self { min: h.vec3(0)?, edge: h.f32(0x10)?, rows, cols, nodes: vec![None; cells as usize] };

        if let Some(rec) = cdat.record(tag4(b"CGcn"))? {
            let r = Reader::new(rec.data, "collision grid nodes");
            let mut at = 0;
            for _ in 0..rec.count {
                let index = r.u16(at + 4)?;
                let counts = r.slice(at + 8, 4)?;
                let offsets = [r.u16(at + 12)?, r.u16(at + 14)?, r.u16(at + 16)?, r.u16(at + 18)?];
                let total: usize = counts.iter().map(|&c| usize::from(c)).sum();
                let mut node = GridNode { index, ..GridNode::default() };
                for (kind, (&n, &off)) in counts.iter().zip(&offsets).enumerate() {
                    let values = (0..usize::from(n))
                        .map(|i| r.u32(at + NODE_HEADER_LEN + usize::from(off) + i * 4))
                        .collect::<Result<Vec<_>>>()?;
                    match kind {
                        0 => node.instances = values.into_iter().map(InstanceRef).collect(),
                        1 => node.triggers = values,
                        2 => node.objects = values,
                        _ => node.road_segments = values,
                    }
                }
                let slot = grid
                    .nodes
                    .get_mut(usize::from(index))
                    .ok_or_else(|| malformed("collision grid", format!("node {index} outside {rows} x {cols}")))?;
                *slot = Some(node);
                at += NODE_HEADER_LEN + total * 4;
            }
        }
        Ok(grid)
    }

    /// Parses a `0x3B800` chunk.
    pub fn from_chunk(chunk: &Chunk<'_>) -> Result<Self> {
        Self::parse(chunk.aligned_payload(PACK_ALIGN))
    }

    /// The grid in `data` (the world metadata file), if it has one.
    pub fn read(data: &[u8]) -> Result<Option<Self>> {
        blackbox_chunk::find(data, ids::CARP_WGRID).map(|c| Self::from_chunk(&c)).transpose()
    }

    pub fn node(&self, index: usize) -> Option<&GridNode> {
        self.nodes.get(index)?.as_ref()
    }

    /// Cell `(row, col)` of a world point, clamped to the grid.
    pub fn cell_of(&self, x: f32, z: f32) -> (u32, u32) {
        let clamp = |v: f32, n: u32| (v.floor().max(0.0) as u32).min(n.saturating_sub(1));
        let inv = 1.0 / self.edge;
        (clamp((z - self.min[2]) * inv, self.rows), clamp((x - self.min[0]) * inv, self.cols))
    }

    pub fn node_index(&self, row: u32, col: u32) -> usize {
        (row * self.cols + col) as usize
    }

    /// Non-empty cells whose square touches the xz projection of the segment `a`-`b`.
    pub fn nodes_along(&self, a: Vec3, b: Vec3) -> Vec<&GridNode> {
        let (r0, c0) = self.cell_of(a[0].min(b[0]), a[2].min(b[2]));
        let (r1, c1) = self.cell_of(a[0].max(b[0]), a[2].max(b[2]));
        let mut out = Vec::new();
        for row in r0..=r1 {
            for col in c0..=c1 {
                if let Some(node) = self.node(self.node_index(row, col))
                    && self.segment_touches(a, b, row, col)
                {
                    out.push(node);
                }
            }
        }
        out
    }

    /// Distinct instance references in the cells along the segment.
    pub fn instances_along(&self, a: Vec3, b: Vec3) -> Vec<InstanceRef> {
        let mut refs: Vec<InstanceRef> =
            self.nodes_along(a, b).iter().flat_map(|n| n.instances.iter().copied()).collect();
        refs.sort_unstable();
        refs.dedup();
        refs
    }

    /// Slab test of the segment against the cell square (xz).
    fn segment_touches(&self, a: Vec3, b: Vec3, row: u32, col: u32) -> bool {
        let lo = [self.min[0] + col as f32 * self.edge, self.min[2] + row as f32 * self.edge];
        let hi = [lo[0] + self.edge, lo[1] + self.edge];
        let (mut t0, mut t1) = (0.0f32, 1.0f32);
        for (p, d, l, h) in [(a[0], b[0] - a[0], lo[0], hi[0]), (a[2], b[2] - a[2], lo[1], hi[1])] {
            if d.abs() < 1e-9 {
                if p < l || p > h {
                    return false;
                }
            } else {
                let (ta, tb) = ((l - p) / d, (h - p) / d);
                t0 = t0.max(ta.min(tb));
                t1 = t1.min(ta.max(tb));
                if t0 > t1 {
                    return false;
                }
            }
        }
        true
    }
}
