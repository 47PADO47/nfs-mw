//! The `[customization]` table of a car manifest: everything the auto shop offers for a car that a mod adds.
//!
//! A car's looks are its own data, not another car's. The table lists the body paints (with the finish each is drawn
//! in), the rim paints, the window tints, and the parts the car carries in its own files: rims, spoilers, roof scoops,
//! vinyls and liveries. Parts are named by the models and textures of the car's own folder (`files/CARS/<ID>/`), so a mod car has
//! no stock car behind it. This module checks the schema only; whether a named model or texture exists is for the game
//! to decide, because only the game opens the car's files.

use serde::Deserialize;

use super::ContentError;
use super::plate::{self, Plate};

/// Most body paints, window tints, rim paints, parts and vinyls a car lists (per kind).
pub const MAX_PER_KIND: usize = 64;
/// Most vinyl colour choices.
pub const MAX_VINYL_COLORS: usize = 64;
/// Longest display name, in characters.
const MAX_NAME_CHARS: usize = 32;
/// Longest model or texture name, in bytes. The game appends `_A` to `_E` (the level of detail) to a model name.
const MAX_ASSET_BYTES: usize = 56;

/// How a paint is drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Finish {
    /// Plain gloss.
    Gloss,
    /// A metallic flake.
    Metallic,
    /// Mirror chrome.
    Chrome,
}

impl Finish {
    pub fn parse(text: &str) -> Option<Self> {
        match text {
            "gloss" => Some(Self::Gloss),
            "metallic" => Some(Self::Metallic),
            "chrome" => Some(Self::Chrome),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Gloss => "gloss",
            Self::Metallic => "metallic",
            Self::Chrome => "chrome",
        }
    }
}

/// A body paint.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Paint {
    pub name: String,
    /// sRGB, 0 to 255.
    pub color: [u8; 3],
    pub finish: Finish,
}

/// A rim paint.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RimPaint {
    pub name: String,
    pub color: [u8; 3],
}

/// A window tint: the colour the glass takes and how much of the light it stops.
#[derive(Debug, Clone, PartialEq)]
pub struct Tint {
    pub name: String,
    pub color: [u8; 3],
    /// 0 (clear) to 1 (opaque): the alpha the glass is drawn with.
    pub opacity: f32,
}

/// A model the car carries: rims, a spoiler or a roof scoop. `model` is the solid's name without the level-of-detail
/// letter; the game looks for `<model>_A` to `<model>_E` in the car's own `GEOMETRY.BIN`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Part {
    pub name: String,
    pub model: String,
}

/// A vinyl: a picture drawn over the body paint, in the car's own texture layout.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Vinyl {
    pub name: String,
    /// The picture's texture in the car's own `VINYLS.BIN`. Its red, green and blue weights say how much of each
    /// colour channel shows where.
    pub bitmap: String,
    /// The mask texture in the same file: its red channel is the opacity.
    pub mask: String,
    /// How many colour channels the bitmap weighs: 0 (the bitmap's own colours) to 3.
    pub channels: u8,
}

/// A livery: a whole-body picture in the layout of the car's paint surfaces, drawn over the body paint. Where the picture
/// is opaque it replaces the paint; where it is transparent the paint colour shows (the 2012 game's rule). Unlike a
/// vinyl it has no mask of its own and no recolourable channels: the picture carries its colours and its alpha.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Livery {
    pub name: String,
    /// The picture's texture in the car's own `TEXTURES.BIN`.
    pub texture: String,
}

/// A colour a vinyl channel can take.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VinylColor {
    pub name: String,
    pub color: [u8; 3],
}

