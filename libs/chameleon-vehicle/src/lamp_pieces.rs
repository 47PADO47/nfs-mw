//! Lamp surfaces cut into the pieces each lamp circuit lights. Spec: `docs/formats/chameleon-lamps.md`, "Lamp circuits".
//!
//! The 2012 shaders light a lamp from a picture, the lit pattern, whose four channels are four lamp circuits: red is the
//! brake, green the head lamps, blue the reverse lamps and alpha the tail lamps. Two techniques read it: `Refraction`
//! (the lens strips, split here into connected pieces) and `EmissiveFourChannel` (the lamp interiors and LED rows, one
//! mesh over the whole car, from which the triangles a channel lights are cut out). Each cut piece is marked as a
//! [`SurfaceKind::Lamp`] with its end, side and circuit, so a converter can light it when the circuit is on.

use std::collections::{BTreeMap, HashMap};

use chameleon_gfx::{PixelFormat, Texture};

use crate::appearance::{LampCircuit, LampEnd, LampSide, LampSurface, SurfaceKind};
use crate::vehicle::PartMesh;

/// The circuit of each channel of the lit pattern, red to alpha.
const CHANNELS: [LampCircuit; 4] = [LampCircuit::Brake, LampCircuit::Head, LampCircuit::Reverse, LampCircuit::Tail];
/// A channel is read as lit above this share of its brightest texel, and never below its floor (the alpha channel of the
/// 918 Spyder holds dim shapes, its tail lamps, with a peak of 45).
const RELATIVE_LEVEL: f32 = 0.35;
const COLOUR_FLOOR: u8 = 48;
const ALPHA_FLOOR: u8 = 16;
/// A triangle is sampled at least every this many texels along its longest edge, and at most [`MAX_STEPS`] times.
const SAMPLE_TEXELS: f32 = 2.0;
const MAX_STEPS: usize = 24;
/// A triangle sampled this many times or fewer is lit by a single lit sample; a larger one needs this share of them.
const SMALL_SAMPLES: usize = 15;
const LIT_SHARE: f32 = 0.03;
/// Half the width of the strip along the centre plane that counts as the middle of the car, metres.
const CENTRE_BAND: f32 = 0.12;

/// The lit pattern of a lamp material, decoded: which texels of each channel are lit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct EmissiveMap {
    width: usize,
    height: usize,
    rgba: Vec<u8>,
    /// The value from which each channel is lit; `None` for a channel with nothing in it (or no alpha channel).
    level: [Option<u8>; 4],
}

impl EmissiveMap {
    pub(crate) fn new(texture: &Texture) -> Option<Self> {
        let rgba = texture.to_rgba8()?;
        let has_alpha = matches!(
            texture.format,
            PixelFormat::Bc2 | PixelFormat::Bc3 | PixelFormat::Bc7 | PixelFormat::Rgba8 | PixelFormat::Bgra8
        );
        let level = std::array::from_fn(|channel| {
            if channel == 3 && !has_alpha {
                return None;
            }
            let peak = rgba.as_chunks::<4>().0.iter().map(|texel| texel[channel]).max().unwrap_or(0);
            let floor = if channel == 3 { ALPHA_FLOOR } else { COLOUR_FLOOR };
            (peak >= floor).then(|| ((f32::from(peak) * RELATIVE_LEVEL) as u8).max(floor))
        });
        let (width, height) = (texture.width as usize, texture.height as usize);
        (width > 0 && height > 0).then_some(Self { width, height, rgba, level })
    }

    /// Which channels are lit at `uv`. The picture is clamped at its edge: a lamp island at the border must not read the
    /// opposite side.
    fn lit_at(&self, uv: [f32; 2]) -> [bool; 4] {
        let texel = |value: f32, size: usize| ((value.clamp(0.0, 1.0) * size as f32) as usize).min(size - 1);
        let (x, y) = (texel(uv[0], self.width), texel(uv[1], self.height));
        let at = (y * self.width + x) * 4;
        std::array::from_fn(|channel| self.level[channel].is_some_and(|level| self.rgba[at + channel] >= level))
    }

    /// The circuit that lights the triangle with texture coordinates `uvs`: the channel lit at most of its samples.
    fn circuit(&self, uvs: [[f32; 2]; 3]) -> Option<LampCircuit> {
        let longest = (0..3)
            .map(|n| {
                let (a, b) = (uvs[n], uvs[(n + 1) % 3]);
                ((a[0] - b[0]) * self.width as f32).abs().max(((a[1] - b[1]) * self.height as f32).abs())
            })
            .fold(0.0, f32::max);
        let steps = ((longest / SAMPLE_TEXELS).ceil() as usize).clamp(2, MAX_STEPS);
        let (mut hits, mut total) = ([0usize; 4], 0usize);
        for i in 0..=steps {
            for j in 0..=(steps - i) {
                let (a, b) = (i as f32 / steps as f32, j as f32 / steps as f32);
                let c = 1.0 - a - b;
                let mix = |axis: usize| uvs[0][axis] * a + uvs[1][axis] * b + uvs[2][axis] * c;
                for (hit, lit) in hits.iter_mut().zip(self.lit_at([mix(0), mix(1)])) {
                    *hit += usize::from(lit);
                }
                total += 1;
            }
        }
        let needed = if total <= SMALL_SAMPLES { 1 } else { (total as f32 * LIT_SHARE).ceil() as usize };
        let (channel, best) = hits.iter().copied().enumerate().rev().max_by_key(|&(_, hit)| hit)?;
        (best >= needed.max(1)).then_some(CHANNELS[channel])
    }
}

