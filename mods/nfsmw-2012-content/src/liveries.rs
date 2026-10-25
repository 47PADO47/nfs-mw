//! Giving the paint surfaces of a 2012 car the texture layout of the stock car it clones.
//!
//! The game paints a car by swapping one texture, `<CAR>_SKIN1`, for a flat colour, and draws a vinyl (a livery) by
//! swapping it for the artwork laid over that colour. The artwork is authored in the clone's texture layout: the
//! hood is at one place of the picture, the doors at another. The 2012 body has its own layout, so its stripes would
//! land in the wrong places. Instead each point of the 2012 paint surface takes the texture coordinates of the nearest
//! point of the clone's paint surface (both are in the same car space once the fit has been applied), so a stripe that
//! runs over the clone's roof runs over the 2012 car's roof.
//!
//! The match is a nearest-point transfer, not an unwrapping: it follows the shape well on the large panels and is
//! rougher where the two cars differ, at the bumpers and around the lamps.

use std::collections::HashMap;

use blackbox_solid::Vertex;

/// Side of a cell of the search grid, in metres.
const CELL: f32 = 0.08;
/// How many rings of cells the search looks through before it gives up (about 3.8 m).
const MAX_RINGS: i32 = 48;
/// Largest distance between two corners of one triangle in texture space (the whole picture is 1). A triangle whose
/// corners land further apart straddles a seam of the clone's layout and takes one texel for all three corners.
const MAX_SPAN: f32 = 0.2;
/// Texture coordinates are told apart at this fraction when vertices are shared.
const UV_STEPS: f32 = 4096.0;

/// One triangle of the clone's paint surface: its corners in car space and their texture coordinates.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Triangle {
    pub corners: [[f32; 3]; 3],
    pub uv: [[f32; 2]; 3],
}

/// The clone's paint surface, searchable by position.
pub struct PaintMap {
    triangles: Vec<Triangle>,
    cells: HashMap<[i32; 3], Vec<u32>>,
}

impl PaintMap {
    /// Index `triangles`. A triangle with no area has no surface to match and is left out.
    pub fn new(triangles: Vec<Triangle>) -> Self {
        let triangles: Vec<Triangle> = triangles.into_iter().filter(|t| area(&t.corners) > 1e-9).collect();
        let mut cells: HashMap<[i32; 3], Vec<u32>> = HashMap::new();
        for (at, triangle) in triangles.iter().enumerate() {
            let (low, high) = bounds(&triangle.corners);
            let (low, high) = (cell_of(low), cell_of(high));
            for x in low[0]..=high[0] {
                for y in low[1]..=high[1] {
                    for z in low[2]..=high[2] {
                        cells.entry([x, y, z]).or_default().push(at as u32);
                    }
                }
            }
        }
        Self { triangles, cells }
    }

    pub fn is_empty(&self) -> bool {
        self.triangles.is_empty()
    }

    /// The texture coordinates of the clone's surface at the point nearest `p`, or `None` when no surface is near.
    pub fn uv_at(&self, p: [f32; 3]) -> Option<[f32; 2]> {
        let home = cell_of(p);
        let mut best: Option<(f32, usize, [f32; 3])> = None;
        for ring in 0..=MAX_RINGS {
            for cell in ring_cells(home, ring) {
                for &at in self.cells.get(&cell).into_iter().flatten() {
                    let (distance, weights) = closest_point(p, &self.triangles[at as usize].corners);
                    if best.is_none_or(|(known, ..)| distance < known) {
                        best = Some((distance, at as usize, weights));
                    }
                }
            }
            // A triangle in a cell outside this ring is at least `ring` cells away from p.
            if best.is_some_and(|(distance, ..)| distance.sqrt() <= ring as f32 * CELL) {
                break;
            }
        }
        let (_, at, [a, b, c]) = best?;
        let uv = self.triangles[at].uv;
        Some([a * uv[0][0] + b * uv[1][0] + c * uv[2][0], a * uv[0][1] + b * uv[1][1] + c * uv[2][1]])
    }

    /// Give the vertices of one group the clone's texture coordinates. A vertex is split in two where the triangles
    /// around it do not agree on its coordinates. `None` when the split group would no longer fit 16-bit indices.
    pub fn retarget(&self, vertices: &[Vertex], indices: &[u16]) -> Option<(Vec<Vertex>, Vec<u16>)> {
        let uvs: Vec<[f32; 2]> =
            vertices.iter().map(|vertex| self.uv_at(vertex.position).unwrap_or(vertex.uv)).collect();
        let mut out: Vec<Vertex> = Vec::with_capacity(vertices.len());
        let mut out_indices = Vec::with_capacity(indices.len());
        let mut known: HashMap<(u16, [i32; 2]), u16> = HashMap::new();
        for triangle in indices.as_chunks::<3>().0 {
            let mut uv: [[f32; 2]; 3] = std::array::from_fn(|corner| uvs[usize::from(triangle[corner])]);
            if span(&uv) > MAX_SPAN {
                let centre = centroid(triangle.iter().map(|&index| vertices[usize::from(index)].position));
                let flat = self.uv_at(centre).unwrap_or(uv[0]);
                uv = [flat; 3];
            }
            for (&index, uv) in triangle.iter().zip(uv) {
                let key = (index, uv.map(|c| (c * UV_STEPS).round() as i32));
                if let Some(&shared) = known.get(&key) {
                    out_indices.push(shared);
                    continue;
                }
                let next = u16::try_from(out.len()).ok()?;
                out.push(Vertex { uv, ..vertices[usize::from(index)] });
                known.insert(key, next);
                out_indices.push(next);
            }
        }
        Some((out, out_indices))
    }
}

