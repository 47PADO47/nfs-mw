//! The `[customization.plate]` table of a car manifest: a licence plate whose text the car can change.
//!
//! A plate is a **glyph sheet**: a texture of the car's own `TEXTURES.BIN` that holds `columns` by `rows` equal cells, one
//! glyph per cell, listed row by row in `glyphs`. The car's body has a plate mesh of `slots` quads, side by side, and quad
//! `n` is mapped to cell `n` of the sheet, so the sheet as authored shows the first `slots` glyphs. To show a text the game
//! draws a copy of the sheet whose first `slots` cells hold the glyphs of the text, centred, and the blank glyph (the space
//! of `glyphs`) in the slots the text leaves empty. This module checks the schema only; whether the sheet exists is for the
//! game to decide.

use std::collections::BTreeMap;

use serde::Deserialize;

use super::ContentError;

/// Most cells a sheet has, and most slots a plate has.
pub const MAX_CELLS: usize = 256;
pub const MAX_SLOTS: usize = 16;
/// Longest texture name, in bytes (the same limit as the other pictures of a car).
const MAX_ASSET_BYTES: usize = 56;

/// A checked plate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Plate {
    /// The text the car wears until the player sets another, as typed (it is drawn as [`Plate::glyphs`] allow). `None`:
    /// the sheet as authored.
    pub text: Option<String>,
    /// The glyph sheet's texture in the car's own `TEXTURES.BIN`.
    pub picture: String,
    /// The glyph of each cell, row by row; always `columns * rows` of them, unique, and including the space (the blank).
    pub glyphs: Vec<char>,
    pub columns: u32,
    pub rows: u32,
    /// Characters the plate shows at once.
    pub slots: u32,
    /// Typed characters that are drawn as another glyph (`0` as `O`), as pairs of (typed, drawn).
    pub aliases: Vec<(char, char)>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Raw {
    #[serde(default)]
    text: Option<String>,
    picture: String,
    glyphs: String,
    columns: u32,
    rows: u32,
    slots: u32,
    #[serde(default)]
    aliases: BTreeMap<String, String>,
}

fn field(field: &'static str, reason: &'static str) -> ContentError {
    ContentError::Field { field, reason }
}

/// Check the table.
pub(super) fn check(raw: Option<Raw>) -> Result<Option<Plate>, ContentError> {
    let Some(raw) = raw else { return Ok(None) };
    let valid = !raw.picture.is_empty()
        && raw.picture.len() <= MAX_ASSET_BYTES
        && raw.picture.bytes().all(|byte| byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-');
    if !valid {
        return Err(ContentError::InvalidModel(raw.picture));
    }
    let glyphs: Vec<char> = raw.glyphs.chars().collect();
    let cells = raw.columns as usize * raw.rows as usize;
    if raw.columns == 0 || raw.rows == 0 || cells > MAX_CELLS || glyphs.len() != cells {
        return Err(field("customization.plate.glyphs", "one glyph for each of columns x rows cells (at most 256)"));
    }
    if glyphs.iter().any(|c| !c.is_ascii_graphic() && *c != ' ') {
        return Err(field("customization.plate.glyphs", "a glyph is a printable ASCII character"));
    }
    if (1..glyphs.len()).any(|at| glyphs[..at].contains(&glyphs[at])) {
        return Err(field("customization.plate.glyphs", "a glyph is listed twice"));
    }
    if !glyphs.contains(&' ') {
        return Err(field("customization.plate.glyphs", "the space is the blank glyph and must be one of the cells"));
    }
    if raw.slots == 0 || raw.slots as usize > MAX_SLOTS {
        return Err(field("customization.plate.slots", "a plate shows 1 to 16 characters"));
    }
    let aliases = raw
        .aliases
        .iter()
        .map(|(typed, drawn)| alias(typed, drawn, &glyphs))
        .collect::<Result<Vec<_>, ContentError>>()?;
    if let Some(text) = &raw.text {
        let long = text.chars().count() > raw.slots as usize;
        if long || text.chars().any(char::is_control) {
            return Err(field("customization.plate.text", "the text fits the slots and has no control character"));
        }
    }
    Ok(Some(Plate {
        text: raw.text,
        picture: raw.picture,
        glyphs,
        columns: raw.columns,
        rows: raw.rows,
        slots: raw.slots,
        aliases,
    }))
}

/// One alias: a typed character and the glyph of the sheet it is drawn as.
fn alias(typed: &str, drawn: &str, glyphs: &[char]) -> Result<(char, char), ContentError> {
    let mut typed = typed.chars();
    let mut drawn = drawn.chars();
    let (Some(typed), None, Some(drawn), None) = (typed.next(), typed.next(), drawn.next(), drawn.next()) else {
        return Err(field("customization.plate.aliases", "an alias is one character drawn as one character"));
    };
    if !glyphs.contains(&drawn) || glyphs.contains(&typed) {
        return Err(field(
            "customization.plate.aliases",
            "an alias is drawn as a glyph of the sheet and is not one itself",
        ));
    }
    Ok((typed, drawn))
}

#[cfg(test)]
#[path = "plate_tests.rs"]
mod tests;
