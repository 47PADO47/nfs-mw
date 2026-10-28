//! A bike as the 2005 car files: `GEOMETRY.BIN` with the body solid at four levels of detail, `TEXTURES.BIN` and
//! `BOUNDS.BIN`.
//!
//! The game draws a car as a body and four copies of one wheel solid. A bike has two wheels, so the importer puts the
//! whole bike (frame, wheels, brakes, exhaust) into the body solid and gives the wheel solid a single hidden triangle:
//! the wheels do not turn, and the two corners of each axle draw nothing.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use blackbox_attrib::vlt_hash;
use blackbox_collision::{BodyLimits, FitOptions, fit_body, write_bounds_pack};
use blackbox_hash::bstring_hash;
use blackbox_solid::{GroupSource, SolidSource, Vertex, write_geometry_pack};
use blackbox_tpk::write_tpk;

use crate::glb::{Glb, Material};
use crate::layout::{self, Kind, LEVELS, PARTS, Part};
use crate::textures::Palette;

/// Vertices one solid may hold: the writer puts all groups of one effect in one buffer with 16-bit indices.
const MAX_SOLID_VERTICES: usize = 60_000;
/// `simsurface` key of `carbody`, the surface the body nodes of a car's bounds have.
const CARBODY_SURFACE: u32 = 0x99DA_6531;
const GROUP_FLAGS: u32 = 0x4180;

/// What the car's files hold, and what the manifest needs to know.
pub struct Built {
    pub geometry: Vec<u8>,
    pub textures: Vec<u8>,
    pub bounds: Vec<u8>,
    /// Half dimensions of the bounds' root box (x across, y up, z along), metres.
    pub half: [f32; 3],
    pub tyre_radius: f32,
    pub triangles: usize,
}

/// The `.glb` files of an export folder, by lower-case file name.
pub struct Exports {
    files: HashMap<String, PathBuf>,
}

impl Exports {
    pub fn scan(root: &Path) -> Result<Self> {
        let mut files = HashMap::new();
        let mut pending = vec![root.to_path_buf()];
        while let Some(dir) = pending.pop() {
            for entry in std::fs::read_dir(&dir).with_context(|| format!("reading {}", dir.display()))? {
                let path = entry?.path();
                if path.is_dir() {
                    pending.push(path);
                    continue;
                }
                let Some(name) = path.file_name().and_then(|n| n.to_str()) else { continue };
                if name.to_ascii_lowercase().ends_with(".glb") {
                    files.insert(name.to_ascii_lowercase(), path);
                }
            }
        }
        Ok(Self { files })
    }

    /// The file of `stem` at level of detail `level`, or the nearest coarser-numbered level below it that exists.
    fn find(&self, stem: &str, level: u32) -> Option<&PathBuf> {
        (0..=level).rev().find_map(|n| {
            let name = if n == 0 { format!("{stem}.glb") } else { format!("{stem}_lod{n}.glb") };
            self.files.get(&name.to_ascii_lowercase())
        })
    }

    fn load(&self, stem: &str, level: u32) -> Result<Glb> {
        let path =
            self.find(stem, level).with_context(|| format!("no export of {stem} (level {level}) in the folder"))?;
        let bytes = std::fs::read(path).with_context(|| format!("reading {}", path.display()))?;
        Glb::parse(&bytes).with_context(|| format!("reading {}", path.display()))
    }
}

/// Where the bike sits in the car's frame: the middle of the wheelbase on the origin, the tyres on the ground.
struct Frame {
    centre: f32,
    lift: f32,
}

impl Frame {
    fn point(&self, part: &Part, p: [f32; 3]) -> [f32; 3] {
        layout::to_car([p[0] + part.offset[0], p[1] + part.offset[1], p[2] + part.offset[2]], self.centre, self.lift)
    }
}

/// The lowest and highest height of a tyre of the bike, as the export frame has it.
fn tyre_heights(exports: &Exports) -> Result<(f32, f32)> {
    let (mut low, mut high) = (f32::MAX, f32::MIN);
    for part in PARTS.iter().filter(|part| part.kind == Kind::Tyre) {
        let glb = exports.load(part.stem, 0)?;
        for p in glb.primitives.iter().flat_map(|primitive| &primitive.positions) {
            low = low.min(p[1] + part.offset[1]);
            high = high.max(p[1] + part.offset[1]);
        }
    }
    if low > high {
        bail!("the tyres have no vertices");
    }
    Ok((low, high))
}

