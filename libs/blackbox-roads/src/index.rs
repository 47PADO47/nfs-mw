//! A bucket grid over the road segments, for "which segments are near this point".

use glam::Vec3;

use crate::{RoadNetwork, centre_line};

/// Cell edge length in metres (the collision grid's).
const CELL: f32 = 64.0;

#[derive(Debug, Clone, PartialEq)]
pub struct SegmentIndex {
    min: [f32; 2],
    cols: usize,
    rows: usize,
    cells: Vec<Vec<u16>>,
}

impl SegmentIndex {
    pub fn build(net: &RoadNetwork) -> Self {
        let (mut lo, mut hi) = ([f32::MAX; 2], [f32::MIN; 2]);
        for node in &net.nodes {
            lo = [lo[0].min(node.position.x), lo[1].min(node.position.z)];
            hi = [hi[0].max(node.position.x), hi[1].max(node.position.z)];
        }
        // Curved segments bulge past their nodes by at most their handles.
        let margin = 500.0;
        let min = [lo[0] - margin, lo[1] - margin];
        let cols = (((hi[0] + margin - min[0]) / CELL) as usize + 1).max(1);
        let rows = (((hi[1] + margin - min[1]) / CELL) as usize + 1).max(1);
        let mut index = Self { min, cols, rows, cells: vec![Vec::new(); cols * rows] };
        for s in 0..net.segments.len() as u16 {
            let points = centre_line(net, s, 1).points;
            let (mut blo, mut bhi) = (points[0], points[0]);
            for p in points {
                blo = blo.min(p);
                bhi = bhi.max(p);
            }
            let ((c0, r0), (c1, r1)) = (index.cell_of(blo), index.cell_of(bhi));
            for r in r0..=r1 {
                for c in c0..=c1 {
                    index.cells[r * cols + c].push(s);
                }
            }
        }
        index
    }

    fn cell_of(&self, p: Vec3) -> (usize, usize) {
        let col = ((p.x - self.min[0]) / CELL).max(0.0) as usize;
        let row = ((p.z - self.min[1]) / CELL).max(0.0) as usize;
        (col.min(self.cols - 1), row.min(self.rows - 1))
    }

    /// Segments whose control polygon overlaps a square of half edge `radius` around `p`, each once.
    pub fn near(&self, p: Vec3, radius: f32) -> Vec<u16> {
        let ((c0, r0), (c1, r1)) = (self.cell_of(p - Vec3::splat(radius)), self.cell_of(p + Vec3::splat(radius)));
        let mut found = Vec::new();
        for r in r0..=r1 {
            for c in c0..=c1 {
                found.extend_from_slice(&self.cells[r * self.cols + c]);
            }
        }
        found.sort_unstable();
        found.dedup();
        found
    }
}
