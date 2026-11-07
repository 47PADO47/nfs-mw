//! Making the variant's files out of the 2012 trees: `GEOMETRY.BIN` with one solid per species and level of detail
//! (`MW12_<SPECIES>_L<level>`), `TEXTURES.BIN` with the textures they use, and the `profile.toml` that says which solid of the
//! 2005 world each replaces (`docs/modding/effect-variants.md`, "Profile format of the trees").
//!
//! A solid is in 2005 space (x forward, y left, z up) with its origin at the foot of the trunk, the way the game's own tree solids
//! stand. Its leaf cards are effect 6 (the game's tree shader) with the punch-through textures of a cut-out; its trunk is effect 0
//! (the world shader). The game fits the solid into the bounds of the one it replaces (`modding/content/model_swap`).

use std::collections::HashMap;

use anyhow::{Context, Result, bail};
use blackbox_hash::bstring_hash;
use blackbox_solid::{GroupSource, SolidSource, Vertex, write_geometry_pack};
use blackbox_tpk::write_tpk;

use super::light;
use super::read::{Level, Role, TreeModel};
use super::species::{self, Species};
use super::stock;
use crate::fit::to_car_space;
use crate::textures::Palette;

/// The effect and group flags of the game's own leaf cards (effect 6, the tree shader) and trunks (effect 0, the world shader).
const FOLIAGE: (u32, u32) = (6, 0x4008);
const BARK: (u32, u32) = (0, 0x4000);
/// Vertices one group may hold: the writer puts the groups of one effect in a single 16-bit-indexed buffer.
const MAX_GROUP_VERTICES: usize = 60_000;
/// The folder of the variant under the mod's `files/`, and the files in it.
pub const FOLDER: &str = "TREES/MW2012";
pub const GEOMETRY_FILE: &str = "GEOMETRY.BIN";
pub const TEXTURES_FILE: &str = "TEXTURES.BIN";

