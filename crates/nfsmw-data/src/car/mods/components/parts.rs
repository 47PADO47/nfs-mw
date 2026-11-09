//! Drawing the parts of a resolved car on the assembled model: the solids its components show, in the frame their role names.
//!
//! The assembled car (`car::assemble`) draws the base parts the manifest's `[parts]` do not bind: body, wheels, fork and bars.
//! Everything a component shows, the stock exhaust as much as an aftermarket one, is a bound part, so it is drawn here, from
//! the car's own `GEOMETRY.BIN` or from the part's own pack (`pack`: a folder under `files/`, shared by every car that binds it).
//!
//! A solid is modelled in its own frame and sits on a car by the binding's `matrix`. The game poses a wheel, fork or bars
//! placement every frame from the car's corner rig and would drop a mount carried by the placement, so the matrix is applied to the
//! solid's vertices when it is added to the model (a copy per car, under a name of its own).

use std::collections::{BTreeMap, HashMap, HashSet};

use anyhow::{Context, Result};
use blackbox_hash::bstring_hash;
use blackbox_solid::Solid;
use blackbox_tpk::Texture;
use game_install::GameDir;
use glam::{Mat3, Mat4, Vec3, Vec4};

use super::{Binding, Components, Role, select};
use crate::car::customization::Selection;
use crate::car::mods::custom::apply::lists;
use crate::car::physics::PerformancePackage;
use crate::car::{CarModel, Corner, Placement, slot};

/// The letters of the levels of detail, `A` the finest.
const LEVELS: usize = 5;

/// Draw the parts the car's components show with `selection` installed, at level of detail `lod` (and every level the model
/// draws while driving). A part that cannot be drawn is left out and the log says why: a car is still a car.
pub fn apply(
    dir: &GameDir,
    folder: &str,
    components: &Components,
    selection: &Selection,
    lod: usize,
    model: &mut CarModel,
) {
    let Some(resolved) = select::resolve(folder, components, selection, PerformancePackage::default()) else { return };
    let wanted: Vec<(&String, &Binding)> =
        resolved.parts.iter().filter_map(|id| components.parts.get_key_value(id)).collect();
    if wanted.is_empty() {
        return;
    }
    if let Err(error) = place(dir, folder, &wanted, lod, model) {
        log::warn!("{folder}: its parts are not drawn: {error:#}");
    }
}

/// The solid names (without letters) each pack is asked for, by pack.
fn names_by_pack<'a>(wanted: &[(&'a String, &'a Binding)]) -> BTreeMap<Option<&'a str>, HashSet<u32>> {
    let mut out: BTreeMap<Option<&str>, HashSet<u32>> = BTreeMap::new();
    for &(_, binding) in wanted {
        let hashes = (0..LEVELS).map(|level| bstring_hash(&format!("{}_{}", binding.solid, letter(level))));
        out.entry(binding.pack.as_deref()).or_default().extend(hashes);
    }
    out
}

fn letter(level: usize) -> char {
    char::from(b'A' + level as u8)
}

