//! The racing line as a ribbon on the ground: the track authors' ideal trajectory, a thin bright strip lying 4 cm above the
//! collision, drawn as an always-loaded cell of the world (`docs/specs/second-world.md`). It is geometry, not an overlay of the
//! host: a world is looked at by the game's own renderer, and this is what a mod can add to one.

use crate::cells::{Cell, Part};
use crate::pack::{Lap, Material, Pack};

/// The ribbon's width, metres.
const WIDTH: f32 = 0.7;
/// How far above the ground the ribbon lies, metres.
const LIFT: f32 = 0.04;
/// The colour of the line.
const COLOUR: [u8; 3] = [40, 240, 120];

/// A pack with the one material of the line, for [`write_cell`](crate::import) to draw its cell with.
pub fn line_pack(track: &str) -> Pack {
    let material = Material { name: "line".into(), texture: None, color: COLOUR, surface: "dirt".into() };
    Pack {
        track: track.to_owned(),
        materials: vec![material],
        groups: Vec::new(),
        lap: Lap::default(),
        sea: None,
        models: Vec::new(),
        object_groups: Vec::new(),
        instances: Vec::new(),
    }
}

/// The cell holding the ribbon along `points` (render space); `ground_height(x, y, z)` gives the height of the collision under a
/// point near `z`, or `None` where there is none (the point's own height is used then). A closed line joins its two ends.
pub fn ribbon_cell(points: &[[f32; 3]], closed: bool, ground_height: impl Fn(f32, f32, f32) -> Option<f32>) -> Cell {
    let n = points.len();
    let mut positions = Vec::with_capacity(2 * n);
    for i in 0..n {
        let before = points[(i + n - 1) % n];
        let after = points[(i + 1) % n];
        let (before, after) = match (i, closed) {
            (0, false) => (points[0], points[1]),
            (last, false) if last == n - 1 => (points[n - 2], points[n - 1]),
            _ => (before, after),
        };
        let (tx, ty) = (after[0] - before[0], after[1] - before[1]);
        let length = tx.hypot(ty).max(1e-6);
        let (lx, ly) = (-ty / length * WIDTH / 2.0, tx / length * WIDTH / 2.0);
        let p = points[i];
        for side in [1.0, -1.0] {
            let (x, y) = (p[0] + side * lx, p[1] + side * ly);
            let z = ground_height(x, y, p[2]).unwrap_or(p[2]) + LIFT;
            positions.push([x, y, z]);
        }
    }
    let segments = if closed { n } else { n - 1 };
    let mut indices = Vec::with_capacity(segments * 6);
    for i in 0..segments {
        let (l, r) = (2 * i as u16, 2 * i as u16 + 1);
        let next = 2 * ((i + 1) % n) as u16;
        indices.extend([l, r, next, r, next + 1, next]);
    }
    let count = positions.len();
    let part =
        Part { material: 0, positions, uvs: vec![[0.0, 0.0]; count], normals: vec![[0, 0, 127]; count], indices };
    Cell { key: (0, 0), parts: vec![part], ground: Vec::new() }
}

/// Whether a pack has a racing line to draw.
pub fn has_line(pack: &Pack) -> bool {
    pack.lap.line.points.len() >= 8 && pack.lap.line.points.len() < usize::from(u16::MAX) / 2
}

#[cfg(test)]
mod tests {
    use super::*;

    fn straight(n: usize) -> Vec<[f32; 3]> {
        (0..n).map(|i| [i as f32 * 2.0, 0.0, 5.0]).collect()
    }

    #[test]
    fn a_ribbon_is_two_vertices_a_sample_lies_flat_above_the_ground_and_faces_up() {
        let cell = ribbon_cell(&straight(10), false, |_, _, _| Some(3.0));
        let part = &cell.parts[0];
        assert_eq!(part.positions.len(), 20);
        assert_eq!(part.indices.len(), 9 * 6);
        assert!(part.positions.iter().all(|p| (p[2] - 3.04).abs() < 1e-5), "above the collision, not the spline");
        // The width: the two vertices of a sample are 0.7 m apart across the line.
        let (a, b) = (part.positions[4], part.positions[5]);
        assert!((a[1] - b[1]).abs() > 0.69 && (a[1] - b[1]).abs() < 0.71);
        // Counter-clockwise seen from above: the cross product of the first triangle points up.
        let t = |k: usize| part.positions[usize::from(part.indices[k])];
        let (u, v) = ([t(1)[0] - t(0)[0], t(1)[1] - t(0)[1]], [t(2)[0] - t(0)[0], t(2)[1] - t(0)[1]]);
        assert!(u[0] * v[1] - u[1] * v[0] > 0.0);
    }

    #[test]
    fn with_no_ground_the_line_keeps_its_own_height_and_a_closed_line_joins_up() {
        let ring: Vec<[f32; 3]> = (0..12)
            .map(|i| {
                let a = i as f32 / 12.0 * std::f32::consts::TAU;
                [50.0 * a.cos(), 50.0 * a.sin(), 7.0]
            })
            .collect();
        let cell = ribbon_cell(&ring, true, |_, _, _| None);
        let part = &cell.parts[0];
        assert_eq!(part.indices.len(), 12 * 6, "one more segment than an open line");
        assert!(part.positions.iter().all(|p| (p[2] - 7.04).abs() < 1e-5));
        assert!(part.indices.iter().all(|&i| usize::from(i) < part.positions.len()));
    }
}
