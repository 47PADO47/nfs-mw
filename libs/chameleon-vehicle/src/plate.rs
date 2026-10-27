//! The licence plate of a 2012 car: how the game turns a registration text into glyphs, and where its default text lives.
//! Spec: `docs/formats/chameleon-plate.md`.
//!
//! The body of every car carries one small mesh with the shader technique `..._Licenseplate`: eight quads side by side,
//! one per character. Its diffuse texture is a **glyph sheet**, 9 columns by 4 rows of cells (the 36 glyphs of
//! [`GLYPHS`]), and quad `n` is mapped to cell `n` of the first row, so the mesh alone shows `ABCDEFGH`. The game
//! chooses another text by telling the shader which cell each quad shows (the constant `VehicleLicencePlateOffsets`).
//!
//! The text is **not** a property of the car: it is the registration string of the player, which the player may set in
//! Autolog; until then it is a default that depends on the *game pack* the car belongs to (the base game, or one of four
//! DLC packs). The five default strings are in the executable; the pack of a car is the `GamePackID` of its vehicle list
//! entry, and the pack's number is in the `Genesys.Gen.GamePack` objects of `GAMELOGIC/GAMEPLAY.BNDL`.

use chameleon_bundle::Bundle;

use crate::error::{Error, Result};
use crate::genesys::{TypeNames, find_object};

/// Characters the plate shows at once: the quads of the plate mesh.
pub const SLOTS: usize = 8;
/// The glyph of each cell of the sheet, row by row. The blank glyph is the space, which the sheet draws as a dash.
pub const GLYPHS: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZ 123456789";
/// Cells across and down the glyph sheet.
pub const COLUMNS: u32 = 9;
pub const ROWS: u32 = 4;
/// The cell of the blank glyph, which an empty slot shows.
pub const BLANK: u8 = 26;

/// The name of the shader technique of the plate's material.
const TECHNIQUE: &str = "Licenseplate";
/// The type the game registers for a game pack.
const PACK_TYPE: &str = "Genesys.Gen.GamePack";
/// Fields of a game pack: its id (the `GamePackID` of vehicle list entries) and its number.
const PACK_ID: usize = 0x1C;
const PACK_NUMBER: usize = 0x28;
/// More packs than this is not a pack list (the install has 5).
const MAX_PACKS: usize = 64;
/// Bytes between the default strings in the executable, and how many there are: the base game and packs 1 to 4.
const RECORD: usize = 12;
const DEFAULTS: usize = 5;

/// Whether a material with this shader technique is the plate (its glyph sheet is the diffuse texture).
pub fn is_plate_technique(technique: &str) -> bool {
    technique.contains(TECHNIQUE)
}

/// The registration the game keeps for typed `text`: letters in upper case, `0` as `O`, and every character the glyph sheet
/// lacks (including a space) as a space.
pub fn registration(text: &str) -> String {
    text.chars().map(glyph_of).collect()
}

fn glyph_of(c: char) -> char {
    let upper = c.to_ascii_uppercase();
    let upper = if upper == '0' { 'O' } else { upper };
    match GLYPHS.contains(upper) {
        true => upper,
        false => ' ',
    }
}

/// The cell of the glyph sheet each of the eight slots shows for `text`: the text centred (a text of `n` characters starts
/// at slot `(8 - n) / 2`), the other slots [`BLANK`]. Characters past the eighth are dropped.
pub fn cells(text: &str) -> [u8; SLOTS] {
    let mut out = [BLANK; SLOTS];
    let kept: Vec<char> = text.chars().take(SLOTS).collect();
    let start = (SLOTS - kept.len()) / 2;
    for (at, c) in kept.into_iter().enumerate() {
        let glyph = glyph_of(c);
        out[start + at] = GLYPHS.find(glyph).map_or(BLANK, |cell| cell as u8);
    }
    out
}

/// One game pack of `GAMELOGIC/GAMEPLAY.BNDL`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GamePack {
    /// The id a vehicle list entry names in its `GamePackID` field (+0x2C of the entry).
    pub id: u32,
    /// 0 for the base game, 1 to 4 for the four DLC packs.
    pub number: u16,
}

/// Every game pack of the gameplay bundle `gameplay`.
pub fn game_packs(gameplay: &Bundle) -> Result<Vec<GamePack>> {
    let types = TypeNames::read(gameplay)?;
    let object = find_object(gameplay, &types, PACK_TYPE)?;
    let offsets: Vec<usize> = object.all(PACK_TYPE).collect();
    if offsets.len() > MAX_PACKS {
        return Err(Error::Spec(format!("the bundle claims {} game packs", offsets.len())));
    }
    offsets
        .into_iter()
        .map(|at| Ok(GamePack { id: object.u32(at + PACK_ID)?, number: object.u16(at + PACK_NUMBER)? }))
        .collect()
}

/// The five default registrations of the executable `exe`: the base game's, then those of packs 1 to 4. `None` when the
/// executable does not hold them (another version of the game).
///
/// The strings sit one after another, each in a record of 12 bytes, the first being `NEED4SPD`.
pub fn default_texts(exe: &[u8]) -> Option<[String; DEFAULTS]> {
    let first = b"NEED4SPD\0";
    let mut from = 0;
    while let Some(found) = exe[from..].windows(first.len()).position(|window| window == first) {
        let start = from + found;
        if let Some(texts) = records(&exe[start..]) {
            return Some(texts);
        }
        from = start + 1;
    }
    None
}

/// The five records at the start of `bytes`: printable text, NUL-terminated inside the record.
fn records(bytes: &[u8]) -> Option<[String; DEFAULTS]> {
    let mut texts = Vec::with_capacity(DEFAULTS);
    for n in 0..DEFAULTS {
        let record = bytes.get(n * RECORD..(n + 1) * RECORD)?;
        let end = record.iter().position(|&byte| byte == 0)?;
        let text = &record[..end];
        if text.is_empty() || !text.iter().all(|byte| (0x20..0x7F).contains(byte)) {
            return None;
        }
        texts.push(String::from_utf8_lossy(text).into_owned());
    }
    texts.try_into().ok()
}

/// The default registration of a car of pack `number`: pack 1 to 4 have a text of their own, any other car (the base game's)
/// takes the first.
pub fn default_for(texts: &[String; DEFAULTS], number: u16) -> &str {
    match texts.get(usize::from(number)) {
        Some(text) if number != 0 => text,
        _ => &texts[0],
    }
}

#[cfg(test)]
#[path = "plate_tests.rs"]
mod tests;
