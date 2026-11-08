//! Making the files of the two sets out of the 2012 models: a `GEOMETRY.BIN` with one solid for every solid of the 2005 world that is
//! replaced (each authored in the frame of the solid it takes the place of, so the host uses it as it is, `fit = "exact"`), a
//! `TEXTURES.BIN` with the pictures they use, and the manifest of the set (`docs/modding/api-replace.md`).

use std::collections::HashMap;
use std::fmt::Write as _;

use anyhow::{Context, Result, bail};
use blackbox_hash::bstring_hash;
use blackbox_solid::{GroupSource, PositionMarker, SolidSource, Vertex, write_geometry_pack};
use blackbox_tpk::write_tpk;

use super::geometry::{self, Mesh, Picture};
use super::table::{self, ARM_SWAPS, HEAD_SWAPS, LIGHT_ATLAS, SIGN_SWAPS};
use crate::textures::Palette;
use crate::trees::light;
use crate::trees::read::{Part, Role, TreeModel};

/// The effect and flags of the 2005 world's own sign groups (effect 0, the world shader) and the grey its vertices carry.
const WORLD: (u32, u32) = (0, 0x4000);
const GREY: [u8; 4] = [66, 66, 66, 255];
/// The most a sign's plate may be wider than the one it replaces (the signs of the 2012 game are narrower for their height).
const PLATE_SHARE: f32 = 1.15;
/// The spread the manifests declare (the host's default; the converter squeezes models to fit within it).
pub const SPREAD: f32 = 1.25;
/// Where the back of a hung signal head is along x, and the hanger's colour and half width.
const HEAD_BACK: [f32; 2] = [-0.12, -0.10];
const HANGER: [f32; 3] = [0.02, 0.02, 0.02];
const HANGER_HALF: f32 = 0.04;
/// How far above the head's origin (where the 2005 world hangs it from the arm) the hanger reaches: the 2012 arm is a little higher than the
/// 2005 one at that place, and the old head's box allows 0.3 m of it.
const HANGER_TOP: f32 = 0.25;
/// The marker int parameter of a red blinking light (`docs/specs/environment-prop-lights.md`).
const RED_BLINK: i32 = 4;
/// Vertices one group may hold: the writer puts the groups of one effect in a single 16-bit-indexed buffer.
const MAX_GROUP_VERTICES: usize = 60_000;

/// What a set's files hold.
pub struct Pack {
    pub geometry: Vec<u8>,
    pub textures: Vec<u8>,
    pub manifest: String,
    /// How many solids of the 2005 world the set replaces.
    pub swaps: usize,
}

/// The name of the texture made from the 2012 texture resource `id`.
pub fn texture_name(id: u64) -> String {
    format!("MW12_S{:06X}", id & 0xFF_FFFF)
}

/// A 2012 model as meshes: the level of detail `level` (the last one when the model has fewer), every mesh drawn.
pub fn meshes(model: &TreeModel, level: usize) -> Vec<Mesh> {
    let Some(level) = model.levels.get(level).or_else(|| model.levels.last()) else { return Vec::new() };
    level.parts.iter().filter(|part| !part.triangles.is_empty()).map(mesh_of).collect()
}

fn mesh_of(part: &Part) -> Mesh {
    Mesh {
        picture: Picture::Atlas(part.texture),
        cut_out: part.role == Role::Foliage,
        positions: part.positions.clone(),
        uvs: part.uvs.clone(),
        triangles: part.triangles.clone(),
    }
}

