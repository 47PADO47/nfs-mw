//! A 2012 car as the 2005 car files: `GEOMETRY.BIN` with the body and wheel solids at four levels of detail, and the
//! car's `TEXTURES.BIN`.
//!
//! The body keeps the 2012 shape, moved onto the wheel layout the car is fitted to by [`Fit`]. The wheel is one solid
//! (tyre, rim, brake disc and caliper) made the way the game expects a "right-hand" wheel: axle along y, rim face at
//! y = 0, tyre running to +y. The game turns it for the left side. Every solid and texture is named after the car's own
//! folder (`<FOLDER>_KIT00_BODY_A`, `<FOLDER>_SKIN1`), and the paint surfaces keep the car's own texture coordinates.

use std::collections::HashMap;

use anyhow::{Context, Result, bail};
use blackbox_hash::bstring_hash;
use blackbox_solid::{GroupSource, PositionMarker, SolidSource, Vertex, write_geometry_pack};
use blackbox_tpk::write_tpk;
use chameleon_vehicle::{LampEnd, LampSide, LampSurface, Lod, PartMesh, SurfaceKind};

use crate::fit::{Fit, to_car_space};
use crate::hinges;
use crate::markers;
use crate::source::SourceCar;
use crate::textures::Palette;

/// The levels of detail the mod writes, as the letters of the solid names.
pub const LOD_LETTERS: [char; 4] = ['A', 'B', 'C', 'D'];
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
    /// The car's `BOUNDS.BIN` (`crate::collision`).
    pub bounds: Vec<u8>,
    /// The `[[damage_parts]]` tables of the car's manifest, empty when it has no hinged part (`crate::hinges`).
    pub damage_parts: String,
}

pub struct Options {
    pub max_texture_size: u32,
    /// The liveries of the car: each picture and the name it takes in the car's `TEXTURES.BIN`.
    pub liveries: Vec<(String, chameleon_gfx::Texture)>,
}

/// Where a mesh's surface gets its look from in the 2005 car shader.
struct Look {
    texture: String,
    light_material: &'static str,
    flags: u32,
}

/// Convert `car`, fitted by `fit`, as the car in `folder`. The body carries the tail pipes the nitrous flames come out of
/// ([`markers::tail_pipes`]); its paint surfaces use the placeholder texture `<FOLDER>_SKIN1`, which the game swaps for
/// the colour the player picks from the car's own paints (`paints.rs`).
pub fn build(car: &SourceCar, fit: &Fit, folder: &str, options: &Options) -> Result<Built> {
    let mut palette = Palette::default();
    let mut solids = Vec::new();
    let body = |p: [f32; 3]| fit.apply(to_car_space(p));
    let finest = lod_for(&car.vehicle.body, 0, "body")?;
    let triangles = finest.iter().map(|mesh| mesh.triangles.len()).sum();
    let body_points: Vec<[f32; 3]> =
        finest.iter().flat_map(|mesh| mesh.vertices.iter().map(|v| body(v.position))).collect();
    let mut body_marks = markers::tail_pipes(&body_points);
    body_marks.extend(markers::lamp_markers(&car.lamps, &body));
    let body_markers = |_: usize| body_marks.clone();
    add_solids(
        &mut solids,
        &format!("{folder}_KIT00_BODY"),
        &car.vehicle.body,
        &Parts { transform: &body, markers: &body_markers },
        &mut |mesh| look(mesh, car, folder, options, &mut palette),
    )?;
    let damage_parts = cut_hinged(&mut solids, car, fit, folder);
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
    let brake_marker = |_: usize| vec![brake];
    let wheel_parts = Parts { transform: &rim_up, markers: &brake_marker };
    add_solids(&mut solids, &format!("{folder}_KIT00_FRONT_TIRE"), &wheel.lods, &wheel_parts, &mut |mesh| {
        look(mesh, car, folder, options, &mut palette)
    })?;
    let geometry = write_geometry_pack(&solids).context("writing GEOMETRY.BIN")?;
    for (name, picture) in &options.liveries {
        palette.add_map(name, picture, options.max_texture_size, false);
    }
    let textures = write_tpk(folder, &palette.into_textures()).context("writing TEXTURES.BIN")?;
    let collision = crate::collision::build(car, &body, folder)?;
    log::debug!("{folder}: bounds from the {:?}, half dimensions {:?}", collision.basis, collision.half);
    Ok(Built { geometry, textures, triangles, bounds: collision.bytes, damage_parts })
}

