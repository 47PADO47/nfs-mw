//! The time-of-day colour cubes of `HAWAII/ENVIRONMENT.BNDL`. Format and evidence: `docs/formats/chameleon-environment-grading.md`.
//!
//! The bundle holds `Genesys.Gen.EnvironmentTimeline` objects: a list of keys, each an hour of the day and an
//! `EnvironmentKeyframe` (lighting, fog, sky, clouds ... and one colour cube). A keyframe also names a second keyframe, the
//! one the game blends to when the weather is wet (its cube is the wet look of the same hour). So a timeline is, for the
//! grading, a list of `(hour, dry cube, wet cube)`. Nothing here opens a file: it takes a parsed bundle, and finds its objects by the type names the bundle's
//! `GenesysType` resources carry (the field names are hashed, the type names are not).

use std::collections::HashMap;

use chameleon_bundle::{Bundle, ResourceKind};

/// The type names of the objects read here.
const TIMELINE_TYPE: &str = "Genesys.Gen.EnvironmentTimeline";
const KEYFRAME_TYPE: &str = "Genesys.Gen.EnvironmentKeyframe";

/// Where a timeline object keeps its list: the offset of the keys at `+0x10`, their count at `+0x14` **[verified]**; each key
/// is 28 bytes, its keyframe is the import at `+0xC` and its hour a float at `+0x18` **[verified]** (the unit, hours of the day
/// from 0 to 23, is a reading).
mod at {
    pub const KEYS: usize = 0x10;
    pub const COUNT: usize = 0x14;
    /// In a keyframe: the import of its wet keyframe (the same keyframe type).
    pub const WET_KEYFRAME: usize = 0xC;
    pub const STRIDE: usize = 28;
    pub const KEYFRAME: usize = 0xC;
    pub const HOUR: usize = 0x18;
}

/// One key of a timeline: from this hour on, the keyframe's cube.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TimelineKey {
    pub hour: f32,
    /// The resource id of the colour cube (`0x52`) the key's keyframe names.
    pub cube: u64,
    /// The cube of the keyframe's wet keyframe, when it has one that names a cube.
    pub wet_cube: Option<u64>,
}

/// An `EnvironmentTimeline`: its keys in file order (the hours rise).
#[derive(Debug, Clone, PartialEq)]
pub struct Timeline {
    /// The resource id of the object.
    pub id: u64,
    pub keys: Vec<TimelineKey>,
}

impl Timeline {
    /// The distinct dry cubes the keys name, in order of first use.
    pub fn cubes(&self) -> Vec<u64> {
        distinct(self.keys.iter().map(|key| key.cube))
    }

    /// The distinct wet cubes the keys name, in order of first use.
    pub fn wet_cubes(&self) -> Vec<u64> {
        distinct(self.keys.iter().filter_map(|key| key.wet_cube))
    }
}

fn distinct(ids: impl Iterator<Item = u64>) -> Vec<u64> {
    let mut found: Vec<u64> = Vec::new();
    for id in ids {
        if !found.contains(&id) {
            found.push(id);
        }
    }
    found
}

/// A colour cube as RGB bytes: `size`^3 entries, red varying fastest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CubeRgb {
    pub size: u32,
    pub rgb: Vec<u8>,
}

/// The id of the `GenesysType` named `name`: the type's block holds the name, ending in a zero **[verified]**.
fn type_id(bundle: &Bundle, name: &str) -> Option<u64> {
    let needle: Vec<u8> = name.bytes().chain(std::iter::once(0)).collect();
    let kind = ResourceKind::GenesysType.id();
    bundle.entries().iter().enumerate().filter(|(_, entry)| entry.kind == kind).find_map(|(index, entry)| {
        let resource = bundle.resource(index).ok()?;
        resource.blocks[0].windows(needle.len()).any(|window| window == needle.as_slice()).then_some(entry.id)
    })
}

fn float(block: &[u8], offset: usize) -> Option<f32> {
    let value = f32::from_le_bytes(block.get(offset..offset + 4)?.try_into().ok()?);
    value.is_finite().then_some(value)
}