/// Build the car files of `folder` from the exports in `exports`; pictures are cut to `max_texture` pixels.
pub fn build(exports: &Exports, folder: &str, max_texture: u32) -> Result<Built> {
    let (low, _) = tyre_heights(exports)?;
    let frame = Frame { centre: layout::wheelbase_centre(), lift: -low };
    let mut palette = Palette::default();
    let mut solids = Vec::new();
    let mut finest: Option<Vec<[f32; 3]>> = None;
    let mut triangles = 0;
    for (letter, chassis_level, part_level) in LEVELS {
        let name = format!("{folder}_KIT00_BODY_{letter}");
        let (solid, points) =
            body_solid(exports, &frame, &name, folder, (chassis_level, part_level), &mut palette, max_texture)?;
        if finest.is_none() {
            triangles = solid.groups.iter().map(|g| g.indices.len() / 3).sum();
            finest = Some(points);
        }
        solids.push(solid);
    }
    let hidden = palette.add_colour([0, 0, 0]);
    for (letter, ..) in LEVELS {
        solids.push(hidden_wheel(&format!("{folder}_KIT00_FRONT_TIRE_{letter}"), &hidden));
    }
    let geometry = write_geometry_pack(&solids).context("writing GEOMETRY.BIN")?;
    let textures = write_tpk(folder, &palette.into_textures()).context("writing TEXTURES.BIN")?;
    let (bounds, half) = bounds(&finest.unwrap_or_default(), folder)?;
    let (low, high) = tyre_heights(exports)?;
    Ok(Built { geometry, textures, bounds, half, tyre_radius: (high - low) / 2.0, triangles })
}

fn body_solid(
    exports: &Exports,
    frame: &Frame,
    name: &str,
    folder: &str,
    (chassis_level, part_level): (u32, u32),
    palette: &mut Palette,
    max_texture: u32,
) -> Result<(SolidSource, Vec<[f32; 3]>)> {
    let mut groups = Vec::new();
    let mut textures = Vec::new();
    let mut lights = Vec::new();
    let mut points = Vec::new();
    for part in &PARTS {
        let level = if part.chassis { chassis_level } else { part_level };
        let glb = exports.load(part.stem, level)?;
        for primitive in &glb.primitives {
            if primitive.indices.is_empty() {
                continue;
            }
            let look = look(part, &glb, &glb.materials[primitive.material], folder, palette, max_texture)?;
            let vertices: Vec<Vertex> = primitive
                .positions
                .iter()
                .zip(&primitive.normals)
                .zip(&primitive.uvs)
                .map(|((&p, &n), &uv)| Vertex {
                    position: frame.point(part, p),
                    normal: layout::direction_to_car(n),
                    color_bgra: [255; 4],
                    uv,
                })
                .collect();
            points.extend(vertices.iter().map(|v| v.position));
            let indices: Vec<u16> = primitive.indices.iter().map(|&i| i as u16).collect();
            let mut group = GroupSource::new(vertices, indices);
            let diffuse = slot_of(&mut textures, bstring_hash(&look.texture));
            group.texture_slots = [diffuse; 5];
            group.light_material = slot_of(&mut lights, bstring_hash(look.light_material));
            group.flags = GROUP_FLAGS;
            groups.push(group);
        }
    }
    let count: usize = groups.iter().map(|g| g.vertices.len()).sum();
    if count > MAX_SOLID_VERTICES {
        bail!("{name}: {count} vertices do not fit one solid ({MAX_SOLID_VERTICES}); use a coarser level of detail");
    }
    let mut solid = SolidSource::new(name, groups);
    solid.texture_hashes = textures;
    solid.light_material_hashes = lights;
    Ok((solid, points))
}

/// The wheel solid of a bike: one hidden triangle, so the car has a wheel part and draws nothing at its corners.
fn hidden_wheel(name: &str, texture: &str) -> SolidSource {
    let corner =
        |position: [f32; 3]| Vertex { position, normal: [0.0, 1.0, 0.0], color_bgra: [255; 4], uv: [0.0, 0.0] };
    let vertices = vec![corner([0.0, 0.0, 0.0]), corner([0.001, 0.0, 0.0]), corner([0.0, 0.0, 0.001])];
    let mut group = GroupSource::new(vertices, vec![0, 1, 2]);
    group.texture_slots = [0; 5];
    group.light_material = 0;
    let mut solid = SolidSource::new(name, vec![group]);
    solid.texture_hashes = vec![bstring_hash(texture)];
    solid.light_material_hashes = vec![bstring_hash("RUBBER")];
    solid
}

struct Look {
    texture: String,
    light_material: &'static str,
}