fn place(dir: &GameDir, folder: &str, wanted: &[(&String, &Binding)], lod: usize, model: &mut CarModel) -> Result<()> {
    let mut packs: HashMap<Option<&str>, HashMap<u32, Solid>> = HashMap::new();
    for (pack, names) in names_by_pack(wanted) {
        let path = match pack {
            Some(pack) => format!("{pack}/GEOMETRY.BIN"),
            None => format!("CARS/{folder}/GEOMETRY.BIN"),
        };
        let bytes = crate::read_unwrapped(dir, &path).with_context(|| format!("reading {path}"))?;
        let solids = blackbox_solid::read_solids_filtered(&bytes, |hash| names.contains(&hash))
            .with_context(|| format!("parsing {path}"))?;
        packs.insert(pack, solids.into_iter().map(|solid| (solid.name_hash, solid)).collect());
    }
    let levels: Vec<usize> = lists(model, lod).iter().map(|(level, _)| *level).collect();
    let corners = model.corners;
    let mut added: Vec<u32> = Vec::new();
    for &(id, binding) in wanted {
        let pack = &packs[&binding.pack.as_deref()];
        let mut found: HashMap<usize, u32> = HashMap::new();
        for &level in &levels {
            if let Some(key) = add_solid(model, pack, id, binding, level) {
                found.insert(level, key);
                added.push(key);
            }
        }
        let Some(&canonical) = found.get(&lod) else {
            // A base part with no solid at this level is a car whose body has fewer solids there: nothing to draw.
            if binding.base {
                continue;
            }
            log::warn!("{folder}: the part {id} is not drawn: its solid {} is not in its files", binding.solid);
            continue;
        };
        for (level, placements) in lists(model, lod) {
            let Some(solid) = found.get(&level).copied().or_else(|| (!binding.base).then_some(canonical)) else {
                continue;
            };
            let Some(placement) = mounted_placement(binding.role, solid, corners.as_ref()) else { continue };
            placements.push(placement);
        }
    }
    load_textures(dir, folder, wanted, &added, model)
}

/// The solid of `binding` at `level` (or the nearest level its pack has), added to the model under the name of the part and
/// the level, with the binding's matrix applied. The key it was added under.
fn add_solid(
    model: &mut CarModel,
    pack: &HashMap<u32, Solid>,
    id: &str,
    binding: &Binding,
    level: usize,
) -> Option<u32> {
    let order = levels_to_try(binding.base, level);
    for at in order {
        let Some(solid) = pack.get(&bstring_hash(&format!("{}_{}", binding.solid, letter(at)))) else { continue };
        let key = bstring_hash(&format!("{id}@{}", letter(at)));
        model.solids.entry(key).or_insert_with(|| mounted(solid, binding.matrix, key));
        return Some(key);
    }
    None
}

/// The levels of detail whose solid stands in for `level`, in order: the part's own, then the nearest finer, then the nearer coarser ones.
/// A base part is the solid of its own level or nothing: the finer solids of a body are not drawn in place of a coarser one.
pub(super) fn levels_to_try(base: bool, level: usize) -> Vec<usize> {
    let order = (0..=level).rev().chain(level + 1..LEVELS);
    order.filter(|&at| !base || at == level).collect()
}

/// Where the game puts a part of `role`: the body's frame is the car's, the others are the corner rig's. `None` for a car without
/// a wheel setup (nothing to hang a wheel part on).
fn mounted_placement(role: Role, solid: u32, corners: Option<&[Corner; 4]>) -> Option<Placement> {
    let corner = |slot: usize, at: usize, brake: bool| {
        corners.map(|c| Placement {
            solid,
            transform: if brake { c[at].brake } else { c[at].wheel },
            left_brake: false,
            slot,
            corner: Some(at),
        })
    };
    match role {
        Role::Body => {
            Some(Placement { solid, transform: Mat4::IDENTITY, left_brake: false, slot: slot::SPOILER, corner: None })
        }
        Role::Steered => corner(slot::REAR_BRAKE, 1, true),
        Role::SteeredSlid => corner(slot::FRONT_BRAKE, 0, true),
        Role::WheelFront => corner(slot::FRONT_WHEEL, 0, false),
        Role::WheelRear => corner(slot::REAR_WHEEL, 2, false),
    }
}