fn word(block: &[u8], offset: usize) -> Option<usize> {
    Some(u32::from_le_bytes(block.get(offset..offset + 4)?.try_into().ok()?) as usize)
}

/// The colour cube a keyframe resource names: its import that is a colour cube.
fn cube_of(bundle: &Bundle, resource: &chameleon_bundle::Resource) -> Option<u64> {
    let cube = ResourceKind::ColourCube.id();
    resource
        .imports
        .iter()
        .find(|import| bundle.find(import.id).is_some_and(|at| bundle.entries()[at].kind == cube))
        .map(|import| import.id)
}

/// The dry cube of a keyframe and the cube of its wet keyframe (the keyframe object its import at `+0xC` names).
fn keyframe_cubes(bundle: &Bundle, keyframe: u64) -> Option<(u64, Option<u64>)> {
    let resource = bundle.resource(bundle.find(keyframe)?).ok()?;
    let dry = cube_of(bundle, &resource)?;
    let wet = resource
        .imports
        .iter()
        .find(|import| import.slot() as usize == at::WET_KEYFRAME)
        .and_then(|import| bundle.resource(bundle.find(import.id)?).ok())
        .and_then(|wet| cube_of(bundle, &wet));
    Some((dry, wet))
}

/// Every timeline of `bundle`, in file order. A timeline with a key that names no keyframe, or a keyframe with no cube, is left
/// out; so is any whose list does not fit its block.
pub fn timelines(bundle: &Bundle) -> Vec<Timeline> {
    let (Some(timeline), Some(_)) = (type_id(bundle, TIMELINE_TYPE), type_id(bundle, KEYFRAME_TYPE)) else {
        return Vec::new();
    };
    let object = ResourceKind::GenesysObject.id();
    let mut cubes: HashMap<u64, Option<(u64, Option<u64>)>> = HashMap::new();
    let mut found = Vec::new();
    for (index, entry) in bundle.entries().iter().enumerate() {
        if entry.kind != object || entry.import_count == 0 {
            continue;
        }
        let Ok(resource) = bundle.resource(index) else { continue };
        if resource.imports.first().map(|import| (import.id, import.slot())) != Some((timeline, 0)) {
            continue;
        }
        let block = &resource.blocks[0];
        let (Some(start), Some(count)) = (word(block, at::KEYS), word(block, at::COUNT)) else { continue };
        if count == 0 || count > 256 {
            continue;
        }
        let keys: Option<Vec<TimelineKey>> = (0..count)
            .map(|k| {
                let base = start + k * at::STRIDE;
                let keyframe = resource.imports.iter().find(|i| i.slot() as usize == base + at::KEYFRAME)?.id;
                let (cube, wet_cube) = (*cubes.entry(keyframe).or_insert_with(|| keyframe_cubes(bundle, keyframe)))?;
                Some(TimelineKey { hour: float(block, base + at::HOUR)?, cube, wet_cube })
            })
            .collect();
        if let Some(keys) = keys {
            found.push(Timeline { id: entry.id, keys });
        }
    }
    found
}

/// The cube resource `id` as RGB bytes. The block is a 16-byte header (the edge as a 32-bit number, twice) and then 3 bytes an
/// entry, blue first **[unconfirmed]** (see `docs/formats/chameleon-environment-grading.md`), the red input varying fastest, so
/// the entry bytes are reversed into red, green, blue. `None` when the block is not a cube.
pub fn cube_rgb(bundle: &Bundle, id: u64) -> Option<CubeRgb> {
    let resource = bundle.resource(bundle.find(id)?).ok()?;
    cube_from_block(&resource.blocks[0])
}

/// [`cube_rgb`] of a cube's data block.
pub fn cube_from_block(block: &[u8]) -> Option<CubeRgb> {
    let size = word(block, 0)? as u32;
    if !(2..=64).contains(&size) || word(block, 4)? as u32 != size || block.len() != 16 + (size as usize).pow(3) * 3 {
        return None;
    }
    let rgb = block[16..].chunks_exact(3).flat_map(|bgr| [bgr[2], bgr[1], bgr[0]]).collect();
    Some(CubeRgb { size, rgb })
}

#[cfg(test)]
#[path = "environment_tests.rs"]
mod tests;
