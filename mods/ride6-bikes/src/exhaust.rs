//! Where a bike's exhaust ends: the outlets of its silencers, found in the silencer mesh, written as the pipe markers the game
//! starts its tail-pipe flames at (`docs/modding/ride6-bikes.md`, "The exhaust flames").
//!
//! The game lights a car's tail pipes at the markers `EXHAUST`, `LEFT_EXHAUST` and `RIGHT_EXHAUST` of its body solid, and the
//! marker's +z axis is the way the flame shoots (`docs/modding/car-customization.md`, "Tail pipes"). RIDE 6 has no outlet
//! socket: the bike's skeleton lists only `SOCKET_AUDIO_EX`, a point on the centre plane behind the silencer that the sound
//! comes from. The silencer is a mesh (`parts.json`, part `muffler`), so the outlet is read from its geometry. Every number below
//! is a **choice** of this importer (the data has no outlet), listed with its reason in `docs/modding/ride6-bikes.md`:
//!
//! 1. **Welded components.** The mesh's vertices that lie within [`WELD`] of each other are one vertex, and the triangles that
//!    share a vertex make one component (a can, an end cap, a hanger, a bolt). A component whose area is under [`KEEP_SHARE`] of the
//!    largest one's is a fitting and is dropped: it must not pull the outlet to a bracket.
//! 2. **Sides.** A component is on the left when the area-weighted mean of its triangles' lateral position is more than
//!    [`CENTRE_BAND`] to the left of the centre plane, on the right when more than that to the right, else in the centre. The
//!    components of one side are one silencer: a bike with two silencers gets two pipes, one with an underbelly pipe or a
//!    single central can gets one.
//! 3. **Axis.** The principal axis of a side's triangles (centres weighted by area, so the can's skin counts and the small fittings
//!    do not). A group shorter than [`MIN_ELONGATION`] times its width has no axis worth following: it is a pot or a collector
//!    (the Yamaha R7's silencer mesh holds one under the bike), not a can, and it is dropped when the mesh has a real can; when
//!    it is all the mesh has, its flame goes straight back.
//! 4. **Direction.** The axis is turned to point backwards. An axis that is nearly crosswise or upright (its backward share is under
//!    [`SIDEWAYS`]) is a side exit or a stack and snaps to out (away from the centre plane) or up.
//! 5. **Outlet.** The vertices within [`CAP_DEPTH`] of the can's length from its end along the direction are the end cap; the outlet
//!    is their mean position pushed out to the end plane, on the axis through the cap's centre.

use std::collections::{BTreeMap, HashMap};

use anyhow::Result;
use blackbox_hash::bstring_hash;
use blackbox_solid::PositionMarker;
use serde::{Deserialize, Serialize};

use crate::exports::Exports;
use crate::layout::Frame;
use crate::partfile::{PartTable, Role};

/// The parts of `parts.json` that are silencers (the header pipes, `exmanifold`, end in the silencer and are not an outlet).
pub const SILENCER_PARTS: [&str; 1] = ["muffler"];

/// Vertices closer than this, metres, are one vertex.
const WELD: f64 = 0.001;
/// A component with less than this share of the largest component's area is a fitting.
const KEEP_SHARE: f64 = 0.1;
/// A component this close to the centre plane (its mean lateral position), metres, is a centre pipe.
const CENTRE_BAND: f64 = 0.03;
/// A silencer at least this many times longer than wide has an axis.
const MIN_ELONGATION: f64 = 1.5;
/// The end cap is the vertices within this share of the can's length from its end.
const CAP_DEPTH: f64 = 0.06;
/// ...but at least this deep, metres.
const MIN_CAP_DEPTH: f64 = 0.005;
/// An axis with less than this backward share (the x of a unit vector) is a side exit or a stack.
const SIDEWAYS: f64 = 0.1;
/// Power iterations of the axis search.
const ITERATIONS: usize = 100;

/// A vector of the car frame (x forward, y left, z up).
type V = [f64; 3];

