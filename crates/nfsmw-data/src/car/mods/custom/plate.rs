//! A mod car's licence plate (`[customization.plate]`, docs/modding/car-customization.md): a glyph sheet in the car's own
//! `TEXTURES.BIN`, drawn again with the text the player picked.
//!
//! The sheet holds `columns` by `rows` equal cells, one glyph each, listed row by row in `glyphs`. The car's plate mesh has
//! `slots` quads and quad `n` is mapped to cell `n`, so the sheet as authored shows the first glyphs. Showing a text means
//! drawing a copy of the sheet whose first `slots` cells hold the glyphs of the text. The text is centred, and the slots it
//! leaves empty show the blank glyph (the space of the sheet).

use std::sync::OnceLock;

use anyhow::{Context, Result, ensure};
use blackbox_tpk::{AlphaUsage, PixelFormat, Texture, decode_rgba8};
use game_install::GameDir;

use super::Plate;
use crate::car::CarModel;

impl Plate {
    /// The glyph a typed character is drawn as: upper case, through the aliases (`0` as `O`), and only when the sheet has it.
    /// A space is the blank glyph.
    pub fn glyph(&self, typed: char) -> Option<char> {
        let upper = typed.to_ascii_uppercase();
        let drawn = self.aliases.iter().find(|(from, _)| *from == upper).map_or(upper, |(_, to)| *to);
        self.glyphs.contains(&drawn).then_some(drawn)
    }

    /// The text as the plate keeps it: every character the sheet lacks becomes a space, and at most `slots` are kept.
    pub fn registration(&self, text: &str) -> String {
        text.chars().take(self.slots as usize).map(|c| self.glyph(c).unwrap_or(' ')).collect()
    }

    /// The cell of the sheet each slot shows for `text`: the text centred, the other slots blank.
    pub fn cells(&self, text: &str) -> Vec<usize> {
        let slots = self.slots as usize;
        let blank = self.glyphs.iter().position(|c| *c == ' ').unwrap_or(0);
        let shown: Vec<char> = self.registration(text).chars().collect();
        let start = (slots - shown.len()) / 2;
        let mut cells = vec![blank; slots];
        for (at, glyph) in shown.into_iter().enumerate() {
            cells[start + at] = self.glyphs.iter().position(|c| *c == glyph).unwrap_or(blank);
        }
        cells
    }

    /// The text the car wears: the one the player picked, else the manifest's, else none (the sheet as authored).
    pub fn worn<'a>(&'a self, picked: Option<&'a str>) -> Option<&'a str> {
        picked.or(self.text.as_deref())
    }

    /// The sheet drawn again with `text` in its first cells, with a full chain of mip levels. `picture` is the sheet as the
    /// car's `TEXTURES.BIN` holds it.
    pub fn compose(&self, picture: &Texture, text: &str) -> Result<Texture> {
        let (width, height) = (picture.width as usize, picture.height as usize);
        let (columns, rows) = (self.columns as usize, self.rows as usize);
        ensure!(width >= columns && height >= rows, "the plate sheet is smaller than its cells");
        let sheet = decode_rgba8(picture).context("the plate sheet cannot be decoded")?;
        let mut pixels = sheet.clone();
        for (slot, source) in self.cells(text).into_iter().enumerate() {
            let target = cell_rect(slot, columns, rows, width, height);
            let from = cell_rect(source, columns, rows, width, height);
            copy_cell(&sheet, &mut pixels, width, from, target);
        }
        Ok(with_mips(picture, pixels))
    }
}

/// The pixels of cell `n`: `(x0, y0, x1, y1)`, the cells being equal parts of the sheet (a cell's edge is rounded to a pixel).
fn cell_rect(n: usize, columns: usize, rows: usize, width: usize, height: usize) -> (usize, usize, usize, usize) {
    let (column, row) = (n % columns, n / columns);
    let edge = |at: usize, count: usize, size: usize| (at * size + count / 2) / count;
    (
        edge(column, columns, width),
        edge(row, rows, height),
        edge(column + 1, columns, width),
        edge(row + 1, rows, height),
    )
}

