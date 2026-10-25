//! A 2012 car as the 2005 car files: `GEOMETRY.BIN` with the body and wheel solids at four levels of detail, and the
//! car's `TEXTURES.BIN`.
//!
//! The body keeps the 2012 shape, moved onto the stock hubs by [`Fit`]. The wheel is one solid (tyre, rim, brake disc
//! and caliper) made the way the game expects a "right-hand" wheel: axle along y, rim face at y = 0, tyre running to
//! +y. The game turns it for the left side.

use std::collections::HashMap;

use anyhow::{Context, Result, bail};
use blackbox_hash::bstring_hash;
use blackbox_solid::{GroupSource, PositionMarker, SolidSource, Vertex, write_geometry_pack};
use blackbox_tpk::write_tpk;
use chameleon_vehicle::{Lod, PartMesh, SurfaceKind};

use crate::fit::{Fit, to_car_space};
use crate::source::SourceCar;
use crate::textures::Palette;

/// The levels of detail the game asks for (it draws nothing for one a car lacks). The player's car uses `B`.
const LOD_LETTERS: [char; 4] = ['A', 'B', 'C', 'D'];
/// Which of the car's own levels each letter takes: the finest for the two the player sees, then coarser ones.
const LOD_SOURCE: [usize; 4] = [0, 0, 1, 2];
/// Vertices one solid may hold: the writer puts all groups of one effect in a single 16-bit-indexed buffer.
const MAX_SOLID_VERTICES: usize = 60_000;
const PAINT_FLAGS: u32 = 0x14180;

/// What the car files hold, and what is in them.
pub struct Built {
    pub geometry: Vec<u8>,
    pub textures: Vec<u8>,
    /// Triangles of the finest body level.
    pub triangles: usize,
}

pub struct Options {
    pub max_texture_size: u32,
}

/// Where a mesh's surface gets its look from in the 2005 car shader.
struct Look {
    texture: String,
    light_material: &'static str,
    flags: u32,
}

pub fn build(car: &SourceCar, fit: &Fit, base: &str, folder: &str, options: &Options) -> Result<Built> {
    let mut palette = Palette::default();
    let mut solids = Vec::new();
    let body = |p: [f32; 3]| fit.apply(to_car_space(p));
    let triangles = lod_for(&car.vehicle.body, 0, "body")?.iter().map(|mesh| mesh.triangles.len()).sum();
    add_solids(&mut solids, &format!("{base}_KIT00_BODY"), &car.vehicle.body, &body, Vec::new(), &mut |mesh| {
        look(mesh, car, base, options, &mut palette)
    })?;
    let wheel =
        car.vehicle.wheels.iter().find(|wheel| wheel.name.starts_with("FrontLeft")).or(car.vehicle.wheels.first());
    let wheel = wheel.context("the car has no wheels")?;
    let shift = wheel_shift(lod_for(&wheel.lods, 0, "wheel")?);
    let rim_up = |p: [f32; 3]| {
        let [x, y, z] = wheel_space(p);
        [x, y - shift, z]
    };
    let brake = PositionMarker {
        name_hash: bstring_hash("FRONT_BRAKE"),
        int_param: 0,
        float_params: [0.0, 0.0],
        matrix: [1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.046, 0.0, 1.0],
    };
    add_solids(&mut solids, &format!("{base}_KIT00_FRONT_TIRE"), &wheel.lods, &rim_up, vec![brake], &mut |mesh| {
        look(mesh, car, base, options, &mut palette)
    })?;
    let geometry = write_geometry_pack(&solids).context("writing GEOMETRY.BIN")?;
    let textures = write_tpk(folder, &palette.into_textures()).context("writing TEXTURES.BIN")?;
    Ok(Built { geometry, textures, triangles })
}

/// The wheel's own frame: the 2012 left wheel (axle along x, rim face towards +x) turned to car axes, then half a turn
/// about the up axis so the rim faces -y, which is how the game wants a right-hand wheel.
fn wheel_space(p: [f32; 3]) -> [f32; 3] {
    let [x, y, z] = to_car_space(p);
    [-x, -y, z]
}

