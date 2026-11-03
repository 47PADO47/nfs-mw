//! The top of a bike's seat, from its exported chassis mesh: the height of the highest surface under the rider along the bike,
//! written to `contacts.json` as `seat_surface` so the game can sit the rider on it (`docs/specs/ride6-rider-render-integration.md`,
//! section 8).
//!
//! `SOCKET_RIDER` is not on the seat: measured on five bikes it is 2.9 to 14.6 cm above the seat's top at its place (BMW R 1300 GS
//! 2.9, KTM 450 SMR 5.9, Honda X-ADV 9.1, Yamaha R7 12.6, Ducati 998R 14.6), and it is the rider's root bone, not his buttocks. The
//! seat's own height is only in the mesh, so it is measured there: vertical rays from above the rider down on the triangles of the
//! body parts, along the centre line and 7 cm to each side (where the buttocks are), taking the highest hit below the rider's
//! root plus [`ABOVE_ROOT_M`]; the three lines are averaged. The profile covers [`BEHIND_M`] behind the root to [`AHEAD_M`] ahead of it.
//! All numbers are choices of the measurement, not data of the game.

use std::path::Path;

use anyhow::{Context, Result, bail};
use glam::Vec3;

use crate::exports::Exports;
use crate::partfile::{Kind, PartTable, Role};

/// How far behind the rider's root the profile starts, metres.
pub const BEHIND_M: f32 = 0.30;
/// How far ahead of the rider's root it ends, metres (the tank begins to rise there: the contact test ignores the rider's
/// knees by restricting the contact points, see `docs/formats/ride6-rider.md`).
pub const AHEAD_M: f32 = 0.10;
/// The spacing of the samples along the bike, metres.
pub const STEP_M: f32 = 0.02;
/// The sideways places of the rays, metres from the centre line (left positive).
pub const LINES_M: [f32; 3] = [-0.07, 0.0, 0.07];
/// A surface higher than this above the rider's root is not the seat (a tank, a back rest): the rays accept hits up to here.
pub const ABOVE_ROOT_M: f32 = 0.03;

/// A triangle in the car frame (x forward, y left, z up), metres.
pub type Triangle = [Vec3; 3];

/// A point of the part's frame in the car frame: the part's anchor into the chassis frame (glTF axes: x forward, y up, z right)
/// and then glTF to car.
fn car_point(anchor: &crate::partfile::Anchor, p: [f32; 3]) -> Vec3 {
    let q = anchor.point(p);
    Vec3::new(q[0], -q[2], q[1])
}

/// The height of the highest triangle under the vertical line through `(x, y)` that is not above `limit`.
pub fn height_at(triangles: &[Triangle], (x, y): (f32, f32), limit: f32) -> Option<f32> {
    let mut best: Option<f32> = None;
    for [a, b, c] in triangles {
        let d = (b.y - c.y) * (a.x - c.x) + (c.x - b.x) * (a.y - c.y);
        if d.abs() < 1e-12 {
            continue;
        }
        let l1 = ((b.y - c.y) * (x - c.x) + (c.x - b.x) * (y - c.y)) / d;
        let l2 = ((c.y - a.y) * (x - c.x) + (a.x - c.x) * (y - c.y)) / d;
        let l3 = 1.0 - l1 - l2;
        if l1 < 0.0 || l2 < 0.0 || l3 < 0.0 {
            continue;
        }
        let z = l1 * a.z + l2 * b.z + l3 * c.z;
        if z <= limit && best.is_none_or(|b| z > b) {
            best = Some(z);
        }
    }
    best
}

/// The seat's profile for a rider whose root is `root`: `[x, z]` samples, x ascending, the mean of the lines that hit something.
/// A sample with no hit on any line is left out.
pub fn profile(triangles: &[Triangle], root: Vec3) -> Vec<[f32; 2]> {
    let limit = root.z + ABOVE_ROOT_M;
    let count = ((BEHIND_M + AHEAD_M) / STEP_M).round() as usize;
    let mut out = Vec::new();
    for n in 0..=count {
        let x = root.x - BEHIND_M + n as f32 * STEP_M;
        let heights: Vec<f32> = LINES_M.iter().filter_map(|&y| height_at(triangles, (x, root.y + y), limit)).collect();
        if heights.is_empty() {
            continue;
        }
        out.push([x, heights.iter().sum::<f32>() / heights.len() as f32]);
    }
    out
}

