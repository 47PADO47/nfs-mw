//! The skid marks of the 2012 game as a variant of the 2005 game's (`SKIDMARKS/MW2012/` in the mod's `files/`, the value **NFS Most
//! Wanted 2012** of the gameplay setting "Skid marks variant"; evidence in `docs/formats/chameleon-tyre-marks.md`).
//!
//! In the 2012 game every road surface (`HAWAII/SURFACELIST.BNDL`, the `SurfaceType` objects) names two `SurfaceVfx` objects,
//! one in each of two weather slots, and a `SurfaceVfx` object keeps the materials of the marks a tyre leaves
//! (`TyreMarksMaterials`: five references into `GLOBALEFFECTS.BNDL`). A mark material runs the `Vfx_TyreMarksNew` shader, whose
//! samplers are three pairs of pictures, a diffuse and a normal map for a tyre that slides sideways (slip), one that spins (spin)
//! and one that rolls (run). This reads the SURFACELIST and the few materials and textures it names **by ranges** (the install's
//! files are never read whole), picks the set of the paved road in each weather slot and the set of the loose road, and writes
//! the slip and spin diffuse pictures of each as PNGs with a profile that the game's skid mark variant draws.

use std::collections::HashMap;
use std::path::Path;

use anyhow::{Context, Result, bail};
use chameleon_bundle::{Bundle, Ranges, ResourceKind};
use chameleon_gfx::{Material, Shader, Texture};

use crate::source::Source;
use crate::trees::ranges;
use crate::vfx::{self, GLOBAL_EFFECTS, Picture};

/// The id of the variant in the `skid_marks_variant` option, and what the menu shows for it.
pub const VARIANT: &str = "mw2012";
pub const LABEL: &str = "NFS Most Wanted 2012";
/// The folder of the variant under the mod's `files/`.
pub const FOLDER: &str = "SKIDMARKS/MW2012";

const SURFACES: &str = "HAWAII/SURFACELIST.BNDL";
const SURFACE_TYPE: &str = "Genesys.Gen.SurfaceType";
const SURFACE_VFX: &str = "Genesys.Gen.SurfaceVfx";
const TYPE_FLAG: u32 = 0x8000_0000;
/// Where a `SurfaceType` object keeps its two `SurfaceVfx` references: the first of every road surface that has marks is the
/// look the dry road gets and the second the wet road's. **[infer]**: the second set's run normal map is a streaked water
/// pattern and the two are equal on the loose roads, which do not change with the rain.
const DRY_VFX: u32 = 0xE0;
const WET_VFX: u32 = 0x1B0;
/// Where a `SurfaceVfx` object keeps the first of the five mark materials of its `TyreMarksMaterials`.
const FIRST_MARK_MATERIAL: u32 = 0x18;
/// The shader's samplers of the two pictures the game's variant draws.
const SLIP_SAMPLER: &str = "SlipDiffuseSampler";
const SPIN_SAMPLER: &str = "SpinDiffuseSampler";
/// A mark picture is loose-road dirt when its red is this much over its blue (the loose roads' marks are brown, the paved
/// roads' black).
const BROWN: f32 = 25.0;
/// Metres of road one repeat of a picture covers. **Chosen, not decoded**: the 32 and 64 pixel pictures are tread prints, and a
/// tyre's circumference is about two metres; the sideways slide's streaks run longer.
const SLIP_TILE: f32 = 4.0;
const SPIN_TILE: f32 = 2.0;

/// The two pictures of one set of marks.
#[derive(Debug, Clone, PartialEq)]
pub struct MarkPictures {
    pub slip: Picture,
    pub spin: Picture,
}

/// What the install holds: the set of the dry paved road, of the wet paved road and of the loose road.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct MarkSets {
    pub paved: Option<MarkPictures>,
    pub paved_wet: Option<MarkPictures>,
    pub loose: Option<MarkPictures>,
}

/// How much of the install was read, for the log.
#[derive(Debug, Clone, Copy, Default)]
pub struct Cost {
    pub read: u64,
    pub total: u64,
}

/// The surfaces' mark materials, tallied: the first material of each set, and how many road surfaces name it in each slot.
#[derive(Debug, Default)]
struct Tally {
    dry: HashMap<u64, usize>,
    wet: HashMap<u64, usize>,
}

/// The ids of the Genesys type resources of the bundle named `name`.
fn type_ids(bundle: &Bundle, name: &str) -> Vec<u64> {
    let kind = ResourceKind::GenesysType.id();
    let mut ids = Vec::new();
    for (index, entry) in bundle.entries().iter().enumerate() {
        if entry.kind != kind {
            continue;
        }
        let Ok(resource) = bundle.resource(index) else { continue };
        if vfx::type_name(&resource.blocks[0]).as_deref() == Some(name) {
            ids.push(entry.id);
        }
    }
    ids
}