/// How far to move the wheel along y so its outermost point (the rim face) sits at y = 0.
fn wheel_shift(finest: &Lod) -> f32 {
    finest.iter().flat_map(|mesh| mesh.vertices.iter()).map(|v| wheel_space(v.position)[1]).fold(f32::MAX, f32::min)
}

fn lod_for<'a>(lods: &'a [Lod], level: usize, what: &str) -> Result<&'a Lod> {
    lods.get(level).or(lods.last()).with_context(|| format!("the {what} has no meshes"))
}

/// The most detailed level at or after `wanted` that fits one solid.
fn pick_lod(lods: &[Lod], wanted: usize) -> Option<&Lod> {
    let fits = |lod: &&Lod| lod.iter().map(|mesh| mesh.vertices.len()).sum::<usize>() <= MAX_SOLID_VERTICES;
    let start = wanted.min(lods.len().saturating_sub(1));
    lods[start..].iter().find(fits)
}

fn add_solids(
    out: &mut Vec<SolidSource>,
    stem: &str,
    lods: &[Lod],
    transform: &dyn Fn([f32; 3]) -> [f32; 3],
    markers: Vec<PositionMarker>,
    look: &mut dyn FnMut(&PartMesh) -> Look,
) -> Result<()> {
    for (letter, wanted) in LOD_LETTERS.iter().zip(LOD_SOURCE) {
        let Some(lod) = pick_lod(lods, wanted) else { bail!("{stem}: no level of detail fits one solid") };
        let mut solid = solid_of(&format!("{stem}_{letter}"), lod, transform, look)?;
        solid.markers = markers.clone();
        out.push(solid);
    }
    Ok(())
}

fn solid_of(
    name: &str,
    lod: &Lod,
    transform: &dyn Fn([f32; 3]) -> [f32; 3],
    look: &mut dyn FnMut(&PartMesh) -> Look,
) -> Result<SolidSource> {
    let positions: Vec<Vec<[f32; 3]>> =
        lod.iter().map(|mesh| mesh.vertices.iter().map(|v| transform(v.position)).collect()).collect();
    let normals = smooth_normals(lod, &positions);
    let mut texture_hashes = Vec::new();
    let mut light_hashes = Vec::new();
    let mut groups = Vec::new();
    for (mesh_at, mesh) in lod.iter().enumerate() {
        if mesh.triangles.is_empty() {
            continue;
        }
        let vertices = mesh
            .vertices
            .iter()
            .enumerate()
            .map(|(n, v)| Vertex {
                position: positions[mesh_at][n],
                normal: normals[mesh_at][n],
                color_bgra: [255; 4],
                uv: v.uv,
            })
            .collect();
        let indices = mesh.triangles.iter().flatten().map(|&index| index as u16).collect();
        let look = look(mesh);
        let diffuse = slot_of(&mut texture_hashes, bstring_hash(&look.texture));
        let mut group = GroupSource::new(vertices, indices);
        group.texture_slots = [diffuse; 5];
        group.light_material = slot_of(&mut light_hashes, bstring_hash(look.light_material));
        group.flags = look.flags;
        groups.push(group);
    }
    if groups.is_empty() {
        bail!("{name}: no triangles");
    }
    let mut solid = SolidSource::new(name, groups);
    solid.texture_hashes = texture_hashes;
    solid.light_material_hashes = light_hashes;
    Ok(solid)
}

/// The index of `hash` in `list`, adding it when it is new.
fn slot_of(list: &mut Vec<u32>, hash: u32) -> u8 {
    let at = list.iter().position(|&known| known == hash).unwrap_or_else(|| {
        list.push(hash);
        list.len() - 1
    });
    at as u8
}