/// The triangles of the body parts of `table` that can be near the seat (inside a box around `root`), in the car frame.
pub fn triangles(table: &PartTable, exports: &Exports, root: Vec3) -> Result<Vec<Triangle>> {
    let (low, high) = (
        Vec3::new(root.x - BEHIND_M - 0.05, root.y - 0.2, root.z - 0.6),
        Vec3::new(root.x + AHEAD_M + 0.05, root.y + 0.2, root.z + 0.6),
    );
    let mut out = Vec::new();
    for part in table.parts.iter().filter(|p| p.kind == Kind::Body && p.role == Role::Body) {
        let glb = exports.load(&part.export, 0)?;
        for primitive in &glb.primitives {
            let points: Vec<Vec3> = primitive.positions.iter().map(|&p| car_point(&part.anchor, p)).collect();
            for corner in primitive.indices.as_chunks::<3>().0 {
                let tri = [points[corner[0] as usize], points[corner[1] as usize], points[corner[2] as usize]];
                let (min, max) = (tri[0].min(tri[1]).min(tri[2]), tri[0].max(tri[1]).max(tri[2]));
                if max.cmpge(low).all() && min.cmple(high).all() {
                    out.push(tri);
                }
            }
        }
    }
    Ok(out)
}

/// The seat profile of the bike in `bike_dir` (`parts.json` and `glb/`) for a rider whose root is `root` (car frame).
pub fn surface(bike_dir: &Path, root: Vec3) -> Result<Vec<[f32; 2]>> {
    let table = PartTable::read(bike_dir)?;
    let exports = Exports::scan(&bike_dir.join("glb"))?;
    let tris = triangles(&table, &exports, root).context("reading the body meshes for the seat")?;
    let points = profile(&tris, root);
    if points.len() < 2 {
        bail!("no surface under the rider's root: {} triangles near it", tris.len());
    }
    Ok(points)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A flat quad of two triangles at height `z`, from x0..x1 and y -1..1.
    fn slab(x0: f32, x1: f32, z: f32) -> Vec<Triangle> {
        let p = |x: f32, y: f32| Vec3::new(x, y, z);
        vec![[p(x0, -1.0), p(x1, -1.0), p(x1, 1.0)], [p(x0, -1.0), p(x1, 1.0), p(x0, 1.0)]]
    }

    #[test]
    fn the_highest_surface_under_the_limit_is_the_seat() {
        let mut tris = slab(-1.0, 1.0, 0.80);
        tris.extend(slab(-1.0, 1.0, 0.70));
        // A tank-like roof above the limit is not the seat.
        tris.extend(slab(-1.0, 1.0, 1.10));
        assert_eq!(height_at(&tris, (0.0, 0.0), 0.95), Some(0.80));
        assert_eq!(height_at(&tris, (5.0, 0.0), 0.95), None, "off the mesh");
        assert_eq!(height_at(&tris, (0.0, 0.0), 0.60), None, "everything is above the limit");
    }

    #[test]
    fn the_profile_follows_the_seat_and_skips_what_has_no_surface() {
        let root = Vec3::new(-0.3, 0.0, 0.9);
        // The seat reaches 0.1 m ahead of the root only; behind it the whole profile has a surface.
        let tris = slab(-1.0, -0.2, 0.84);
        let points = profile(&tris, root);
        assert!(points.len() > 10);
        assert!((points[0][0] - (root.x - BEHIND_M)).abs() < 1e-5);
        assert!(points.iter().all(|p| (p[1] - 0.84).abs() < 1e-5));
        assert!(points.windows(2).all(|w| w[0][0] < w[1][0]), "ascending");
        assert!(points.last().unwrap()[0] <= -0.2 + 1e-4, "no sample past the end of the mesh");
    }

    #[test]
    fn the_lines_beside_the_centre_are_averaged_with_it() {
        let root = Vec3::new(0.0, 0.0, 0.9);
        // The centre line has a 2 cm ridge (a seat with a spine): the three lines average to a third of it.
        let mut tris = slab(-1.0, 1.0, 0.80);
        let ridge = |x0: f32, x1: f32| {
            let p = |x: f32, y: f32| Vec3::new(x, y, 0.82);
            vec![[p(x0, -0.03), p(x1, -0.03), p(x1, 0.03)], [p(x0, -0.03), p(x1, 0.03), p(x0, 0.03)]]
        };
        tris.extend(ridge(-1.0, 1.0));
        let points = profile(&tris, root);
        assert!((points[0][1] - (0.80 + 0.02 / 3.0)).abs() < 1e-5, "{:?}", points[0]);
    }

    #[test]
    fn a_part_is_moved_by_its_anchor_and_gltf_axes_become_the_car_frame() {
        let anchor =
            crate::partfile::Anchor { bone: None, translation: [0.1, 0.2, 0.3], rotation: [0.0, 0.0, 0.0, 1.0] };
        // glTF (x forward, y up, z right) (1, 2, 3) + (0.1, 0.2, 0.3) = (1.1, 2.2, 3.3) is car (1.1, -3.3, 2.2).
        assert!(car_point(&anchor, [1.0, 2.0, 3.0]).distance(Vec3::new(1.1, -3.3, 2.2)) < 1e-5);
    }
}