/// A solid of `name` made of `meshes` (in the 2005 solid's own axes), with the textures they use added to `palette`.
fn solid_of(
    name: &str,
    meshes: &[Mesh],
    markers: Vec<PositionMarker>,
    models: &[&TreeModel],
    palette: &mut Palette,
    max_texture_size: u32,
) -> Result<SolidSource> {
    let mut hashes: Vec<u32> = Vec::new();
    let mut groups = Vec::new();
    for mesh in meshes.iter().filter(|mesh| !mesh.triangles.is_empty()) {
        if mesh.positions.len() > MAX_GROUP_VERTICES {
            bail!("a mesh of {} vertices is too big for a solid", mesh.positions.len());
        }
        let texture = match mesh.picture {
            Picture::Colour(rgb) => palette.add_colour(rgb),
            Picture::Atlas(id) => {
                let picture = models
                    .iter()
                    .find_map(|model| model.textures.get(&id))
                    .with_context(|| format!("the texture {id:#x} was not read"))?;
                let name = texture_name(id);
                let added = match mesh.cut_out {
                    true => palette.add_foliage(&name, picture, max_texture_size),
                    false => palette.add_bark(&name, picture, max_texture_size),
                };
                added.with_context(|| format!("the texture {id:#x} is in a format the 2005 engine cannot draw"))?
            }
        };
        let hash = bstring_hash(&texture);
        let slot = hashes.iter().position(|&known| known == hash).unwrap_or_else(|| {
            hashes.push(hash);
            hashes.len() - 1
        });
        let normals = light::smooth_normals(&mesh.positions, &mesh.triangles);
        let vertices: Vec<Vertex> = mesh
            .positions
            .iter()
            .zip(&normals)
            .zip(&mesh.uvs)
            .map(|((&position, &normal), &uv)| Vertex { position, normal, color_bgra: GREY, uv })
            .collect();
        let indices: Vec<u16> = mesh.triangles.iter().flatten().map(|&index| index as u16).collect();
        groups.push(GroupSource {
            texture_slots: [slot as u8; 5],
            effect_id: WORLD.0,
            flags: WORLD.1,
            ..GroupSource::new(vertices, indices)
        });
    }
    if groups.is_empty() {
        bail!("{name} has nothing to draw");
    }
    let mut solid = SolidSource::new(name, groups);
    solid.flags = 0;
    solid.texture_hashes = hashes;
    solid.markers = markers;
    Ok(solid)
}

/// The marker of a red blinking light at `at`.
fn red_marker(at: [f32; 3]) -> PositionMarker {
    let mut matrix = [0.0; 16];
    matrix[0] = 1.0;
    matrix[5] = 1.0;
    matrix[10] = 1.0;
    matrix[12..15].copy_from_slice(&at);
    matrix[15] = 1.0;
    PositionMarker { name_hash: bstring_hash("SIGNAL_RED"), int_param: RED_BLINK, float_params: [0.0; 2], matrix }
}

/// The solid names of the packs: `MW12_<kind letter><number>`, short because a solid's name is a fixed field of the format.
fn solid_name(letter: char, number: usize) -> String {
    format!("MW12_{letter}{number:02}")
}

/// The road signs pack: for every swap of [`SIGN_SWAPS`] whose sign was read, the sign turned to face -y and fitted into the box of the
/// 2005 solid it replaces. `models` are the 2012 signs by their key of [`table::SIGNS`].
pub fn signs(models: &HashMap<&'static str, TreeModel>, max_texture_size: u32) -> Result<Pack> {
    let all: Vec<&TreeModel> = models.values().collect();
    let mut palette = Palette::default();
    let mut solids = Vec::new();
    let mut entries: Vec<(&str, String)> = Vec::new();
    for swap in &SIGN_SWAPS {
        let Some(sign) = table::SIGNS.iter().find(|sign| sign.key == swap.sign) else { continue };
        let Some(model) = models.get(sign.key) else { continue };
        let turned = geometry::face_minus_y(&geometry::to_old_axes(&meshes(model, 0)), sign.front);
        let fitted = geometry::squeeze_into(&geometry::stand_in(&turned, &swap.old, PLATE_SHARE), &swap.old, SPREAD);
        let name = solid_name('S', entries.len());
        match solid_of(&name, &fitted, Vec::new(), &all, &mut palette, max_texture_size) {
            Ok(solid) => {
                solids.push(solid);
                entries.push((swap.target, name));
            }
            Err(error) => log::warn!("signs: {} ({}) is left out: {error:#}", swap.target, sign.label),
        }
    }
    finish(solids, palette, entries, table::SIGNS_KIND, table::SIGNS_KIND_LABEL, table::SIGNS_FOLDER)
}