/// A checked `[customization]` table.
#[derive(Debug, Clone, PartialEq)]
pub struct Customization {
    /// The body paints, at least one.
    pub paints: Vec<Paint>,
    /// The position in `paints` of the paint the car has until the player changes it.
    pub default_paint: usize,
    pub rim_paints: Vec<RimPaint>,
    pub tints: Vec<Tint>,
    pub rims: Vec<Part>,
    pub spoilers: Vec<Part>,
    pub roof_scoops: Vec<Part>,
    pub vinyls: Vec<Vinyl>,
    /// The colours the vinyl channels can take. Empty keeps the game's built-in palette.
    pub vinyl_colors: Vec<VinylColor>,
    /// The liveries, offered with the vinyls.
    pub liveries: Vec<Livery>,
    /// The position in `liveries` of the livery the car wears until the player changes it (a police car's markings); `None`:
    /// the plain paint.
    pub default_livery: Option<usize>,
    /// The licence plate the car can change the text of, `[customization.plate]`. `None`: the car has no such plate.
    pub plate: Option<Plate>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Raw {
    #[serde(default)]
    paints: Vec<RawPaint>,
    #[serde(default)]
    default_paint: Option<usize>,
    #[serde(default)]
    rim_paints: Vec<RawColor>,
    #[serde(default)]
    tints: Vec<RawTint>,
    #[serde(default)]
    rims: Vec<RawPart>,
    #[serde(default)]
    spoilers: Vec<RawPart>,
    #[serde(default)]
    roof_scoops: Vec<RawPart>,
    #[serde(default)]
    vinyls: Vec<RawVinyl>,
    #[serde(default)]
    vinyl_colors: Vec<RawColor>,
    #[serde(default)]
    liveries: Vec<RawLivery>,
    #[serde(default)]
    default_livery: Option<usize>,
    #[serde(default)]
    plate: Option<plate::Raw>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawPaint {
    name: String,
    color: [i64; 3],
    #[serde(default)]
    finish: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawColor {
    name: String,
    color: [i64; 3],
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawTint {
    name: String,
    color: [i64; 3],
    opacity: f32,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawPart {
    name: String,
    model: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawVinyl {
    name: String,
    bitmap: String,
    mask: String,
    #[serde(default)]
    channels: Option<i64>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawLivery {
    name: String,
    texture: String,
}

fn field(field: &'static str, reason: &'static str) -> ContentError {
    ContentError::Field { field, reason }
}

/// Check the table. A table is present only for a car that carries its own customization: it needs at least one paint.
pub(super) fn check(raw: Option<Raw>) -> Result<Option<Customization>, ContentError> {
    let Some(raw) = raw else { return Ok(None) };
    if raw.paints.is_empty() {
        return Err(field("customization.paints", "a car needs at least one body paint"));
    }
    let too_many = |len: usize| len > MAX_PER_KIND;
    if [raw.paints.len(), raw.rim_paints.len(), raw.tints.len(), raw.rims.len()].into_iter().any(too_many)
        || [raw.spoilers.len(), raw.roof_scoops.len(), raw.vinyls.len(), raw.liveries.len()].into_iter().any(too_many)
        || raw.vinyl_colors.len() > MAX_VINYL_COLORS
    {
        return Err(field("customization", "a list has more than 64 entries"));
    }
    let default_paint = raw.default_paint.unwrap_or(0);
    if default_paint >= raw.paints.len() {
        return Err(field("customization.default_paint", "the default paint is a position in paints"));
    }
    if raw.default_livery.is_some_and(|position| position >= raw.liveries.len()) {
        return Err(field("customization.default_livery", "the default livery is a position in liveries"));
    }
    let paints = raw
        .paints
        .into_iter()
        .map(|paint| {
            let finish = match paint.finish.as_deref() {
                None => Finish::Gloss,
                Some(text) => Finish::parse(text)
                    .ok_or(field("customization.paints.finish", "a finish is gloss, metallic or chrome"))?,
            };
            Ok(Paint { name: name(paint.name)?, color: color(paint.color)?, finish })
        })
        .collect::<Result<Vec<_>, ContentError>>()?;
    let rim_paints = raw
        .rim_paints
        .into_iter()
        .map(|rim| Ok(RimPaint { name: name(rim.name)?, color: color(rim.color)? }))
        .collect::<Result<Vec<_>, ContentError>>()?;
    let tints = raw
        .tints
        .into_iter()
        .map(|tint| {
            if !tint.opacity.is_finite() || !(0.0..=1.0).contains(&tint.opacity) {
                return Err(field("customization.tints.opacity", "opacity is a number from 0 to 1"));
            }
            Ok(Tint { name: name(tint.name)?, color: color(tint.color)?, opacity: tint.opacity })
        })
        .collect::<Result<Vec<_>, ContentError>>()?;
    let vinyl_colors = raw
        .vinyl_colors
        .into_iter()
        .map(|entry| Ok(VinylColor { name: name(entry.name)?, color: color(entry.color)? }))
        .collect::<Result<Vec<_>, ContentError>>()?;
    let vinyls = raw
        .vinyls
        .into_iter()
        .map(|vinyl| {
            let channels = vinyl.channels.unwrap_or(0);
            if !(0..=3).contains(&channels) {
                return Err(field("customization.vinyls.channels", "channels is 0 to 3"));
            }
            Ok(Vinyl {
                name: name(vinyl.name)?,
                bitmap: asset(vinyl.bitmap)?,
                mask: asset(vinyl.mask)?,
                channels: channels as u8,
            })
        })
        .collect::<Result<Vec<_>, ContentError>>()?;
    let liveries = raw
        .liveries
        .into_iter()
        .map(|livery| Ok(Livery { name: name(livery.name)?, texture: asset(livery.texture)? }))
        .collect::<Result<Vec<_>, ContentError>>()?;
    let result = Customization {
        paints,
        default_paint,
        rim_paints,
        tints,
        rims: parts(raw.rims)?,
        spoilers: parts(raw.spoilers)?,
        roof_scoops: parts(raw.roof_scoops)?,
        vinyls,
        vinyl_colors,
        liveries,
        default_livery: raw.default_livery,
        plate: plate::check(raw.plate)?,
    };
    no_repeats(&result)?;
    Ok(Some(result))
}

fn parts(raw: Vec<RawPart>) -> Result<Vec<Part>, ContentError> {
    raw.into_iter().map(|part| Ok(Part { name: name(part.name)?, model: asset(part.model)? })).collect()
}

/// A display name: 1 to 32 characters, no control character, trimmed.
fn name(text: String) -> Result<String, ContentError> {
    let trimmed = text.trim();
    if trimmed.is_empty() || trimmed.chars().count() > MAX_NAME_CHARS || trimmed.chars().any(char::is_control) {
        return Err(ContentError::InvalidText(text));
    }
    Ok(trimmed.to_owned())
}

/// A model or texture name of the car's own files: letters, digits, `_` and `-`, 1 to 56 bytes.
fn asset(text: String) -> Result<String, ContentError> {
    let valid = !text.is_empty()
        && text.len() <= MAX_ASSET_BYTES
        && text.bytes().all(|byte| byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-');
    if !valid {
        return Err(ContentError::InvalidModel(text));
    }
    Ok(text)
}

fn color(rgb: [i64; 3]) -> Result<[u8; 3], ContentError> {
    if rgb.iter().any(|channel| !(0..=255).contains(channel)) {
        return Err(field("customization.color", "a colour is three numbers from 0 to 255"));
    }
    Ok(rgb.map(|channel| channel as u8))
}

/// Two entries of one list must not share a name: the game tells choices apart by name, and a saved car remembers the
/// choice by it.
fn no_repeats(table: &Customization) -> Result<(), ContentError> {
    let vinyl_names: Vec<&str> =
        table.vinyls.iter().map(|e| e.name.as_str()).chain(table.liveries.iter().map(|e| e.name.as_str())).collect();
    let lists: [(&'static str, Vec<&str>); 10] = [
        ("customization.paints", table.paints.iter().map(|e| e.name.as_str()).collect()),
        ("customization.rim_paints", table.rim_paints.iter().map(|e| e.name.as_str()).collect()),
        ("customization.tints", table.tints.iter().map(|e| e.name.as_str()).collect()),
        ("customization.rims", table.rims.iter().map(|e| e.name.as_str()).collect()),
        ("customization.spoilers", table.spoilers.iter().map(|e| e.name.as_str()).collect()),
        ("customization.roof_scoops", table.roof_scoops.iter().map(|e| e.name.as_str()).collect()),
        ("customization.vinyls", table.vinyls.iter().map(|e| e.name.as_str()).collect()),
        ("customization.vinyl_colors", table.vinyl_colors.iter().map(|e| e.name.as_str()).collect()),
        ("customization.liveries", table.liveries.iter().map(|e| e.name.as_str()).collect()),
        // The shop lists vinyls and liveries in one category, so a livery may not take a vinyl's name.
        ("customization.liveries and vinyls", vinyl_names),
    ];
    for (field_name, names) in lists {
        let mut seen: Vec<&str> = Vec::with_capacity(names.len());
        for entry in names {
            if seen.contains(&entry) {
                return Err(field(field_name, "two entries have the same name"));
            }
            seen.push(entry);
        }
    }
    Ok(())
}