/// One group of triangles of a mesh, by their position in `mesh.triangles`, and the lamp surface it is, `None` for the
/// triangles of an emissive mesh that no circuit lights.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LampGroup {
    pub triangles: Vec<usize>,
    pub surface: Option<LampSurface>,
}

/// The groups a lamp mesh is cut into, in the order they come out; `None` for a mesh that is not cut (it stays one mesh).
/// A lens is cut into its connected pieces, each by circuit; the lamp interiors (`EmissiveFourChannel`) into the triangles
/// of each circuit and side, and the rest of the mesh, which needs the lit pattern `map`.
pub(crate) fn lamp_groups(mesh: &PartMesh, map: Option<&EmissiveMap>) -> Option<Vec<LampGroup>> {
    if mesh.triangles.is_empty() {
        return None;
    }
    if mesh.appearance.kind == SurfaceKind::Glass && mesh.appearance.technique.contains("Refraction") {
        return Some(lens_groups(mesh, map));
    }
    if mesh.appearance.technique.contains("EmissiveFourChannel") {
        return interior_groups(mesh, map?);
    }
    None
}

/// Cut `mesh` into its [`lamp_groups`], one mesh each; a mesh that is not cut is returned as it is.
pub(crate) fn split(mesh: PartMesh, map: Option<&EmissiveMap>) -> Vec<PartMesh> {
    let Some(groups) = lamp_groups(&mesh, map) else { return vec![mesh] };
    groups.iter().map(|group| piece(&mesh, group)).collect()
}

/// The centre of a triangle: (x left, z forward) of the engine's vehicle space.
fn centre(mesh: &PartMesh, triangle: [u32; 3]) -> (f32, f32) {
    let at = |axis: usize| triangle.iter().map(|&v| mesh.vertices[v as usize].position[axis]).sum::<f32>() / 3.0;
    (at(0), at(2))
}

fn uvs(mesh: &PartMesh, triangle: [u32; 3]) -> [[f32; 2]; 3] {
    triangle.map(|v| mesh.vertices[v as usize].uv)
}

/// A circuit lights a lamp at its own end of the car only: a head lamp channel on a triangle at the rear is not read.
fn at_its_end(circuit: LampCircuit, z: f32) -> bool {
    circuit.end() == end_of(z)
}

fn end_of(z: f32) -> LampEnd {
    if z >= 0.0 { LampEnd::Front } else { LampEnd::Rear }
}

/// The side of the car a lamp at `x` (left) is on.
fn side_of(x: f32) -> LampSide {
    match x {
        x if x > CENTRE_BAND => LampSide::Left,
        x if x < -CENTRE_BAND => LampSide::Right,
        _ => LampSide::Centre,
    }
}

/// The triangles of the interiors that a circuit lights, one group per circuit and side, after the group of the others.
fn interior_groups(mesh: &PartMesh, map: &EmissiveMap) -> Option<Vec<LampGroup>> {
    let mut rest = Vec::new();
    let mut lit: BTreeMap<(LampCircuit, u8), (LampSide, Vec<usize>)> = BTreeMap::new();
    for (at, &triangle) in mesh.triangles.iter().enumerate() {
        let (x, z) = centre(mesh, triangle);
        let circuit = map.circuit(uvs(mesh, triangle)).filter(|&circuit| at_its_end(circuit, z));
        let Some(circuit) = circuit else {
            rest.push(at);
            continue;
        };
        let side = side_of(x);
        lit.entry((circuit, side as u8)).or_insert_with(|| (side, Vec::new())).1.push(at);
    }
    if lit.is_empty() {
        return None;
    }
    let mut groups = Vec::new();
    if !rest.is_empty() {
        groups.push(LampGroup { triangles: rest, surface: None });
    }
    for ((circuit, _), (side, triangles)) in lit {
        groups.push(LampGroup { triangles, surface: Some(LampSurface { end: circuit.end(), side, circuit }) });
    }
    Some(groups)
}