/// Copy the cell `from` of `source` over the cell `target` of `pixels` (both RGBA, `width` wide), taking the nearest pixel when
/// the two cells differ in size by a rounding.
fn copy_cell(
    source: &[u8],
    pixels: &mut [u8],
    width: usize,
    from: (usize, usize, usize, usize),
    target: (usize, usize, usize, usize),
) {
    let (from_width, from_height) = (from.2 - from.0, from.3 - from.1);
    let (target_width, target_height) = (target.2 - target.0, target.3 - target.1);
    if from_width == 0 || from_height == 0 || target_width == 0 || target_height == 0 {
        return;
    }
    for y in 0..target_height {
        let source_y = from.1 + y * from_height / target_height;
        for x in 0..target_width {
            let source_x = from.0 + x * from_width / target_width;
            let (read, write) = ((source_y * width + source_x) * 4, ((target.1 + y) * width + target.0 + x) * 4);
            pixels[write..write + 4].copy_from_slice(&source[read..read + 4]);
        }
    }
}

/// `picture`'s name and settings with `rgba` (`width * height` pixels) as an A8R8G8B8 texture and every mip level down to 1x1,
/// each the average of 2x2 pixels of the one above.
fn with_mips(picture: &Texture, rgba: Vec<u8>) -> Texture {
    let (mut width, mut height) = (picture.width as usize, picture.height as usize);
    let mut level = rgba;
    let mut data = Vec::new();
    let mut levels = 0;
    loop {
        data.extend(level.as_chunks::<4>().0.iter().flat_map(|p| [p[2], p[1], p[0], p[3]]));
        levels += 1;
        if width == 1 && height == 1 {
            break;
        }
        let (next_width, next_height) = ((width / 2).max(1), (height / 2).max(1));
        level = halve(&level, (width, height), (next_width, next_height));
        (width, height) = (next_width, next_height);
    }
    Texture {
        mip_levels: levels,
        format: PixelFormat::Argb8888,
        compression_type: 0,
        alpha_usage: AlphaUsage::None,
        data,
        palette: Vec::new(),
        ..picture.clone()
    }
}

/// One mip level down: each pixel is the mean of the (up to) 2x2 pixels it covers.
fn halve(level: &[u8], (width, height): (usize, usize), (next_width, next_height): (usize, usize)) -> Vec<u8> {
    let mut out = Vec::with_capacity(next_width * next_height * 4);
    for y in 0..next_height {
        for x in 0..next_width {
            let mut sum = [0u32; 4];
            let mut count = 0;
            for (dy, dx) in [(0, 0), (0, 1), (1, 0), (1, 1)] {
                let (sx, sy) = ((2 * x + dx).min(width - 1), (2 * y + dy).min(height - 1));
                let at = (sy * width + sx) * 4;
                sum.iter_mut().zip(&level[at..at + 4]).for_each(|(total, value)| *total += u32::from(*value));
                count += 1;
            }
            out.extend(sum.map(|total| (total / count) as u8));
        }
    }
    out
}

/// Where the player's own plate texts are kept: a function from a car's folder to the text typed for it. The host sets it
/// once (the texts are in the profile's own data, beside the save); without it every car wears the text of its manifest.
static SOURCE: OnceLock<fn(&str) -> Option<String>> = OnceLock::new();

/// Set where the player's plate texts are read from. The first call wins.
pub fn set_source(source: fn(&str) -> Option<String>) {
    let _ = SOURCE.set(source);
}

/// The text the player kept for the car in `folder`, if any.
pub fn stored(folder: &str) -> Option<String> {
    SOURCE.get().and_then(|source| source(folder))
}

/// Draw the car's plate on `model` with the text it wears: `picked` (a preview in the garage), else the one the player kept
/// ([`stored`]), else the manifest's. A car whose sheet is missing from its `TEXTURES.BIN` keeps the sheet as authored.
pub(super) fn apply(
    dir: &GameDir,
    folder: &str,
    plate: &Plate,
    picked: Option<&str>,
    model: &mut CarModel,
) -> Result<()> {
    let kept = stored(folder);
    let Some(text) = plate.worn(picked.or(kept.as_deref())) else { return Ok(()) };
    let wanted = blackbox_hash::bstring_hash(&plate.picture);
    let found = super::livery::own_textures(dir, folder, &std::collections::HashSet::from([wanted]))?;
    let picture = found.get(&wanted).context("the plate sheet is missing from TEXTURES.BIN")?;
    let drawn = plate.compose(picture, text)?;
    model.textures.insert(drawn.name_hash, drawn);
    Ok(())
}

#[cfg(test)]
#[path = "plate_tests.rs"]
mod tests;