/// The texture and the shader of a surface: its picture when it has one, else a flat colour for what it is made of.
fn look(
    part: &Part,
    glb: &Glb,
    material: &Material,
    folder: &str,
    palette: &mut Palette,
    max_texture: u32,
) -> Result<Look> {
    let light_material = match part.kind {
        Kind::Tyre => "RUBBER",
        Kind::Rim | Kind::Brake | Kind::Metal => "MAGSILVER",
        Kind::Body => "SHINYPLASTIC",
    };
    if let Some(image) = material.image.and_then(|index| glb.images.get(index)) {
        let name = format!("{folder}_{}", image.name.to_ascii_uppercase());
        let texture = palette.add_png(&name, &image.png, max_texture)?;
        return Ok(Look { texture, light_material });
    }
    let texture = palette.add_colour(flat_colour(part.kind, material));
    Ok(Look { texture, light_material })
}

/// The colour of a surface with no picture: the material's own colour when it has one, else a grey for the part.
fn flat_colour(kind: Kind, material: &Material) -> [u8; 3] {
    let [r, g, b, _] = material.color;
    if [r, g, b] != [1.0; 3] {
        return [r, g, b].map(srgb_byte);
    }
    match kind {
        Kind::Tyre => [14, 14, 14],
        Kind::Rim => [150, 150, 155],
        Kind::Brake => [64, 64, 68],
        Kind::Metal => [118, 118, 124],
        Kind::Body => [96, 96, 102],
    }
}

fn srgb_byte(linear: f32) -> u8 {
    let value = linear.clamp(0.0, 1.0);
    let encoded = if value <= 0.003_130_8 { value * 12.92 } else { 1.055 * value.powf(1.0 / 2.4) - 0.055 };
    (encoded * 255.0 + 0.5) as u8
}

/// The index of `hash` in `list`, adding it when it is new.
fn slot_of(list: &mut Vec<u32>, hash: u32) -> u8 {
    let at = list.iter().position(|&known| known == hash).unwrap_or_else(|| {
        list.push(hash);
        list.len() - 1
    });
    at as u8
}

/// The car's bounds from the points of its finest body: car space (x forward, y left, z up) to the bounds space of
/// vehicles (x right, y up, z forward).
fn bounds(points: &[[f32; 3]], folder: &str) -> Result<(Vec<u8>, [f32; 3])> {
    let physics: Vec<[f32; 3]> = points.iter().map(|p| [-p[1], p[2], p[0]]).collect();
    let options = FitOptions { surface: CARBODY_SURFACE, ..FitOptions::default() };
    let set = fit_body(vlt_hash(&folder.to_ascii_uppercase()), &physics, &options).context("fitting the bounds")?;
    set.check_vehicle_body(&BodyLimits::default()).context("the bounds do not make a vehicle body")?;
    let bytes = write_bounds_pack(std::slice::from_ref(&set)).context("writing BOUNDS.BIN")?;
    Ok((bytes, set.root().half_dimensions))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slots_are_reused() {
        let mut list = Vec::new();
        assert_eq!(slot_of(&mut list, 5), 0);
        assert_eq!(slot_of(&mut list, 9), 1);
        assert_eq!(slot_of(&mut list, 5), 0);
    }

    #[test]
    fn a_missing_level_falls_back_to_a_finer_one() {
        let mut files = HashMap::new();
        files.insert("a.glb".to_owned(), PathBuf::from("a0"));
        files.insert("a_lod1.glb".to_owned(), PathBuf::from("a1"));
        let exports = Exports { files };
        assert_eq!(exports.find("a", 3), Some(&PathBuf::from("a1")));
        assert_eq!(exports.find("A", 0), Some(&PathBuf::from("a0")));
        assert_eq!(exports.find("b", 2), None);
    }

    #[test]
    fn flat_colours_follow_the_part() {
        let white = Material { name: "m".into(), image: None, color: [1.0; 4] };
        assert_eq!(flat_colour(Kind::Tyre, &white), [14, 14, 14]);
        let red = Material { name: "m".into(), image: None, color: [1.0, 0.0, 0.0, 1.0] };
        assert_eq!(flat_colour(Kind::Body, &red), [255, 0, 0]);
    }

    #[test]
    fn the_hidden_wheel_is_one_triangle() {
        let solid = hidden_wheel("X_KIT00_FRONT_TIRE_A", "C_000000");
        assert_eq!(solid.groups.len(), 1);
        assert_eq!(solid.groups[0].indices, vec![0, 1, 2]);
    }
}