/// The textures the added solids list and the model lacks, from the car's own pack and the packs of the parts.
fn load_textures(
    dir: &GameDir,
    folder: &str,
    wanted: &[(&String, &Binding)],
    added: &[u32],
    model: &mut CarModel,
) -> Result<()> {
    let hashes: HashSet<u32> = added
        .iter()
        .filter_map(|key| model.solids.get(key))
        .flat_map(|solid| solid.texture_hashes.iter().copied())
        .filter(|hash| !model.textures.contains_key(hash))
        .collect();
    if hashes.is_empty() {
        return Ok(());
    }
    let packs: HashSet<&str> = wanted.iter().filter_map(|(_, binding)| binding.pack.as_deref()).collect();
    let mut found: HashMap<u32, Texture> = crate::car::textures::load(dir, folder, &hashes)?;
    for pack in packs {
        let path = format!("{pack}/TEXTURES.BIN");
        if !dir.exists(&path) {
            continue;
        }
        let bytes = crate::read_unwrapped(dir, &path).with_context(|| format!("reading {path}"))?;
        let texture_packs = blackbox_tpk::read_texture_packs_filtered(&bytes, |hash| hashes.contains(&hash))
            .with_context(|| format!("parsing {path}"))?;
        for texture in texture_packs.into_iter().flat_map(|pack| pack.textures) {
            found.entry(texture.name_hash).or_insert(texture);
        }
    }
    for (hash, texture) in found {
        model.textures.entry(hash).or_insert(texture);
    }
    Ok(())
}

/// The three rows of a 3 by 4 affine matrix as a transform.
pub fn affine(rows: [f32; 12]) -> Mat4 {
    Mat4::from_cols(
        Vec4::new(rows[0], rows[4], rows[8], 0.0),
        Vec4::new(rows[1], rows[5], rows[9], 0.0),
        Vec4::new(rows[2], rows[6], rows[10], 0.0),
        Vec4::new(rows[3], rows[7], rows[11], 1.0),
    )
}

/// The box of `min` and `max` after `m`.
fn boxed(m: Mat4, min: [f32; 3], max: [f32; 3]) -> ([f32; 3], [f32; 3]) {
    let mut low = Vec3::splat(f32::MAX);
    let mut high = Vec3::splat(f32::MIN);
    for corner in 0..8u8 {
        let pick = |bit: u8, a: f32, b: f32| if corner & bit == 0 { a } else { b };
        let point = Vec3::new(pick(1, min[0], max[0]), pick(2, min[1], max[1]), pick(4, min[2], max[2]));
        let moved = m.transform_point3(point);
        low = low.min(moved);
        high = high.max(moved);
    }
    (low.to_array(), high.to_array())
}

/// The matrix that carries a normal of a surface moved by `m` (the inverse transpose of its linear part, so a part with a scale,
/// like a rim seated on its tyre, keeps its normals square to its surface); the linear part itself when it cannot be inverted.
fn normal_matrix(m: Mat4) -> Mat3 {
    let linear = Mat3::from_mat4(m);
    match linear.determinant().abs() < 1e-12 {
        true => linear,
        false => linear.inverse().transpose(),
    }
}

/// A copy of `solid` named `key` with `matrix` applied to its vertices, normals, tangents and boxes; without a matrix, the copy.
pub fn mounted(solid: &Solid, matrix: Option<[f32; 12]>, key: u32) -> Solid {
    let mut out = solid.clone();
    out.name_hash = key;
    let Some(rows) = matrix else { return out };
    let m = affine(rows);
    let normals = normal_matrix(m);
    for vertex in &mut out.vertices {
        vertex.position = m.transform_point3(Vec3::from(vertex.position)).to_array();
        vertex.normal = (normals * Vec3::from(vertex.normal)).normalize_or_zero().to_array();
    }
    for tangent in out.tangents.iter_mut().flatten() {
        let moved = m.transform_vector3(Vec3::new(tangent[0], tangent[1], tangent[2])).normalize_or_zero();
        *tangent = [moved.x, moved.y, moved.z, tangent[3]];
    }
    (out.bounds_min, out.bounds_max) = boxed(m, solid.bounds_min, solid.bounds_max);
    for group in &mut out.groups {
        (group.bounds_min, group.bounds_max) = boxed(m, group.bounds_min, group.bounds_max);
    }
    out
}