/// The road surfaces' marks: for each `SurfaceType` object, the first mark material of the `SurfaceVfx` it names in each slot.
fn tally(surfaces: &Bundle) -> Tally {
    let (types, vfxs) = (type_ids(surfaces, SURFACE_TYPE), type_ids(surfaces, SURFACE_VFX));
    let object = ResourceKind::GenesysObject.id();
    let mut materials: HashMap<u64, u64> = HashMap::new();
    let mut roads: Vec<(Option<u64>, Option<u64>)> = Vec::new();
    for (index, entry) in surfaces.entries().iter().enumerate() {
        if entry.kind != object || entry.import_count == 0 {
            continue;
        }
        let Ok(resource) = surfaces.resource(index) else { continue };
        let at = |slot: u32| {
            resource.imports.iter().find(|import| import.offset == TYPE_FLAG | slot).map(|import| import.id)
        };
        if resource.imports.iter().any(|import| import.offset == TYPE_FLAG && vfxs.contains(&import.id))
            && let Some(material) = at(FIRST_MARK_MATERIAL)
        {
            materials.insert(entry.id, material);
        }
        if resource.imports.iter().any(|import| import.offset == TYPE_FLAG && types.contains(&import.id)) {
            roads.push((at(DRY_VFX), at(WET_VFX)));
        }
    }
    let mut tally = Tally::default();
    for (dry, wet) in roads {
        if let Some(material) = dry.and_then(|vfx| materials.get(&vfx)) {
            *tally.dry.entry(*material).or_default() += 1;
        }
        if let Some(material) = wet.and_then(|vfx| materials.get(&vfx)) {
            *tally.wet.entry(*material).or_default() += 1;
        }
    }
    tally
}

/// The two diffuse pictures of a mark material, by the names of the shader's samplers.
fn pictures(effects: &Bundle, shaders: &Bundle, material: u64) -> Result<MarkPictures> {
    let at = effects.find(material).with_context(|| format!("no material {material:#x}"))?;
    let resource = effects.resource(at)?;
    let parsed = Material::parse(&resource.blocks[0], &resource.imports).map_err(|error| anyhow::anyhow!("{error}"))?;
    let shader_at = shaders.find(parsed.shader).context("the mark material's shader is not in SHADERS.BNDL")?;
    let shader = shaders.resource(shader_at)?;
    let shader =
        Shader::parse(&shader.blocks[0], &shader.imports, |_| false).map_err(|error| anyhow::anyhow!("{error}"))?;
    let picture = |sampler: &str| -> Result<Picture> {
        let slot = shader.sampler_slot(sampler).with_context(|| format!("the shader has no {sampler}"))?;
        let id = *parsed.textures.get(slot).with_context(|| format!("the material has no texture for {sampler}"))?;
        let texture = effects.resource(effects.find(id).with_context(|| format!("no texture {id:#x}"))?)?;
        let texture =
            Texture::parse(&texture.blocks[0], &texture.blocks[1]).map_err(|error| anyhow::anyhow!("{error}"))?;
        vfx::decode(&texture)
    };
    Ok(MarkPictures { slip: picture(SLIP_SAMPLER)?, spin: picture(SPIN_SAMPLER)? })
}

/// Whether a set's marks are brown (loose road): the mean colour of the slip picture's visible pixels.
fn is_brown(pictures: &MarkPictures) -> bool {
    let (mut sums, mut weight) = ([0.0f32; 3], 0.0f32);
    for pixel in pictures.slip.rgba.as_chunks::<4>().0 {
        let alpha = f32::from(pixel[3]) / 255.0;
        weight += alpha;
        for (sum, channel) in sums.iter_mut().zip(pixel) {
            *sum += f32::from(*channel) * alpha;
        }
    }
    weight > 0.0 && (sums[0] - sums[2]) / weight > BROWN
}

/// The most named material of `counts` that `wanted` accepts (the lowest id when two are named as often).
fn most_named(counts: &HashMap<u64, usize>, wanted: impl Fn(u64) -> bool) -> Option<u64> {
    counts
        .iter()
        .filter(|(id, _)| wanted(**id))
        .min_by_key(|(id, count)| (std::cmp::Reverse(**count), **id))
        .map(|(id, _)| *id)
}