/// Vertex normals: the area-weighted mean of the faces around each position, shared across every mesh of the solid.
fn smooth_normals(lod: &Lod, positions: &[Vec<[f32; 3]>]) -> Vec<Vec<[f32; 3]>> {
    let key = |p: [f32; 3]| p.map(|c| (c * 2000.0).round() as i64);
    let mut sums: HashMap<[i64; 3], [f32; 3]> = HashMap::new();
    for (mesh, points) in lod.iter().zip(positions) {
        for &[a, b, c] in &mesh.triangles {
            let (pa, pb, pc) = (points[a as usize], points[b as usize], points[c as usize]);
            let face = cross(sub(pb, pa), sub(pc, pa));
            for p in [pa, pb, pc] {
                let sum = sums.entry(key(p)).or_default();
                *sum = [sum[0] + face[0], sum[1] + face[1], sum[2] + face[2]];
            }
        }
    }
    positions
        .iter()
        .map(|points| points.iter().map(|&p| normalised(sums.get(&key(p)).copied().unwrap_or_default())).collect())
        .collect()
}

fn sub(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}

fn normalised(v: [f32; 3]) -> [f32; 3] {
    let length = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    if length < 1e-12 {
        return [0.0, 0.0, 1.0];
    }
    [v[0] / length, v[1] / length, v[2] / length]
}

/// The 2005 texture and light material that stand in for a 2012 surface.
fn look(mesh: &PartMesh, car: &SourceCar, base: &str, options: &Options, palette: &mut Palette) -> Look {
    let appearance = &mesh.appearance;
    match appearance.kind {
        // The paint placeholder: the game swaps in the colour the player picked.
        SurfaceKind::Paint => Look { texture: format!("{base}_SKIN1"), light_material: "CARSKIN", flags: PAINT_FLAGS },
        SurfaceKind::Glass => Look { texture: palette.add_glass(), light_material: "WINDSHIELD", flags: 0x4180 },
        kind => {
            let (light_material, fallback) = match kind {
                SurfaceKind::Tyre => ("RUBBER", [0.03; 3]),
                SurfaceKind::Wheel => ("MAGSILVER", [0.4; 3]),
                SurfaceKind::Body => ("SHINYPLASTIC", [0.1; 3]),
                SurfaceKind::Light => ("MATTEPLASTIC", [0.8; 3]),
                _ => ("MATTEPLASTIC", [0.3; 3]),
            };
            let map = appearance.diffuse_texture.and_then(|id| {
                let texture = car.textures.get(&id)?;
                let name = format!("T{:X}", id & 0xFF_FFFF_FFFF);
                palette.add_map(&name, texture, options.max_texture_size, appearance.alpha_test)
            });
            // A lamp's constants colour its light, not its surface: they would paint a headlight red.
            let colour = appearance.diffuse_colour.filter(|_| kind != SurfaceKind::Light).unwrap_or(fallback);
            let texture = map.unwrap_or_else(|| palette.add_colour(colour));
            Look { texture, light_material, flags: 0x4180 }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wheel_space_turns_the_rim_outwards_to_minus_y() {
        // A point on the outer side of the 2012 left wheel (+x) ends up on -y; forward (+z) ends up on +x.
        assert_eq!(wheel_space([0.2, 0.1, 0.3]), [-0.3, -0.2, 0.1]);
    }

    #[test]
    fn a_closed_surface_keeps_outward_normals() {
        // Counter-clockwise triangle on the z = 0 plane seen from +z: its normal is +z.
        let lod = vec![PartMesh {
            vertices: [[0.0; 3], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]]
                .map(|position| chameleon_gfx::Vertex { position, color: [255; 4], uv: [0.0; 2] })
                .to_vec(),
            triangles: vec![[0, 1, 2]],
            appearance: chameleon_vehicle::Appearance {
                kind: SurfaceKind::Body,
                technique: String::new(),
                diffuse_texture: None,
                diffuse_colour: None,
                alpha_test: false,
            },
        }];
        let positions = vec![lod[0].vertices.iter().map(|v| v.position).collect::<Vec<_>>()];
        assert_eq!(smooth_normals(&lod, &positions)[0][0], [0.0, 0.0, 1.0]);
    }

    #[test]
    fn slots_are_reused() {
        let mut list = Vec::new();
        assert_eq!(slot_of(&mut list, 5), 0);
        assert_eq!(slot_of(&mut list, 9), 1);
        assert_eq!(slot_of(&mut list, 5), 0);
    }
}