/// The connected pieces of a lens, cut again by circuit. A triangle takes the circuit its own texels say; one the pattern
/// does not light takes the main circuit of its piece, and a piece the pattern does not light at all (or a lens with no
/// pattern) takes the circuit of its end of the car, head at the front and brake at the rear.
fn lens_groups(mesh: &PartMesh, map: Option<&EmissiveMap>) -> Vec<LampGroup> {
    let mut pieces: HashMap<usize, Vec<usize>> = HashMap::new();
    let root = components(mesh);
    for (at, triangle) in mesh.triangles.iter().enumerate() {
        pieces.entry(root[triangle[0] as usize]).or_default().push(at);
    }
    let mut roots: Vec<usize> = pieces.keys().copied().collect();
    roots.sort_unstable();
    let mut groups = Vec::new();
    for key in roots {
        let triangles = pieces.remove(&key).unwrap_or_default();
        let own: Vec<Option<LampCircuit>> = triangles
            .iter()
            .map(|&at| {
                let triangle = mesh.triangles[at];
                let z = centre(mesh, triangle).1;
                map?.circuit(uvs(mesh, triangle)).filter(|&circuit| at_its_end(circuit, z))
            })
            .collect();
        let mut votes = [0usize; 4];
        for circuit in own.iter().flatten() {
            votes[*circuit as usize] += 1;
        }
        let main = CHANNELS.iter().copied().rev().max_by_key(|&circuit| votes[circuit as usize]);
        let main = main.filter(|&circuit| votes[circuit as usize] > 0);
        let fallback = main.unwrap_or_else(|| {
            let z = centre(mesh, mesh.triangles[triangles[0]]).1;
            if end_of(z) == LampEnd::Front { LampCircuit::Head } else { LampCircuit::Brake }
        });
        let mut by_circuit: BTreeMap<LampCircuit, Vec<usize>> = BTreeMap::new();
        for (&at, circuit) in triangles.iter().zip(&own) {
            by_circuit.entry(circuit.unwrap_or(fallback)).or_default().push(at);
        }
        for (circuit, triangles) in by_circuit {
            let surface = surface_of(mesh, &triangles, circuit);
            groups.push(LampGroup { triangles, surface: Some(surface) });
        }
    }
    groups
}

/// The surface of the triangles `triangles` of `mesh` lit by `circuit`: the end and side of their mean place.
fn surface_of(mesh: &PartMesh, triangles: &[usize], circuit: LampCircuit) -> LampSurface {
    let mut seen = std::collections::HashSet::new();
    let (mut x, mut z, mut count) = (0.0, 0.0, 0.0);
    for &at in triangles {
        for &v in &mesh.triangles[at] {
            if seen.insert(v) {
                let position = mesh.vertices[v as usize].position;
                (x, z, count) = (x + position[0], z + position[2], count + 1.0);
            }
        }
    }
    let count = f32::max(count, 1.0);
    LampSurface { end: end_of(z / count), side: side_of(x / count), circuit }
}

/// For each vertex, a number shared by every vertex of its connected piece (vertices at one place are one).
fn components(mesh: &PartMesh) -> Vec<usize> {
    let key = |p: [f32; 3]| p.map(|c| (c * 1000.0).round() as i64);
    let mut places: HashMap<[i64; 3], usize> = HashMap::new();
    let place: Vec<usize> = mesh
        .vertices
        .iter()
        .map(|v| {
            let next = places.len();
            *places.entry(key(v.position)).or_insert(next)
        })
        .collect();
    let mut parent: Vec<usize> = (0..places.len()).collect();
    fn find(parent: &mut [usize], mut x: usize) -> usize {
        while parent[x] != x {
            parent[x] = parent[parent[x]];
            x = parent[x];
        }
        x
    }
    for triangle in &mesh.triangles {
        let [a, b, c] = triangle.map(|v| place[v as usize]);
        for other in [b, c] {
            let (ra, ro) = (find(&mut parent, a), find(&mut parent, other));
            parent[ro] = ra;
        }
    }
    place.iter().map(|&p| find(&mut parent, p)).collect()
}

/// One group of a mesh as a mesh of its own, with compact vertices; a lit group is marked as the lamp surface it is.
fn piece(mesh: &PartMesh, group: &LampGroup) -> PartMesh {
    let mut remap: HashMap<u32, u32> = HashMap::new();
    let mut vertices = Vec::new();
    let mut triangles = Vec::with_capacity(group.triangles.len());
    for &at in &group.triangles {
        triangles.push(mesh.triangles[at].map(|v| {
            *remap.entry(v).or_insert_with(|| {
                vertices.push(mesh.vertices[v as usize]);
                (vertices.len() - 1) as u32
            })
        }));
    }
    let mut appearance = mesh.appearance.clone();
    if let Some(surface) = group.surface {
        appearance.kind = SurfaceKind::Lamp(surface);
    }
    PartMesh { vertices, triangles, appearance }
}

#[cfg(test)]
#[path = "lamp_pieces_tests.rs"]
mod tests;