/// What the variant's files hold.
pub struct Pack {
    pub geometry: Vec<u8>,
    pub textures: Vec<u8>,
    pub profile: String,
    /// The species that made it into the pack, and how many levels of detail each has.
    pub species: Vec<(&'static str, usize)>,
}

/// The name of the pack's texture made from the 2012 texture resource `id`.
pub fn texture_name(id: u64) -> String {
    format!("MW12_T{:06X}", id & 0xFF_FFFF)
}

/// The pack of `models`. A species that cannot be carried over (a texture the 2005 engine cannot draw, a level with too many
/// vertices) is left out with a warning; fails when no species is left.
pub fn build(models: &[(&'static Species, TreeModel)], max_texture_size: u32) -> Result<Pack> {
    let mut palette = Palette::default();
    let mut solids = Vec::new();
    let mut built: Vec<(&'static str, usize)> = Vec::new();
    for (species, model) in models {
        match species_solids(species, model, &mut palette, max_texture_size) {
            Ok(list) if !list.is_empty() => {
                built.push((species.key, list.len()));
                solids.extend(list);
            }
            Ok(_) => log::warn!("trees: the {} ({}) has no mesh that can be carried over", species.key, species.label),
            Err(error) => log::warn!("trees: the {} ({}) is left out: {error:#}", species.key, species.label),
        }
    }
    if solids.is_empty() {
        bail!("no tree could be carried over");
    }
    let geometry = write_geometry_pack(&solids).context("writing the tree models")?;
    let textures = write_tpk("TREES", &palette.into_textures()).context("writing the tree textures")?;
    let profile = profile_text(&built);
    Ok(Pack { geometry, textures, profile, species: built })
}

/// The solids of one species, one per level of detail, and its textures added to `palette`. The levels stop at the first one
/// with nothing to draw.
fn species_solids(
    species: &Species,
    model: &TreeModel,
    palette: &mut Palette,
    max_texture_size: u32,
) -> Result<Vec<SolidSource>> {
    let mut roles: HashMap<u64, Role> = HashMap::new();
    for part in model.levels.iter().flat_map(|level| &level.parts) {
        roles.entry(part.texture).or_insert(part.role);
    }
    let mut ids: Vec<u64> = roles.keys().copied().collect();
    ids.sort_unstable();
    for id in ids {
        let texture = model.textures.get(&id).with_context(|| format!("the texture {id:#x} was not read"))?;
        let name = texture_name(id);
        let added = match roles[&id] {
            Role::Foliage => palette.add_foliage(&name, texture, max_texture_size),
            Role::Bark => palette.add_bark(&name, texture, max_texture_size),
        };
        added.with_context(|| format!("the texture {id:#x} is in a format the 2005 engine cannot draw"))?;
    }
    let mut solids = Vec::new();
    for (at, level) in model.levels.iter().enumerate() {
        let Some(solid) = level_solid(species::solid_name(species.key, at), level)? else { break };
        solids.push(solid);
    }
    Ok(solids)
}

/// One level of a tree as a solid: the bark groups, then the leaf groups, in 2005 space. `None` when the level has no triangle.
fn level_solid(name: String, level: &Level) -> Result<Option<SolidSource>> {
    let leaves: Vec<[f32; 3]> = level
        .parts
        .iter()
        .filter(|part| part.role == Role::Foliage)
        .flat_map(|part| part.positions.iter().map(|&p| to_car_space(p)))
        .collect();
    let crown = light::centre(&leaves);
    let (low, high) = leaves.iter().fold((f32::MAX, f32::MIN), |(low, high), p| (low.min(p[2]), high.max(p[2])));
    let mut hashes: Vec<u32> = Vec::new();
    let mut groups = Vec::new();
    for role in [Role::Bark, Role::Foliage] {
        for part in level.parts.iter().filter(|part| part.role == role && !part.triangles.is_empty()) {
            if part.positions.len() > MAX_GROUP_VERTICES {
                bail!("a mesh of {} vertices is too big for a solid", part.positions.len());
            }
            let positions: Vec<[f32; 3]> = part.positions.iter().map(|&p| to_car_space(p)).collect();
            let hash = bstring_hash(&texture_name(part.texture));
            let slot = hashes.iter().position(|&known| known == hash).unwrap_or_else(|| {
                hashes.push(hash);
                hashes.len() - 1
            });
            let normals = match role {
                Role::Foliage => light::crown_normals(&positions, crown),
                Role::Bark => light::smooth_normals(&positions, &part.triangles),
            };
            let vertices: Vec<Vertex> = positions
                .iter()
                .zip(&normals)
                .zip(&part.uvs)
                .map(|((&position, &normal), &uv)| Vertex {
                    position,
                    normal,
                    color_bgra: match role {
                        Role::Foliage => light::crown_colour(position[2], low, high),
                        Role::Bark => light::BARK,
                    },
                    uv,
                })
                .collect();
            let indices: Vec<u16> = part.triangles.iter().flatten().map(|&index| index as u16).collect();
            let (effect_id, flags) = match role {
                Role::Foliage => FOLIAGE,
                Role::Bark => BARK,
            };
            groups.push(GroupSource {
                texture_slots: [slot as u8; 5],
                effect_id,
                flags,
                ..GroupSource::new(vertices, indices)
            });
        }
    }
    if groups.is_empty() {
        return Ok(None);
    }
    let mut solid = SolidSource::new(name, groups);
    solid.flags = 0;
    solid.texture_hashes = hashes;
    Ok(Some(solid))
}

/// The text of the variant's `profile.toml`: a swap for every solid of [`stock::FAMILIES`] whose species is in `built` (the
/// species and the number of levels each has).
pub fn profile_text(built: &[(&str, usize)]) -> String {
    let mut text =
        format!("format = 1\ngeometry = \"{FOLDER}/{GEOMETRY_FILE}\"\ntextures = \"{FOLDER}/{TEXTURES_FILE}\"\n");
    for family in stock::FAMILIES {
        let Some(&(_, levels)) = built.iter().find(|(key, _)| *key == family.species) else { continue };
        for (solid, code) in family.names() {
            let model = species::solid_name(family.species, stock::level(code, family.row, levels));
            let fit = match family.row {
                true => "row",
                false => "height",
            };
            text.push_str(&format!("\n[[swap]]\nsolid = \"{solid}\"\nmodel = \"{model}\"\nfit = \"{fit}\"\n"));
        }
    }
    text
}
