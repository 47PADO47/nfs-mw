//! Reading the resources: types by name, structures by the import table, fields by offset.

use std::collections::HashMap;

use chameleon_bundle::{Bundle, Resource, ResourceKind};

use crate::{Clouds, Key, Timeline};

/// Why the sky could not be read.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("the bundle: {0}")]
    Bundle(#[from] chameleon_bundle::Error),
    #[error("a texture: {0}")]
    Gfx(#[from] chameleon_gfx::Error),
    #[error("the bundle has no type named {0}")]
    MissingType(&'static str),
    #[error("{0} is shorter than the field read from it")]
    Short(&'static str),
    #[error("the bundle has no moon material")]
    NoMoon,
}

/// A result of this crate.
pub type Result<T> = std::result::Result<T, Error>;

const TIMELINE_TYPE: &str = "Genesys.Gen.EnvironmentTimeline";
const CLOUDS_TYPE: &str = "Genesys.Gen.EnvironmentKeyframe.Clouds";
/// A timeline's keyframe records: the offset of the array and their count are at `+0x10` and `+0x14` of the object, each record
/// is 0x1C bytes with a handle to the keyframe at `+0x0C` and the hour (a float) at `+0x18`.
const RECORD: usize = 0x1C;
const HANDLE: usize = 0x0C;
const HOUR: usize = 0x18;

fn u32_at(block: &[u8], at: usize, what: &'static str) -> Result<u32> {
    let bytes = block.get(at..at + 4).ok_or(Error::Short(what))?;
    Ok(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
}

fn f32_at(block: &[u8], at: usize, what: &'static str) -> Result<f32> {
    Ok(f32::from_bits(u32_at(block, at, what)?))
}

fn floats<const N: usize>(block: &[u8], at: usize, what: &'static str) -> Result<[f32; N]> {
    let mut out = [0.0; N];
    for (index, slot) in out.iter_mut().enumerate() {
        *slot = f32_at(block, at + 4 * index, what)?;
    }
    Ok(out)
}

/// The name of a `GenesysType` resource: the zero-terminated string at the offset in `+0x20`, plus 8.
fn type_name(block: &[u8]) -> Option<String> {
    let start = u32_at(block, 0x20, "a type").ok()? as usize + 8;
    let tail = block.get(start..)?;
    let end = tail.iter().position(|&byte| byte == 0)?;
    String::from_utf8(tail[..end].to_vec()).ok()
}

/// The ids of the types of the bundle by their names.
fn types(bundle: &Bundle<'_>) -> Result<HashMap<u64, String>> {
    let mut out = HashMap::new();
    for (index, entry) in bundle.entries().iter().enumerate() {
        if entry.kind != ResourceKind::GenesysType.id() {
            continue;
        }
        let resource = bundle.resource(index)?;
        if let Some(name) = type_name(&resource.blocks[0]) {
            out.insert(entry.id, name);
        }
    }
    Ok(out)
}

/// The offset in `object` of the structure whose type is named `name`.
fn structure(object: &Resource, names: &HashMap<u64, String>, name: &str) -> Option<usize> {
    let import = object.imports.iter().find(|import| names.get(&import.id).is_some_and(|found| found == name))?;
    Some(import.slot() as usize)
}

fn clouds(object: &Resource, names: &HashMap<u64, String>) -> Result<Clouds> {
    let at = structure(object, names, CLOUDS_TYPE).ok_or(Error::MissingType(CLOUDS_TYPE))?;
    let block = &object.blocks[0];
    Ok(Clouds {
        texture: u32_at(block, at + 0x30, "a clouds structure")?,
        lit: floats(block, at + 0x10, "a clouds structure")?,
        ambient: floats(block, at + 0x20, "a clouds structure")?,
        params: floats(block, at + 0x38, "a clouds structure")?,
    })
}

pub fn timelines(bundle: &Bundle<'_>) -> Result<Vec<Timeline>> {
    let names = types(bundle)?;
    let mut out = Vec::new();
    let mut keyframes: HashMap<u64, Clouds> = HashMap::new();
    for (index, entry) in bundle.entries().iter().enumerate() {
        if entry.kind != ResourceKind::GenesysObject.id() {
            continue;
        }
        let object = bundle.resource(index)?;
        if structure(&object, &names, TIMELINE_TYPE) != Some(0) {
            continue;
        }
        let block = &object.blocks[0];
        let (array, count) = (u32_at(block, 0x10, "a timeline")? as usize, u32_at(block, 0x14, "a timeline")? as usize);
        let mut keys = Vec::with_capacity(count);
        for record in 0..count {
            let base = array + record * RECORD;
            let slot = (base + HANDLE) as u32;
            let Some(import) = object.imports.iter().find(|import| import.slot() == slot) else { continue };
            let hour = f32_at(block, base + HOUR, "a timeline")?;
            let clouds = match keyframes.get(&import.id) {
                Some(clouds) => *clouds,
                None => {
                    let keyframe = bundle.resource_by_id(import.id)?;
                    let clouds = clouds(&keyframe, &names)?;
                    keyframes.insert(import.id, clouds);
                    clouds
                }
            };
            keys.push(Key { hour, keyframe: import.id as u32, clouds });
        }
        out.push(Timeline { id: entry.id as u32, keys });
    }
    Ok(out)
}

pub fn texture(bundle: &Bundle<'_>, id: u64) -> Result<chameleon_gfx::Texture> {
    let resource = bundle.resource_by_id(id)?;
    Ok(chameleon_gfx::Texture::parse(&resource.blocks[0], &resource.blocks[1])?)
}

/// The moon material is the bundle's one `Material`; its imports are its shader, its texture and a sampler state: the texture
/// is the import of kind texture.
pub fn moon_texture(bundle: &Bundle<'_>) -> Result<u32> {
    for (index, entry) in bundle.entries().iter().enumerate() {
        if entry.kind != ResourceKind::Material.id() {
            continue;
        }
        let material = bundle.resource(index)?;
        for import in &material.imports {
            let Some(target) = bundle.find(import.id) else { continue };
            if bundle.entries()[target].kind == ResourceKind::Texture.id() {
                return Ok(import.id as u32);
            }
        }
    }
    Err(Error::NoMoon)
}