fn cell_of(p: [f32; 3]) -> [i32; 3] {
    p.map(|c| (c / CELL).floor() as i32)
}

/// The cells at Chebyshev distance `ring` from `home` (just `home` for ring 0).
fn ring_cells(home: [i32; 3], ring: i32) -> impl Iterator<Item = [i32; 3]> {
    let steps = -ring..=ring;
    steps.clone().flat_map(move |x| {
        let steps = steps.clone();
        steps.clone().flat_map(move |y| {
            steps.clone().filter_map(move |z| {
                let on_shell = x.abs().max(y.abs()).max(z.abs()) == ring;
                on_shell.then_some([home[0] + x, home[1] + y, home[2] + z])
            })
        })
    })
}

fn bounds(corners: &[[f32; 3]; 3]) -> ([f32; 3], [f32; 3]) {
    let low = std::array::from_fn(|axis| corners.iter().map(|c| c[axis]).fold(f32::MAX, f32::min));
    let high = std::array::from_fn(|axis| corners.iter().map(|c| c[axis]).fold(f32::MIN, f32::max));
    (low, high)
}

/// The largest distance between two of the three texture coordinates.
fn span(uv: &[[f32; 2]; 3]) -> f32 {
    let apart = |a: [f32; 2], b: [f32; 2]| ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2)).sqrt();
    apart(uv[0], uv[1]).max(apart(uv[1], uv[2])).max(apart(uv[0], uv[2]))
}

fn centroid(points: impl Iterator<Item = [f32; 3]>) -> [f32; 3] {
    let (sum, count) =
        points.fold(([0.0; 3], 0.0), |(sum, count), p| ([sum[0] + p[0], sum[1] + p[1], sum[2] + p[2]], count + 1.0));
    sum.map(|c| c / f32::max(count, 1.0))
}

fn sub(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn area(corners: &[[f32; 3]; 3]) -> f32 {
    let (ab, ac) = (sub(corners[1], corners[0]), sub(corners[2], corners[0]));
    let cross = [ab[1] * ac[2] - ab[2] * ac[1], ab[2] * ac[0] - ab[0] * ac[2], ab[0] * ac[1] - ab[1] * ac[0]];
    dot(cross, cross).sqrt() / 2.0
}

/// The squared distance from `p` to the triangle, and the barycentric weights of the nearest point on it.
fn closest_point(p: [f32; 3], corners: &[[f32; 3]; 3]) -> (f32, [f32; 3]) {
    let [a, b, c] = *corners;
    let weights = barycentric_of_closest(p, a, b, c);
    let at: [f32; 3] = std::array::from_fn(|k| weights[0] * a[k] + weights[1] * b[k] + weights[2] * c[k]);
    let gap = sub(p, at);
    (dot(gap, gap), weights)
}

/// The weights of the point of triangle `abc` nearest `p` (Ericson, Real-Time Collision Detection, 5.1.5).
fn barycentric_of_closest(p: [f32; 3], a: [f32; 3], b: [f32; 3], c: [f32; 3]) -> [f32; 3] {
    let (ab, ac, ap) = (sub(b, a), sub(c, a), sub(p, a));
    let (d1, d2) = (dot(ab, ap), dot(ac, ap));
    if d1 <= 0.0 && d2 <= 0.0 {
        return [1.0, 0.0, 0.0];
    }
    let bp = sub(p, b);
    let (d3, d4) = (dot(ab, bp), dot(ac, bp));
    if d3 >= 0.0 && d4 <= d3 {
        return [0.0, 1.0, 0.0];
    }
    let vc = d1 * d4 - d3 * d2;
    if vc <= 0.0 && d1 >= 0.0 && d3 <= 0.0 {
        let v = d1 / (d1 - d3);
        return [1.0 - v, v, 0.0];
    }
    let cp = sub(p, c);
    let (d5, d6) = (dot(ab, cp), dot(ac, cp));
    if d6 >= 0.0 && d5 <= d6 {
        return [0.0, 0.0, 1.0];
    }
    let vb = d5 * d2 - d1 * d6;
    if vb <= 0.0 && d2 >= 0.0 && d6 <= 0.0 {
        let w = d2 / (d2 - d6);
        return [1.0 - w, 0.0, w];
    }
    let va = d3 * d6 - d5 * d4;
    if va <= 0.0 && d4 - d3 >= 0.0 && d5 - d6 >= 0.0 {
        let w = (d4 - d3) / ((d4 - d3) + (d5 - d6));
        return [0.0, 1.0 - w, w];
    }
    let denominator = 1.0 / (va + vb + vc);
    let (v, w) = (vb * denominator, vc * denominator);
    [1.0 - v - w, v, w]
}

#[cfg(test)]
mod tests;
