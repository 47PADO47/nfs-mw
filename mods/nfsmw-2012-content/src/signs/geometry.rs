//! The geometry of the conversion: meshes in the 2012 engine's axes turned and fitted into the box of the 2005 solid each takes the
//! place of, a mast arm cut free of its signal heads, and the signal head hung where the 2005 world hangs its own.
//!
//! Every function works on [`Mesh`]es: positions, texture coordinates and triangles of one picture. Winding is kept by everything
//! here (the axes are permuted cyclically, turned about the vertical axis and scaled by positive factors), so a model that is drawn
//! from its front in the 2012 game is drawn from its front in the 2005 game.

use std::collections::HashMap;

use super::table::{Box3, Front, Hanging};
use crate::fit::to_car_space;

/// What a mesh is textured with: one of the 2012 pictures, by resource id, or a flat colour (linear, 0 to 1).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Picture {
    Atlas(u64),
    Colour([f32; 3]),
}

/// A mesh: the vertices a set of triangles uses, with the picture they are drawn with.
#[derive(Debug, Clone, PartialEq)]
pub struct Mesh {
    pub picture: Picture,
    /// Whether the picture has a cut-out alpha (the plate of a round sign, a lens visor): drawn punch-through from both sides.
    pub cut_out: bool,
    pub positions: Vec<[f32; 3]>,
    pub uvs: Vec<[f32; 2]>,
    pub triangles: Vec<[u32; 3]>,
}

/// The box of the vertices of `meshes`; `None` when there are none.
pub fn bounds(meshes: &[Mesh]) -> Option<Box3> {
    let mut points = meshes.iter().flat_map(|mesh| mesh.positions.iter());
    let first = *points.next()?;
    let (mut low, mut high) = (first, first);
    for point in points {
        for axis in 0..3 {
            low[axis] = low[axis].min(point[axis]);
            high[axis] = high[axis].max(point[axis]);
        }
    }
    Some(Box3 { low, high })
}

fn map_positions(meshes: &[Mesh], f: impl Fn([f32; 3]) -> [f32; 3]) -> Vec<Mesh> {
    meshes
        .iter()
        .map(|mesh| Mesh { positions: mesh.positions.iter().map(|&p| f(p)).collect(), ..mesh.clone() })
        .collect()
}

/// 2012 axes (x left, y up, z forward) to 2005 axes (x forward, y left, z up).
pub fn to_old_axes(meshes: &[Mesh]) -> Vec<Mesh> {
    map_positions(meshes, to_car_space)
}

/// Turn a model about the vertical axis, in 2005 axes, so that its front (which looks along +x for a 2012 model that faces +z, and
/// along -x for one that faces -z) looks along -y, where the 2005 signs show their picture.
pub fn face_minus_y(meshes: &[Mesh], front: Front) -> Vec<Mesh> {
    map_positions(meshes, |[x, y, z]| match front {
        Front::PlusZ => [y, -x, z],
        Front::MinusZ => [-y, x, z],
    })
}

/// Scale each axis and move: `p * scale + offset`.
pub fn scale_move(meshes: &[Mesh], scale: [f32; 3], offset: [f32; 3]) -> Vec<Mesh> {
    map_positions(meshes, |p| [0, 1, 2].map(|axis| p[axis] * scale[axis] + offset[axis]))
}

/// A model scaled evenly so that it is as tall as `old` and its plate no wider than `old` allows (`width_share` times the old width),
/// centred on `old` across and along, its lowest point on the old lowest point. For a sign on its pole.
pub fn stand_in(meshes: &[Mesh], old: &Box3, width_share: f32) -> Vec<Mesh> {
    let Some(now) = bounds(meshes) else { return Vec::new() };
    let size = |b: &Box3, axis: usize| (b.high[axis] - b.low[axis]).max(1e-3);
    let scale = (size(old, 2) / size(&now, 2)).min(width_share * size(old, 0) / size(&now, 0));
    let centre = |b: &Box3, axis: usize| 0.5 * (b.low[axis] + b.high[axis]);
    let offset = [
        centre(old, 0) - scale * centre(&now, 0),
        centre(old, 1) - scale * centre(&now, 1),
        old.low[2] - scale * now.low[2],
    ];
    scale_move(meshes, [scale; 3], offset)
}