/// Cut the hinged parts out of the finest body solids (`A` and `B`, which share the finest level) into solids of their
/// own, and return the `[[damage_parts]]` text that tells the game how they swing. Nothing is cut, and the text is empty,
/// when the car has no hinged part or its finest level does not fit one solid.
fn cut_hinged(solids: &mut Vec<SolidSource>, car: &SourceCar, fit: &Fit, folder: &str) -> String {
    let Some(hinged) = &car.hinged else { return String::new() };
    if !matches!(pick_lod(&car.vehicle.body, 0), Some((0, _))) {
        return String::new();
    }
    let stems: Vec<String> = hinged.parts.iter().map(|p| format!("{folder}_KIT00_{}", hinges::stem(&p.name))).collect();
    let mut cuts = Vec::new();
    for letter in ['A', 'B'] {
        let name = format!("{folder}_KIT00_BODY_{letter}");
        let Some(at) = solids.iter().position(|solid| solid.name == name) else { return String::new() };
        let Some(cut) = hinges::split(&solids[at], &stems, letter, &hinged.classes) else {
            log::warn!("{folder}: the hinged parts do not line up with the body: not cut");
            return String::new();
        };
        cuts.push((at, cut));
    }
    let owned: Vec<bool> = (0..stems.len()).map(|n| cuts.iter().all(|(_, (_, parts))| parts[n].is_some())).collect();
    for (at, (rest, parts)) in cuts {
        solids[at] = rest;
        solids.extend(parts.into_iter().zip(&owned).filter(|(_, owned)| **owned).filter_map(|(solid, _)| solid));
    }
    let kept = |n: usize| owned[n];
    let parts = hinged.parts.iter().enumerate().filter(|(n, _)| kept(*n)).map(|(_, part)| part.clone()).collect();
    let kept_stems: Vec<String> = stems.into_iter().enumerate().filter(|(n, _)| kept(*n)).map(|(_, s)| s).collect();
    hinges::manifest_tables(&hinges::Hinged { parts, classes: Vec::new() }, &kept_stems, fit)
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

/// The position and meshes of the most detailed level at or after `wanted` that fits one solid.
fn pick_lod(lods: &[Lod], wanted: usize) -> Option<(usize, &Lod)> {
    let fits =
        |(_, lod): &(usize, &Lod)| lod.iter().map(|mesh| mesh.vertices.len()).sum::<usize>() <= MAX_SOLID_VERTICES;
    let start = wanted.min(lods.len().saturating_sub(1));
    lods.iter().enumerate().skip(start).find(fits)
}

/// How the meshes of one part are carried into the game's car files.
struct Parts<'a> {
    /// A point of the 2012 part in 2005 car space.
    transform: &'a dyn Fn([f32; 3]) -> [f32; 3],
    /// The markers of the solid at a level of detail, by position in [`LOD_LETTERS`].
    markers: &'a dyn Fn(usize) -> Vec<PositionMarker>,
}

/// One solid per level of detail letter. Letters that land on the same level of the car share one build (the finest
/// level serves both `A` and `B`), which is a copy with another name, not another round of normals and buffers.
fn add_solids(
    out: &mut Vec<SolidSource>,
    stem: &str,
    lods: &[Lod],
    parts: &Parts,
    look: &mut dyn FnMut(&PartMesh) -> Look,
) -> Result<()> {
    let mut built: Vec<(usize, SolidSource)> = Vec::new();
    for (at, (letter, wanted)) in LOD_LETTERS.iter().zip(LOD_SOURCE).enumerate() {
        let Some((level, lod)) = pick_lod(lods, wanted) else { bail!("{stem}: no level of detail fits one solid") };
        let name = format!("{stem}_{letter}");
        let mut solid = match built.iter().find(|(known, _)| *known == level) {
            Some((_, solid)) => SolidSource { name, ..solid.clone() },
            None => {
                let solid = solid_of(&name, lod, parts.transform, look)?;
                built.push((level, solid.clone()));
                solid
            }
        };
        solid.markers = (parts.markers)(at);
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
        let vertices: Vec<Vertex> = mesh
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
        let indices: Vec<u16> = mesh.triangles.iter().flatten().map(|&index| index as u16).collect();
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

/// The placeholder texture of the game's lamps that a lens piece is drawn with. The front centre has none: it follows the
/// left lamp, and the head lamps of both sides look the same.
fn lamp_placeholder(surface: LampSurface) -> &'static str {
    match (surface.end, surface.side) {
        (LampEnd::Front, LampSide::Right) => "HEADLIGHT_RIGHT",
        (LampEnd::Front, _) => "HEADLIGHT_LEFT",
        (LampEnd::Rear, LampSide::Left) => "BRAKELIGHT_LEFT",
        (LampEnd::Rear, LampSide::Right) => "BRAKELIGHT_RIGHT",
        (LampEnd::Rear, LampSide::Centre) => "BRAKELIGHT_CENTRE",
    }
}

/// The 2005 texture and light material that stand in for a 2012 surface.
fn look(mesh: &PartMesh, car: &SourceCar, folder: &str, options: &Options, palette: &mut Palette) -> Look {
    let appearance = &mesh.appearance;
    match appearance.kind {
        // The paint placeholder: the game swaps in the colour the player picked.
        SurfaceKind::Paint => {
            Look { texture: format!("{folder}_SKIN1"), light_material: "CARSKIN", flags: PAINT_FLAGS }
        }
        SurfaceKind::Glass => Look { texture: palette.add_glass(), light_material: "WINDSHIELD", flags: 0x4180 },
        // A lit lens strip: the game's lamp placeholder, which it swaps for the car's own lit or unlit texture.
        SurfaceKind::Lamp(surface) => {
            palette.add_lamp_states(folder);
            Look { texture: lamp_placeholder(surface).to_owned(), light_material: "MATTEPLASTIC", flags: 0x4180 }
        }
        kind => {
            let (light_material, fallback) = match kind {
                SurfaceKind::Tyre => ("RUBBER", [0.03; 3]),
                SurfaceKind::Wheel => ("MAGSILVER", [0.4; 3]),
                SurfaceKind::Body => ("SHINYPLASTIC", [0.1; 3]),
                SurfaceKind::Light => ("MATTEPLASTIC", [0.1; 3]),
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