const BACKWARD: V = [-1.0, 0.0, 0.0];

/// Which side of the bike a pipe is on, which picks its marker.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Side {
    Left,
    Right,
    Centre,
}

impl Side {
    /// The marker the game starts a pipe's flame at.
    pub fn marker_name(self) -> &'static str {
        match self {
            Side::Left => "LEFT_EXHAUST",
            Side::Right => "RIGHT_EXHAUST",
            Side::Centre => "EXHAUST",
        }
    }
}

/// One outlet in the body solid's frame (x forward, y left, z up), metres.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Pipe {
    pub side: Side,
    pub at: [f32; 3],
    /// A unit vector: the way the flame shoots.
    pub dir: [f32; 3],
}

fn sub(a: V, b: V) -> V {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn dot(a: V, b: V) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn cross(a: V, b: V) -> V {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}

fn scale(a: V, k: f64) -> V {
    a.map(|c| c * k)
}

/// `a` as a unit vector; `None` for a vector with no length.
fn unit(a: V) -> Option<V> {
    let length = dot(a, a).sqrt();
    (length > 1e-9).then(|| scale(a, 1.0 / length))
}

fn point(p: [f32; 3]) -> V {
    p.map(f64::from)
}

/// A triangle of the mesh with what the search weighs it by.
struct Tri {
    corners: [usize; 3],
    centre: V,
    area: f64,
}

/// The triangles that have an area and indices inside `points`.
fn triangles_of(points: &[[f32; 3]], triangles: &[[u32; 3]]) -> Vec<Tri> {
    let mut out = Vec::new();
    for corners in triangles.iter().map(|t| t.map(|i| i as usize)) {
        if corners.iter().any(|&i| i >= points.len()) {
            continue;
        }
        let [a, b, c] = corners.map(|i| point(points[i]));
        let n = cross(sub(b, a), sub(c, a));
        let area = 0.5 * dot(n, n).sqrt();
        if area < 1e-12 {
            continue;
        }
        let centre = [(a[0] + b[0] + c[0]) / 3.0, (a[1] + b[1] + c[1]) / 3.0, (a[2] + b[2] + c[2]) / 3.0];
        out.push(Tri { corners, centre, area });
    }
    out
}

/// For each vertex, the number of the vertex it is welded to.
fn welded(points: &[[f32; 3]]) -> Vec<usize> {
    let mut seen: HashMap<[i64; 3], usize> = HashMap::new();
    points
        .iter()
        .map(|p| {
            let key = point(*p).map(|c| (c / WELD).round() as i64);
            let next = seen.len();
            *seen.entry(key).or_insert(next)
        })
        .collect()
}

fn find(parent: &mut [usize], mut i: usize) -> usize {
    while parent[i] != i {
        parent[i] = parent[parent[i]];
        i = parent[i];
    }
    i
}

/// The triangles of each connected component, by the welded vertex they share.
fn components(points: &[[f32; 3]], tris: &[Tri]) -> Vec<Vec<usize>> {
    let ids = welded(points);
    let mut parent: Vec<usize> = (0..ids.iter().max().map_or(0, |m| m + 1)).collect();
    for tri in tris {
        let [a, b, c] = tri.corners.map(|i| ids[i]);
        for (x, y) in [(a, b), (b, c)] {
            let (rx, ry) = (find(&mut parent, x), find(&mut parent, y));
            parent[rx] = ry;
        }
    }
    let mut by_root: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
    for (n, tri) in tris.iter().enumerate() {
        let root = find(&mut parent, ids[tri.corners[0]]);
        by_root.entry(root).or_default().push(n);
    }
    by_root.into_values().collect()
}

fn area_of(tris: &[Tri], members: &[usize]) -> f64 {
    members.iter().map(|&n| tris[n].area).sum()
}

/// The triangles of the components worth a pipe, split into left, right and centre.
fn sides(points: &[[f32; 3]], tris: &[Tri]) -> [Vec<usize>; 3] {
    let parts = components(points, tris);
    let largest = parts.iter().map(|members| area_of(tris, members)).fold(0.0, f64::max);
    let mut groups: [Vec<usize>; 3] = Default::default();
    for members in parts.iter().filter(|members| area_of(tris, members) >= KEEP_SHARE * largest) {
        let area = area_of(tris, members);
        let lateral = members.iter().map(|&n| tris[n].area * tris[n].centre[1]).sum::<f64>() / area;
        let slot = match lateral {
            y if y > CENTRE_BAND => 0,
            y if y < -CENTRE_BAND => 1,
            _ => 2,
        };
        groups[slot].extend(members);
    }
    groups
}

/// The area-weighted centre and principal axis of the triangles `members`.
fn centre_and_axis(tris: &[Tri], members: &[usize]) -> (V, V) {
    let total = area_of(tris, members);
    let mut centre = [0.0; 3];
    for &n in members {
        centre = std::array::from_fn(|k| centre[k] + tris[n].area * tris[n].centre[k] / total);
    }
    let mut covariance = [[0.0; 3]; 3];
    for &n in members {
        let d = sub(tris[n].centre, centre);
        for (i, row) in covariance.iter_mut().enumerate() {
            for (j, cell) in row.iter_mut().enumerate() {
                *cell += tris[n].area * d[i] * d[j] / total;
            }
        }
    }
    let mut axis = unit([0.8, 0.5, 0.3]).unwrap_or(BACKWARD);
    for _ in 0..ITERATIONS {
        let next = std::array::from_fn(|i| dot(covariance[i], axis));
        match unit(next) {
            Some(next) => axis = next,
            None => return (centre, BACKWARD),
        }
    }
    (centre, axis)
}

/// Two unit vectors that, with `axis`, make a frame.
fn across(axis: V) -> (V, V) {
    let first =
        unit(cross(axis, [0.0, 0.0, 1.0])).or_else(|| unit(cross(axis, [0.0, 1.0, 0.0]))).unwrap_or([0.0, 1.0, 0.0]);
    (first, cross(axis, first))
}

/// The way an outlet of a silencer along `axis` shoots (module comment, step 4).
fn outlet_direction(axis: V, side: Side) -> V {
    let back = if axis[0] > 0.0 { scale(axis, -1.0) } else { axis };
    if -back[0] >= SIDEWAYS {
        return back;
    }
    if back[2].abs() > back[1].abs() {
        return [0.0, 0.0, 1.0];
    }
    let out = match side {
        Side::Left => 1.0,
        Side::Right => -1.0,
        Side::Centre if back[1] < 0.0 => -1.0,
        Side::Centre => 1.0,
    };
    [0.0, out, 0.0]
}

/// The pipe of one side's triangles, and whether the group is a can (longer than it is wide).
fn pipe_of(points: &[[f32; 3]], tris: &[Tri], members: &[usize], side: Side) -> (Pipe, bool) {
    let (centre, axis) = centre_and_axis(tris, members);
    let mut vertices: Vec<usize> = members.iter().flat_map(|&n| tris[n].corners).collect();
    vertices.sort_unstable();
    vertices.dedup();
    let at = |i: usize| sub(point(points[i]), centre);
    let extent = |direction: V| {
        let (low, high) = vertices.iter().fold((f64::MAX, f64::MIN), |(low, high), &i| {
            let t = dot(at(i), direction);
            (low.min(t), high.max(t))
        });
        high - low
    };
    let (first, second) = across(axis);
    let width = extent(first).max(extent(second));
    let can = extent(axis) >= MIN_ELONGATION * width;
    let dir = match can {
        true => outlet_direction(axis, side),
        false => BACKWARD,
    };
    let along: Vec<f64> = vertices.iter().map(|&i| dot(at(i), dir)).collect();
    let (low, high) = along.iter().fold((f64::MAX, f64::MIN), |(low, high), &t| (low.min(t), high.max(t)));
    let depth = (CAP_DEPTH * (high - low)).max(MIN_CAP_DEPTH);
    let cap: Vec<(usize, f64)> = vertices.iter().copied().zip(along).filter(|&(_, t)| t >= high - depth).collect();
    let n = cap.len() as f64;
    let mean_t = cap.iter().map(|&(_, t)| t).sum::<f64>() / n;
    let mut mean = [0.0; 3];
    for &(i, _) in &cap {
        mean = std::array::from_fn(|k| mean[k] + point(points[i])[k] / n);
    }
    let outlet: V = std::array::from_fn(|k| mean[k] + dir[k] * (high - mean_t));
    (Pipe { side, at: outlet.map(|c| c as f32), dir: dir.map(|c| c as f32) }, can)
}

/// The outlets of a silencer mesh (`points` in the body solid's frame, `triangles` indices into them): one pipe for each side
/// the mesh has a silencer on. Empty for a mesh with no surface.
pub fn pipes(points: &[[f32; 3]], triangles: &[[u32; 3]]) -> Vec<Pipe> {
    let tris = triangles_of(points, triangles);
    if tris.is_empty() {
        return Vec::new();
    }
    let groups = sides(points, &tris);
    let found: Vec<(Pipe, bool)> = [Side::Left, Side::Right, Side::Centre]
        .into_iter()
        .zip(&groups)
        .filter(|(_, members)| !members.is_empty())
        .map(|(side, members)| pipe_of(points, &tris, members, side))
        .collect();
    let has_can = found.iter().any(|&(_, can)| can);
    found.into_iter().filter(|&(_, can)| can || !has_can).map(|(pipe, _)| pipe).collect()
}

/// The outlets of the stock silencer of `table`, read from its finest mesh and placed in the body solid's frame. Empty for a
/// bike that has no silencer part.
pub fn stock_pipes(table: &PartTable, exports: &Exports, frame: Frame) -> Result<Vec<Pipe>> {
    let (mut points, mut triangles) = (Vec::new(), Vec::new());
    for part in table.parts_of(Role::Body).filter(|part| SILENCER_PARTS.contains(&part.id.as_str())) {
        let glb = exports.load(&part.export, 0)?;
        for primitive in &glb.primitives {
            let base = points.len() as u32;
            points.extend(primitive.positions.iter().map(|&p| frame.body_point(part.anchor.point(p))));
            triangles
                .extend(primitive.indices.as_chunks::<3>().0.iter().map(|t| [base + t[0], base + t[1], base + t[2]]));
        }
    }
    Ok(pipes(&points, &triangles))
}

/// The marker of a pipe: its z axis is the way the flame shoots, its x axis points up (forward for a pipe that shoots up) and
/// its y axis completes a right-handed frame. Stored row-major: the three axes, then the position.
pub fn marker(pipe: &Pipe) -> PositionMarker {
    let z = point(pipe.dir);
    let reference = if z[2].abs() > 0.99 { [1.0, 0.0, 0.0] } else { [0.0, 0.0, 1.0] };
    let x = unit(sub(reference, scale(z, dot(reference, z)))).unwrap_or([1.0, 0.0, 0.0]);
    let y = cross(z, x);
    let [px, py, pz] = pipe.at;
    let row = |v: V, w: f32| [v[0] as f32, v[1] as f32, v[2] as f32, w];
    let mut matrix = [0.0; 16];
    for (k, part) in [row(x, 0.0), row(y, 0.0), row(z, 0.0), [px, py, pz, 1.0]].into_iter().enumerate() {
        matrix[k * 4..k * 4 + 4].copy_from_slice(&part);
    }
    PositionMarker { name_hash: bstring_hash(pipe.side.marker_name()), int_param: 0, float_params: [0.0; 2], matrix }
}

/// The markers of all pipes.
pub fn markers(pipes: &[Pipe]) -> Vec<PositionMarker> {
    pipes.iter().map(marker).collect()
}

#[cfg(test)]
#[path = "exhaust_tests.rs"]
mod tests;
