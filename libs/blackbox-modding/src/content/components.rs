//! The vehicle components of a mod (ADR 0014, `docs/specs/vehicle-components/manifest.md`): the catalog files of
//! `components/` and the `[[slot]]` and `[parts]` tables of a car manifest.
//!
//! This is the loader of the first stage: it reads and checks the two shapes and hands the model types of
//! `blackbox-components` to the game, which binds them (`nfsmw_data::car::mods::components`). Whether a named solid exists, and
//! what a parameter key means, is for the game to decide, because only the game opens the car's files and knows its physics.

use std::collections::BTreeMap;

use blackbox_components::{Catalog, Slot};
use serde::Deserialize;

use super::ContentError;

/// The folder inside a mod that holds the component catalog files (`[[component]]` tables), read in path order.
pub const COMPONENTS_FOLDER: &str = "components";
/// Most parts one car binds.
pub const MAX_PARTS: usize = 512;
/// Most slots one car lists.
pub const MAX_SLOTS: usize = 32;
/// Longest solid or pack name, in bytes. The game appends `_A` to `_D` (the level of detail) to a solid name.
const MAX_SOLID_BYTES: usize = 56;

/// How the game poses a part: which frame its solid is modelled in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PartRole {
    /// The frame of the car's body solid.
    #[default]
    Body,
    /// The frame of a bike's bars (the front hub on the origin, turning with the handlebar).
    Steered,
    /// The frame of a bike's fork (the front hub on the origin, turning and travelling with the front wheel).
    SteeredSlid,
    /// The frame of the front wheel (the hub on the origin, turning with the wheel).
    WheelFront,
    /// The frame of the rear wheel.
    WheelRear,
}

/// `[parts."<id>"]` as serde reads it.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct RawPart {
    solid: String,
    #[serde(default)]
    pack: Option<String>,
    #[serde(default)]
    role: PartRole,
    #[serde(default)]
    matrix: Option<Vec<f32>>,
}

/// Where the solid of one part is and how it sits on the car.
#[derive(Debug, Clone, PartialEq)]
pub struct PartBinding {
    /// The solid's name without its level-of-detail letter.
    pub solid: String,
    /// The shared folder under `CARS/` the solid is in; `None`: the car's own `GEOMETRY.BIN`.
    pub pack: Option<String>,
    pub role: PartRole,
    /// The mount: three rows of a 3 by 4 affine matrix (`p' = M [p, 1]`) taking the solid's points to the frame of `role`.
    /// `None`: the solid is modelled in that frame already.
    pub matrix: Option<[f32; 12]>,
}

/// The slots of a car and the parts it binds.
#[derive(Debug, Clone, PartialEq)]
pub struct Components {
    pub slots: Vec<Slot>,
    pub parts: BTreeMap<String, PartBinding>,
}

/// A solid or pack name: letters, digits, `_` and `-`.
fn is_solid_name(text: &str) -> bool {
    !text.is_empty()
        && text.len() <= MAX_SOLID_BYTES
        && text.bytes().all(|byte| byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-')
}

/// A part, slot or component id: lower-case letters, digits and `_ . : -`, 1 to 64 bytes.
pub fn is_component_id(text: &str) -> bool {
    !text.is_empty()
        && text.len() <= 64
        && text.bytes().all(|byte| matches!(byte, b'a'..=b'z' | b'0'..=b'9' | b'_' | b'.' | b':' | b'-'))
}

fn bad(field: &'static str, reason: &'static str) -> ContentError {
    ContentError::Field { field, reason }
}

fn part(raw: RawPart) -> Result<PartBinding, ContentError> {
    if !is_solid_name(&raw.solid) {
        return Err(ContentError::InvalidModel(raw.solid));
    }
    if let Some(pack) = &raw.pack
        && !is_solid_name(pack)
    {
        return Err(ContentError::InvalidModel(pack.clone()));
    }
    let matrix = match raw.matrix {
        None => None,
        Some(list) => {
            let rows: [f32; 12] = list.try_into().map_err(|_| bad("parts.matrix", "twelve numbers"))?;
            if rows.iter().any(|n| !n.is_finite()) {
                return Err(bad("parts.matrix", "finite numbers"));
            }
            Some(rows)
        }
    };
    Ok(PartBinding { solid: raw.solid, pack: raw.pack, role: raw.role, matrix })
}

/// Check the `[[slot]]` and `[parts]` tables of a car. `None` when the car gives neither. Which components the slots accept,
/// and whether the rules hold, is checked against the catalog by `VehicleDef::check` when the game publishes the car.
pub(super) fn check(
    slots: Option<Vec<Slot>>,
    parts: Option<BTreeMap<String, RawPart>>,
) -> Result<Option<Components>, ContentError> {
    let slots = slots.unwrap_or_default();
    let raw = parts.unwrap_or_default();
    if slots.is_empty() && raw.is_empty() {
        return Ok(None);
    }
    if slots.len() > MAX_SLOTS {
        return Err(bad("slot", "at most 32 slots"));
    }
    if raw.len() > MAX_PARTS {
        return Err(bad("parts", "at most 512 parts"));
    }
    if let Some(slot) = slots.iter().find(|slot| !is_component_id(&slot.id)) {
        return Err(ContentError::InvalidText(slot.id.clone()));
    }
    let mut parts = BTreeMap::new();
    for (id, table) in raw {
        if !is_component_id(&id) {
            return Err(ContentError::InvalidText(id));
        }
        parts.insert(id, part(table)?);
    }
    Ok(Some(Components { slots, parts }))
}

/// Parse one catalog file of `components/`: `[[component]]` tables. A component with an id the syntax does not allow, or one
/// defined twice in the file, refuses the file.
pub fn parse_catalog(text: &str) -> Result<Catalog, ContentError> {
    let catalog: Catalog = toml::from_str(text).map_err(|error| ContentError::Parse(error.to_string()))?;
    match catalog.iter().find(|component| !is_component_id(&component.id)) {
        Some(component) => Err(ContentError::InvalidText(component.id.clone())),
        None => Ok(catalog),
    }
}

/// Add the components of `more` to `catalog`; an id that is already there refuses the file (a mod cannot replace a component).
pub fn merge_catalog(catalog: &mut Catalog, more: Catalog) -> Result<(), ContentError> {
    for component in more.iter().cloned() {
        catalog.insert(component).map_err(ContentError::Parse)?;
    }
    Ok(())
}

#[cfg(test)]
#[path = "components_tests.rs"]
mod tests;