/// A model stretched to fill `old` on every axis (a mast arm: seven metres long on the 2005 pole with the long arm, 3.5 on the other),
/// the pole's own side (the end of the arm that is at 0 along `y`) staying where it is.
pub fn fill(meshes: &[Mesh], old: &Box3) -> Vec<Mesh> {
    let Some(now) = bounds(meshes) else { return Vec::new() };
    let mut scale = [1.0; 3];
    let mut offset = [0.0; 3];
    for axis in 0..3 {
        let (n, o) = (now.high[axis] - now.low[axis], old.high[axis] - old.low[axis]);
        scale[axis] = o / n.max(1e-3);
        offset[axis] = old.low[axis] - scale[axis] * now.low[axis];
    }
    scale_move(meshes, scale, offset)
}

/// How much room the host gives a model that is used as it is (`place::exact`): half of `spread - 1` of the old size on each side, and
/// a few centimetres. Kept a little inside, so rounding never throws a model out.
fn room(old: &Box3, axis: usize, spread: f32) -> f32 {
    0.5 * (spread - 1.0) * (old.high[axis] - old.low[axis]) + 0.05 - 0.01
}

/// A model squeezed on any axis on which it would stick out of `old` by more than the room the host allows, so that it is accepted as
/// it is: the axis is scaled down about its middle until it fits. A model that fits is returned as it is.
pub fn squeeze_into(meshes: &[Mesh], old: &Box3, spread: f32) -> Vec<Mesh> {
    let Some(now) = bounds(meshes) else { return Vec::new() };
    let mut scale = [1.0f32; 3];
    let mut offset = [0.0f32; 3];
    for axis in 0..3 {
        let (low, high) = (old.low[axis] - room(old, axis, spread), old.high[axis] + room(old, axis, spread));
        let inside = now.low[axis] >= low && now.high[axis] <= high;
        if inside {
            continue;
        }
        let (middle, half) = (0.5 * (now.low[axis] + now.high[axis]), 0.5 * (now.high[axis] - now.low[axis]));
        let target = 0.5 * (high - low);
        scale[axis] = (target / half.max(1e-4)).min(1.0);
        let wanted = middle.max(low + target).min(high - target);
        offset[axis] = wanted - scale[axis] * middle;
    }
    scale_move(meshes, scale, offset)
}

fn find(parent: &mut [usize], mut at: usize) -> usize {
    while parent[at] != at {
        parent[at] = parent[parent[at]];
        at = parent[at];
    }
    at
}

fn join(parent: &mut [usize], a: usize, b: usize) {
    let (a, b) = (find(parent, a), find(parent, b));
    if a != b {
        parent[a] = b;
    }
}

/// The triangles of `mesh` grouped by the surface they belong to: two triangles are in one group when they share a position. Groups are
/// returned as lists of triangle numbers, in the order of their first triangle.
fn surfaces(mesh: &Mesh) -> Vec<Vec<usize>> {
    let mut parent: Vec<usize> = (0..mesh.positions.len()).collect();
    let mut first: HashMap<[i32; 3], usize> = HashMap::new();
    for (at, position) in mesh.positions.iter().enumerate() {
        let key = position.map(|value| (value * 1000.0).round() as i32);
        let known = *first.entry(key).or_insert(at);
        join(&mut parent, at, known);
    }
    for triangle in &mesh.triangles {
        join(&mut parent, triangle[0] as usize, triangle[1] as usize);
        join(&mut parent, triangle[1] as usize, triangle[2] as usize);
    }
    let mut order: Vec<usize> = Vec::new();
    let mut by_root: HashMap<usize, Vec<usize>> = HashMap::new();
    for (number, triangle) in mesh.triangles.iter().enumerate() {
        let at = find(&mut parent, triangle[0] as usize);
        by_root
            .entry(at)
            .or_insert_with(|| {
                order.push(at);
                Vec::new()
            })
            .push(number);
    }
    order.into_iter().filter_map(|at| by_root.remove(&at)).collect()
}

/// The mesh made of the triangles `numbers` of `mesh`, keeping only the vertices they use.
fn subset(mesh: &Mesh, numbers: &[usize]) -> Mesh {
    let mut renumber: HashMap<u32, u32> = HashMap::new();
    let mut out = Mesh { positions: Vec::new(), uvs: Vec::new(), triangles: Vec::new(), ..mesh.clone() };
    for &number in numbers {
        let mut triangle = [0; 3];
        for (slot, &index) in triangle.iter_mut().zip(&mesh.triangles[number]) {
            *slot = *renumber.entry(index).or_insert_with(|| {
                out.positions.push(mesh.positions[index as usize]);
                out.uvs.push(mesh.uvs[index as usize]);
                (out.positions.len() - 1) as u32
            });
        }
        out.triangles.push(triangle);
    }
    out
}