/// The traffic lights pack: the 2012 mast arm (its heads cut away) in the box of each pole of the 2005 world, and the 2012 signal head
/// hung as each of the 2005 heads is. `arm` and `head` are the 2012 models of [`table::ARM`] and [`table::HEAD`].
pub fn lights(arm: &TreeModel, head: &TreeModel, max_texture_size: u32) -> Result<Pack> {
    let all = [arm, head];
    let mut palette = Palette::default();
    let mut solids = Vec::new();
    let mut entries: Vec<(&str, String)> = Vec::new();
    for swap in &ARM_SWAPS {
        let pieces = meshes(arm, swap.level);
        let cut: Vec<Mesh> = pieces
            .iter()
            .map(|mesh| {
                let cut = geometry::cut_heads(mesh);
                log::debug!("signs: the arm of {} loses {} pieces of signal equipment", swap.target, cut.taken.len());
                cut.arm
            })
            .collect();
        let fitted =
            geometry::squeeze_into(&geometry::fill(&geometry::to_old_axes(&cut), &swap.old), &swap.old, SPREAD);
        let name = solid_name('A', entries.len());
        match solid_of(&name, &fitted, Vec::new(), &all, &mut palette, max_texture_size) {
            Ok(solid) => {
                solids.push(solid);
                entries.push((swap.target, name));
            }
            Err(error) => log::warn!("signs: {} is left out: {error:#}", swap.target),
        }
    }
    for (number, swap) in HEAD_SWAPS.iter().enumerate() {
        let Some(built) = signal_head(head, swap, HEAD_BACK[number / 3]) else { continue };
        let name = solid_name('H', entries.len());
        match solid_of(&name, &built.0, vec![red_marker(built.1)], &all, &mut palette, max_texture_size) {
            Ok(solid) => {
                solids.push(solid);
                entries.push((swap.target, name));
            }
            Err(error) => log::warn!("signs: {} is left out: {error:#}", swap.target),
        }
    }
    finish(solids, palette, entries, table::LIGHTS_KIND, table::LIGHTS_KIND_LABEL, table::LIGHTS_FOLDER)
}

/// One signal head, hung and fitted into the box of `swap`, with a hanger up to the arm, and where its red lens is.
fn signal_head(head: &TreeModel, swap: &table::HeadSwap, back: f32) -> Option<(Vec<Mesh>, [f32; 3])> {
    let own = |level: usize| -> Vec<Mesh> {
        meshes(head, level).into_iter().filter(|mesh| mesh.picture == Picture::Atlas(atlas_of(head))).collect()
    };
    let parts = Some(own(swap.level)).filter(|parts| !parts.is_empty()).unwrap_or_else(|| own(0));
    let (mut hung, red) = geometry::hang(&parts, swap.hanging, back)?;
    let top = geometry::bounds(&hung)?.high[2];
    let middle = 0.5 * (geometry::bounds(&hung)?.low[0] + geometry::bounds(&hung)?.high[0]);
    let mut rod = geometry::rod(HANGER_TOP, top, HANGER_HALF, HANGER);
    rod.positions.iter_mut().for_each(|p| p[0] += middle);
    hung.push(rod);
    // The marker rides along as a one-point mesh so that the squeeze moves it like the rest.
    hung.push(Mesh {
        picture: Picture::Colour(HANGER),
        cut_out: false,
        positions: vec![red],
        uvs: vec![[0.0; 2]],
        triangles: Vec::new(),
    });
    let mut fitted = geometry::squeeze_into(&hung, &swap.old, SPREAD);
    let point = fitted.pop()?.positions[0];
    Some((fitted, point))
}

/// The resource id of the picture the heads of the 2012 model use (the atlas of [`LIGHT_ATLAS`]).
fn atlas_of(model: &TreeModel) -> u64 {
    model.textures.keys().copied().find(|id| id & 0xFF_FFFF == LIGHT_ATLAS).unwrap_or(0)
}

fn finish(
    solids: Vec<SolidSource>,
    palette: Palette,
    entries: Vec<(&str, String)>,
    kind: &str,
    label: &str,
    folder: &str,
) -> Result<Pack> {
    if solids.is_empty() {
        bail!("nothing could be carried over");
    }
    let geometry = write_geometry_pack(&solids).context("writing the models")?;
    let textures = write_tpk("SIGNS", &palette.into_textures()).context("writing the textures")?;
    let manifest = manifest_text(&entries, kind, label, folder);
    Ok(Pack { geometry, textures, manifest, swaps: entries.len() })
}

/// The text of a set's manifest (`replace/<kind>-mw2012.toml`): every model is authored in the frame of the solid it replaces, so the
/// fit is `exact`.
pub fn manifest_text(entries: &[(&str, String)], kind: &str, kind_label: &str, folder: &str) -> String {
    let mut text = String::new();
    let _ = write!(
        text,
        "format = 1

[set]
kind = \"{kind}\"
kind_label = \"{kind_label}\"
value = \"{value}\"
value_label = \"{label}\"
fit = \"exact\"

[files]
geometry = \"{folder}/{geometry}\"
textures = \"{folder}/{textures}\"
",
        value = table::VALUE,
        label = table::VALUE_LABEL,
        geometry = table::GEOMETRY_FILE,
        textures = table::TEXTURES_FILE,
    );
    for (target, model) in entries {
        let _ = write!(text, "\n[[model]]\ntarget = \"{target}\"\nmodel = \"{model}\"\n");
    }
    text
}