/// The sets of the install, read by ranges. `Cost` is what that took.
pub fn find(source: &Source) -> Result<(MarkSets, Cost)> {
    let (surfaces, surfaces_reader) = ranges::open(&source.root().join(SURFACES))?;
    let (effects, effects_reader) = ranges::open(&source.root().join(GLOBAL_EFFECTS))?;
    let shaders = Bundle::parse(source.shared_bundles().1).context("reading SHADERS.BNDL")?;
    let tally = tally(&surfaces);
    if tally.dry.is_empty() {
        bail!("no road surface of {SURFACES} names marks");
    }
    let mut found: HashMap<u64, MarkPictures> = HashMap::new();
    for material in tally.dry.keys().chain(tally.wet.keys()) {
        match pictures(&effects, &shaders, *material) {
            Ok(set) => {
                found.insert(*material, set);
            }
            Err(error) => log::warn!("skid marks: the mark material {material:#x} is not read: {error:#}"),
        }
    }
    let brown = |material: u64| found.get(&material).is_some_and(is_brown);
    let readable = |material: u64| found.contains_key(&material);
    let paved_dry = most_named(&tally.dry, |id| readable(id) && !brown(id));
    let paved_wet = most_named(&tally.wet, |id| readable(id) && !brown(id)).filter(|id| Some(*id) != paved_dry);
    let loose_id = most_named(&tally.dry, |id| readable(id) && brown(id));
    let paved = paved_dry.and_then(|id| found.get(&id).cloned());
    // The wet set differs from the dry one in its normal maps and constants only (the 2012 diffuse pictures are the same):
    // the profile has a wet set only where the pictures it would draw differ.
    let paved_wet = paved_wet.and_then(|id| found.get(&id).cloned()).filter(|wet| Some(wet) != paved.as_ref());
    let sets = MarkSets { paved, paved_wet, loose: loose_id.and_then(|id| found.get(&id).cloned()) };
    let read = surfaces_reader.bytes_read() + effects_reader.bytes_read();
    Ok((sets, Cost { read, total: Ranges::size(&*surfaces_reader) + Ranges::size(&*effects_reader) }))
}

/// The text of the profile for the sets that exist.
pub fn profile_text(sets: &MarkSets) -> String {
    let mut text = String::from("format = 1\n");
    let mut add = |name: &str, surface: &str, wet: bool, found: &Option<MarkPictures>| {
        if found.is_none() {
            return;
        }
        text.push_str(&format!(
            "\n[[set]]\nsurface = \"{surface}\"\nwet = {wet}\nslip = \"{FOLDER}/{name}_SLIP.PNG\"\nspin = \"{FOLDER}/{name}_SPIN.PNG\"\n\
             slip_tile = {SLIP_TILE:?}\nspin_tile = {SPIN_TILE:?}\nwidth = 1.0\nopacity = 1.0\nlife = 45.0\n"
        ));
    };
    add("PAVED", "paved", false, &sets.paved);
    add("PAVED_WET", "paved", true, &sets.paved_wet);
    add("LOOSE", "loose", false, &sets.loose);
    text
}

/// Read the install's marks and write the variant under `out/files/SKIDMARKS/MW2012/`. A failure only costs the variant: the
/// game's own marks stay the only choice.
pub fn write(source: &Source, out: &Path) {
    let started = std::time::Instant::now();
    let (sets, cost) = match find(source) {
        Ok(found) => found,
        Err(error) => {
            log::warn!("the skid marks variant is not written: {error:#}");
            return;
        }
    };
    if sets.paved.is_none() && sets.loose.is_none() {
        log::warn!("the 2012 install has no skid mark pictures that can be read: the skid marks variant has none");
        return;
    }
    match write_files(out, &sets) {
        Ok(bytes) => log::info!(
            "skid marks variant: read {} KiB of {} KiB of the bundles, wrote {bytes} bytes, {} ms",
            cost.read / 1024,
            cost.total / 1024,
            started.elapsed().as_millis()
        ),
        Err(error) => log::warn!("the skid marks variant is not written: {error:#}"),
    }
}

fn write_files(out: &Path, sets: &MarkSets) -> Result<usize> {
    let folder = out.join("files").join(FOLDER);
    std::fs::create_dir_all(&folder).with_context(|| format!("creating {}", folder.display()))?;
    let mut written = 0;
    for (name, found) in [("PAVED", &sets.paved), ("PAVED_WET", &sets.paved_wet), ("LOOSE", &sets.loose)] {
        let Some(found) = found else { continue };
        for (kind, picture) in [("SLIP", &found.slip), ("SPIN", &found.spin)] {
            let bytes = vfx::png_bytes(picture)?;
            written += bytes.len();
            std::fs::write(folder.join(format!("{name}_{kind}.PNG")), bytes)
                .with_context(|| format!("writing {name}_{kind}.PNG"))?;
        }
    }
    let profile = profile_text(sets);
    written += profile.len();
    std::fs::write(folder.join("profile.toml"), profile).context("writing the skid marks profile")?;
    Ok(written)
}

#[cfg(test)]
#[path = "marks_assets_tests.rs"]
mod tests;