/// The longest side of the box of the points `numbers` of `mesh` use, and the lowest point, in 2012 axes (y up).
fn surface_size(mesh: &Mesh, numbers: &[usize]) -> (f32, f32) {
    let part = subset(mesh, numbers);
    let Some(b) = bounds(std::slice::from_ref(&part)) else { return (0.0, 0.0) };
    let longest = (0..3).map(|axis| b.high[axis] - b.low[axis]).fold(0.0, f32::max);
    (longest, b.low[1])
}

/// What the cut of a mast arm leaves and what it takes.
pub struct Cut {
    /// The pole, the arm and its struts, in 2012 axes.
    pub arm: Mesh,
    /// The surfaces taken away: signal heads, their lenses and hangers, the pedestrian head.
    pub taken: Vec<Mesh>,
}

/// The surfaces higher than this above the ground (metres, 2012 axes) that are no longer than [`HEAD_LONGEST`] are signal equipment.
/// The pole's own pieces reach the ground; the arm and its struts are metres long; a signal head, its lenses and its hangers hang
/// from the arm and are about a metre or less.
const HEAD_FLOOR: f32 = 1.5;
const HEAD_LONGEST: f32 = 1.1;

/// Cut the mast arm `mesh` into the arm and the equipment that hangs from it (the heads are separate objects in the 2005 world).
pub fn cut_heads(mesh: &Mesh) -> Cut {
    let mut keep = Vec::new();
    let mut take = Vec::new();
    for numbers in surfaces(mesh) {
        let (longest, low) = surface_size(mesh, &numbers);
        match low > HEAD_FLOOR && longest <= HEAD_LONGEST {
            true => take.push(subset(mesh, &numbers)),
            false => keep.extend(numbers),
        }
    }
    keep.sort_unstable();
    Cut { arm: subset(mesh, &keep), taken: take }
}

/// The measured spacing of the three lenses of the 2012 signal head, metres.
pub const LENS_PITCH: f32 = 0.272;

/// A signal head of the 2012 game, hung as the 2005 world hangs its own (`hanging`): its middle lens `hanging.centre` below the
/// origin, a lens every `hanging.pitch` metres, the housing's back on `back` along x and the head turned to face +x (as the 2005
/// heads and the host assume, `LAMP_FACING`). `head` is the head's meshes in 2012 axes. Also returns the point of the red lens, on
/// the front of its visor, in the same axes: where the authored red flare of the head goes.
pub fn hang(head: &[Mesh], hanging: Hanging, back: f32) -> Option<(Vec<Mesh>, [f32; 3])> {
    let housing = head
        .iter()
        .flat_map(|mesh| surfaces(mesh).into_iter().map(move |numbers| (mesh, numbers)))
        .max_by_key(|(_, numbers)| numbers.len())?;
    let part = subset(housing.0, &housing.1);
    let b = bounds(std::slice::from_ref(&part))?;
    let (middle_y, back_z, middle_x) = (0.5 * (b.low[1] + b.high[1]), b.low[2], 0.5 * (b.low[0] + b.high[0]));
    let scale = hanging.pitch / LENS_PITCH;
    // 2012 (x left, y up, z forward) -> 2005 (x forward = z, y left = x, z up = y), then scaled and moved.
    let place = |p: [f32; 3]| {
        [(p[2] - back_z) * scale + back, (p[0] - middle_x) * scale, (p[1] - middle_y) * scale + hanging.centre]
    };
    let hung: Vec<Mesh> = head
        .iter()
        .map(|mesh| Mesh { positions: mesh.positions.iter().map(|&p| place(p)).collect(), ..mesh.clone() })
        .collect();
    let front = bounds(&hung)?.high[0];
    Some((hung, [front, 0.0, hanging.centre + hanging.pitch]))
}

/// A thin square rod from `top` down to `bottom` (z) at the origin of x and y, with a flat colour: the hanger that joins a signal head to
/// the arm it hangs from.
pub fn rod(top: f32, bottom: f32, half: f32, colour: [f32; 3]) -> Mesh {
    let corners = [[-half, -half], [half, -half], [half, half], [-half, half]];
    let mut positions = Vec::new();
    for z in [bottom, top] {
        positions.extend(corners.iter().map(|c| [c[0], c[1], z]));
    }
    let uvs = vec![[0.5, 0.5]; positions.len()];
    let mut triangles = Vec::new();
    for side in 0..4u32 {
        let next = (side + 1) % 4;
        // Outward faces of the four sides, counter-clockwise seen from outside.
        triangles.push([side, next, 4 + next]);
        triangles.push([side, 4 + next, 4 + side]);
    }
    Mesh { picture: Picture::Colour(colour), cut_out: false, positions, uvs, triangles }
}

#[cfg(test)]
#[path = "geometry_tests.rs"]
mod tests;
