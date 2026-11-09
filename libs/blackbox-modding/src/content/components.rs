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
/// Most markers one part carries.
pub const MAX_PART_MARKERS: usize = 8;
/// Longest marker name, in bytes.
const MAX_MARKER_NAME_BYTES: usize = 32;
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
    #[serde(default)]
    base: bool,
    #[serde(default)]
    markers: Vec<RawMarker>,
}

/// One entry of a part's `markers` list as serde reads it.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct RawMarker {
    name: String,
    position: [f32; 3],
    direction: [f32; 3],
}

/// A named point a part carries (a tail pipe: where its flame starts and the way it shoots).
#[derive(Debug, Clone, PartialEq)]
pub struct PartMarker {
    /// The marker's name, upper case, digits and `_` (`RIGHT_EXHAUST`): which of them the game reads is for the game to say.
    pub name: String,
    /// Metres, in the frame of the part's `role` (for a body part, the car's frame: x forward, y left, z up), **after** the part's
    /// `matrix` like the points of its solid: the marker is written where the pipe's outlet is on this car.
    pub position: [f32; 3],
    /// The way the flame shoots, a unit vector in the same frame.
    pub direction: [f32; 3],
}

/// Where the solid of one part is and how it sits on the car.
#[derive(Debug, Clone, PartialEq)]
pub struct PartBinding {
    /// The solid's name without its level-of-detail letter.
    pub solid: String,
    /// The folder under the mod's `files/` that holds the part's own `GEOMETRY.BIN` and `TEXTURES.BIN` (`PARTS/R6/MUF974_001`),
    /// shared by every car that binds the part; `None`: the car's own `GEOMETRY.BIN`.
    pub pack: Option<String>,
    pub role: PartRole,
    /// The mount: three rows of a 3 by 4 affine matrix (`p' = M [p, 1]`) taking the solid's points to the frame of `role`.
    /// `None`: the solid is modelled in that frame already.
    pub matrix: Option<[f32; 12]>,
    /// The car is drawn with this part whatever its slots hold (`VehicleDef::parts`): the extra solids of a body whose mesh does not
    /// fit one solid. A part of a component (`show`) leaves this `false`.
    pub base: bool,
    /// The points the part carries (`markers = [{ name, position, direction }]`): an exhaust part says where its flame starts, so
    /// the flames follow the part that is fitted. Empty: the part has none.
    pub markers: Vec<PartMarker>,
}

/// The slots of a car and the parts it binds.
#[derive(Debug, Clone, PartialEq)]
pub struct Components {
    pub slots: Vec<Slot>,
    pub parts: BTreeMap<String, PartBinding>,
}

/// A pack folder: names of letters, digits, `_` and `-` separated by `/`, 1 to 64 bytes in all.
fn is_pack_path(text: &str) -> bool {
    text.len() <= 64 && text.split('/').all(is_solid_name)
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
        && !is_pack_path(pack)
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
    let markers = markers(raw.markers)?;
    Ok(PartBinding { solid: raw.solid, pack: raw.pack, role: raw.role, matrix, base: raw.base, markers })
}

/// The markers of a part: a plain name, finite numbers, a direction that points somewhere (stored as a unit vector).
fn markers(raw: Vec<RawMarker>) -> Result<Vec<PartMarker>, ContentError> {
    if raw.len() > MAX_PART_MARKERS {
        return Err(bad("parts.markers", "at most 8 markers"));
    }
    let mut out = Vec::with_capacity(raw.len());
    for marker in raw {
        let plain = |byte: u8| byte.is_ascii_uppercase() || byte.is_ascii_digit() || byte == b'_';
        if marker.name.is_empty() || marker.name.len() > MAX_MARKER_NAME_BYTES || !marker.name.bytes().all(plain) {
            return Err(bad("parts.markers", "a name of capitals, digits and _"));
        }
        if marker.position.iter().chain(&marker.direction).any(|n| !n.is_finite()) {
            return Err(bad("parts.markers", "finite numbers"));
        }
        let [x, y, z] = marker.direction;
        let length = (x * x + y * y + z * z).sqrt();
        if length < 1e-6 {
            return Err(bad("parts.markers", "a direction that is not zero"));
        }
        let direction = [x / length, y / length, z / length];
        out.push(PartMarker { name: marker.name, position: marker.position, direction });
    }
    Ok(out)
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
    if let Some(component) = catalog.iter().find(|component| !is_component_id(&component.id)) {
        return Err(ContentError::InvalidText(component.id.clone()));
    }
    Ok(catalog)
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
