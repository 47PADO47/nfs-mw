//! A mod car's plate designs: backgrounds the player picks to put behind the glyphs of the plate
//! (`designs` of `[customization.plate]`, docs/modding/car-customization.md, "Plate designs").
//!
//! The designs are not part of a car: the plate names an **index** (a TOML file under the mod's `files/`) that every car of
//! the mod shares. The index lists the designs and names a texture pack beside it that holds their pictures, so the pictures
//! are written once, not once per car. A design picture is the ground of the plate's glyph strip: it is stretched over the
//! `slots` cells the plate shows, and the glyphs are drawn over it by [`lay_on_ground`] with their own ink.
//!
//! ```toml
//! pack = "DESIGNS.BIN"            # a texture pack in the folder of the index
//!
//! [[design]]
//! id = "1863272"                  # letters, digits, _ and -; kept in the player's plates.toml
//! name = "START ME UP"            # shown in the garage
//! picture = "PD1863272"           # a texture of the pack: the ground behind the glyphs
//! group = "Base game"             # optional: a category label
//! glyphs = "PG1863272"            # optional: a glyph sheet in the car's layout, instead of the car's own
//! ink = [20, 20, 24]              # optional: the glyph colour (else dark on a light picture, light on a dark one)
//! ```

use std::collections::HashSet;
use std::sync::Arc;

use anyhow::{Context, Result, ensure};
use blackbox_hash::bstring_hash;
use blackbox_tpk::{Texture, decode_rgba8, read_texture_packs_filtered};
use game_install::GameDir;
use serde::Deserialize;

use super::Plate;
use crate::car::customization::PlateDesignChoice;

/// More designs than this is not an index (the 2012 install has 469 distinct pictures).
pub const MAX_DESIGNS: usize = 4096;
/// Longest design id, name and texture name, in bytes.
const MAX_ID: usize = 64;
const MAX_NAME: usize = 64;
const MAX_TEXTURE: usize = 56;

/// One design of an index.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Design {
    pub id: String,
    pub name: String,
    pub group: Option<String>,
    /// The texture of the pack that is the ground behind the glyphs.
    pub picture: String,
    /// A glyph sheet of the pack that replaces the car's own while the design is worn.
    pub glyphs: Option<String>,
    /// The glyph colour; `None` chooses dark or light from the picture.
    pub ink: Option<[u8; 3]>,
}

/// A design index read: where its pack is, and the designs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DesignSet {
    /// The path of the texture pack, relative to the install.
    pub pack: String,
    pub designs: Vec<Design>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawIndex {
    pack: String,
    #[serde(default)]
    design: Vec<RawDesign>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawDesign {
    id: String,
    name: String,
    picture: String,
    #[serde(default)]
    group: Option<String>,
    #[serde(default)]
    glyphs: Option<String>,
    #[serde(default)]
    ink: Option<[u8; 3]>,
}

fn plain_name(text: &str, longest: usize) -> bool {
    !text.is_empty() && text.len() <= longest && text.chars().all(|c| !c.is_control())
}

fn identifier(text: &str, longest: usize) -> bool {
    !text.is_empty()
        && text.len() <= longest
        && text.bytes().all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-'))
}

/// The folder of `index` (a path with `/`), with a trailing slash; empty for a file at the root.
fn folder_of(index: &str) -> &str {
    index.rfind('/').map_or("", |at| &index[..=at])
}

/// Read the index text of the file `index` (a path relative to the install). A design that is malformed or repeats an id is
/// left out with a warning; an index that is not TOML, or whose `pack` is not a plain file name, is an error.
pub fn parse_index(index: &str, text: &str) -> Result<DesignSet> {
    let raw: RawIndex = toml::from_str(text).context("the plate design index is not valid")?;
    ensure!(
        identifier(&raw.pack.replace('.', "_"), MAX_TEXTURE),
        "the plate design index names the pack {:?}: a file name beside the index",
        raw.pack
    );
    ensure!(raw.design.len() <= MAX_DESIGNS, "the plate design index lists {} designs", raw.design.len());
    let mut seen = HashSet::new();
    let mut designs = Vec::with_capacity(raw.design.len());
    for entry in raw.design {
        let valid = identifier(&entry.id, MAX_ID)
            && plain_name(&entry.name, MAX_NAME)
            && plain_name(&entry.picture, MAX_TEXTURE)
            && entry.glyphs.as_deref().is_none_or(|glyphs| plain_name(glyphs, MAX_TEXTURE))
            && entry.group.as_deref().is_none_or(|group| plain_name(group, MAX_NAME));
        if !valid || !seen.insert(entry.id.clone()) {
            log::warn!("{index}: the plate design {:?} is malformed or repeated: left out", entry.id);
            continue;
        }
        designs.push(Design {
            id: entry.id,
            name: entry.name,
            group: entry.group,
            picture: entry.picture,
            glyphs: entry.glyphs,
            ink: entry.ink,
        });
    }
    Ok(DesignSet { pack: format!("{}{}", folder_of(index), raw.pack), designs })
}

/// The design index `index` of the install, read once per session while the file is unchanged.
pub fn set(dir: &GameDir, index: &str) -> Result<Arc<DesignSet>> {
    crate::memo(dir, &[index], || {
        let bytes = dir.read(index).with_context(|| format!("reading {index}"))?;
        let text = String::from_utf8(bytes).with_context(|| format!("{index} is not UTF-8"))?;
        parse_index(index, &text)
    })
}

/// The designs the garage offers for `plate`: those of its index whose picture is in the pack. Empty (and a warning) when the
/// plate names no index, or the index or its pack cannot be read.
pub fn offered(dir: &GameDir, folder: &str, plate: &Plate) -> Vec<PlateDesignChoice> {
    let Some(index) = &plate.designs else { return Vec::new() };
    let found = set(dir, index).and_then(|set| {
        let pictures = pictures(dir, &set, set.designs.iter().map(|design| design.picture.as_str()))?;
        Ok((set, pictures))
    });
    let (set, pictures) = match found {
        Ok(found) => found,
        Err(error) => {
            log::warn!("{folder}: no plate designs are offered: {error:#}");
            return Vec::new();
        }
    };
    set.designs
        .iter()
        .filter(|design| pictures.contains(&bstring_hash(&design.picture)))
        .map(|design| PlateDesignChoice {
            id: design.id.clone(),
            name: design.name.clone(),
            group: design.group.clone(),
        })
        .collect()
}

/// The name hashes of the textures among `names` that the pack of `set` holds.
fn pictures<'a>(dir: &GameDir, set: &DesignSet, names: impl Iterator<Item = &'a str>) -> Result<HashSet<u32>> {
    let wanted: HashSet<u32> = names.map(bstring_hash).collect();
    let bytes = crate::shared_unwrapped(dir, &set.pack)?;
    let packs = read_texture_packs_filtered(&bytes, |hash| wanted.contains(&hash))
        .with_context(|| format!("reading the plate design pack {}", set.pack))?;
    Ok(packs.into_iter().flat_map(|pack| pack.textures).map(|texture| texture.name_hash).collect())
}

/// The textures of `names` in the pack of `set`, by name hash.
fn textures(dir: &GameDir, set: &DesignSet, names: &[&str]) -> Result<Vec<Texture>> {
    let wanted: HashSet<u32> = names.iter().map(|name| bstring_hash(name)).collect();
    let bytes = crate::shared_unwrapped(dir, &set.pack)?;
    let packs = read_texture_packs_filtered(&bytes, |hash| wanted.contains(&hash))
        .with_context(|| format!("reading the plate design pack {}", set.pack))?;
    Ok(packs.into_iter().flat_map(|pack| pack.textures).collect())
}

/// The picture behind the glyphs, decoded, and what goes with it.
#[derive(Debug, Clone, PartialEq)]
pub struct Ground {
    rgba: Vec<u8>,
    width: usize,
    height: usize,
    /// The glyph colour: the design's, else dark or light by the picture.
    pub ink: [u8; 3],
    /// A glyph sheet that replaces the car's while the design is worn.
    pub glyphs: Option<Texture>,
}

impl Ground {
    /// The ground of `picture`, glyphs coloured `ink` (`None`: chosen from the picture's brightness).
    pub fn new(picture: &Texture, ink: Option<[u8; 3]>) -> Result<Self> {
        let rgba = decode_rgba8(picture).context("the plate design picture cannot be decoded")?;
        let (width, height) = (picture.width as usize, picture.height as usize);
        ensure!(width > 0 && height > 0 && rgba.len() >= width * height * 4, "the plate design picture is empty");
        let ink = ink.unwrap_or_else(|| match mean_luminance(&rgba) > 128.0 {
            true => DARK_INK,
            false => LIGHT_INK,
        });
        Ok(Self { rgba, width, height, ink, glyphs: None })
    }

    /// The colour of the picture at `(u, v)`, each from 0 to 1 over its width and height, linearly blended.
    fn sample(&self, u: f32, v: f32) -> [f32; 3] {
        let x = (u * self.width as f32 - 0.5).clamp(0.0, (self.width - 1) as f32);
        let y = (v * self.height as f32 - 0.5).clamp(0.0, (self.height - 1) as f32);
        let (x0, y0) = (x.floor() as usize, y.floor() as usize);
        let (x1, y1) = ((x0 + 1).min(self.width - 1), (y0 + 1).min(self.height - 1));
        let (fx, fy) = (x - x0 as f32, y - y0 as f32);
        let pixel = |px: usize, py: usize, channel: usize| f32::from(self.rgba[(py * self.width + px) * 4 + channel]);
        let mut out = [0.0; 3];
        for (channel, value) in out.iter_mut().enumerate() {
            let top = pixel(x0, y0, channel) * (1.0 - fx) + pixel(x1, y0, channel) * fx;
            let bottom = pixel(x0, y1, channel) * (1.0 - fx) + pixel(x1, y1, channel) * fx;
            *value = top * (1.0 - fy) + bottom * fy;
        }
        out
    }
}

/// Glyph colours chosen by the picture: dark on a light one, light on a dark one.
const DARK_INK: [u8; 3] = [20, 20, 24];
const LIGHT_INK: [u8; 3] = [245, 245, 238];

fn luminance(r: u8, g: u8, b: u8) -> f32 {
    0.299 * f32::from(r) + 0.587 * f32::from(g) + 0.114 * f32::from(b)
}

fn mean_luminance(rgba: &[u8]) -> f32 {
    let pixels = rgba.as_chunks::<4>().0;
    let total: f32 = pixels.iter().map(|p| luminance(p[0], p[1], p[2])).sum();
    total / pixels.len().max(1) as f32
}

/// The design `id` of the plate's index with its picture decoded. `Err` when the plate has no index, the index lacks the
/// design, or its picture is not in the pack. A glyph sheet the design names that cannot be used is dropped with a warning:
/// the car's own sheet is drawn instead.
pub fn ground(dir: &GameDir, plate: &Plate, id: &str) -> Result<Ground> {
    let index = plate.designs.as_deref().context("the plate has no designs")?;
    let set = set(dir, index)?;
    let design =
        set.designs.iter().find(|design| design.id == id).with_context(|| format!("no plate design {id:?}"))?;
    let names: Vec<&str> = std::iter::once(design.picture.as_str()).chain(design.glyphs.as_deref()).collect();
    let found = textures(dir, &set, &names)?;
    let find = |name: &str| found.iter().find(|texture| texture.name_hash == bstring_hash(name));
    let picture =
        find(&design.picture).with_context(|| format!("the picture {:?} is not in the pack", design.picture))?;
    let mut ground = Ground::new(picture, design.ink)?;
    ground.glyphs = design.glyphs.as_deref().and_then(find).cloned();
    Ok(ground)
}

/// The pixel levels of a sheet's glyph cells, as luminance: `(ground, ink)`, the ground being what most of the sheet is and
/// the ink its darkest part. `None` for a sheet with no contrast (nothing to tell glyph from ground).
pub(super) fn levels(rgba: &[u8]) -> Option<(f32, f32)> {
    let mut histogram = [0usize; 256];
    for pixel in rgba.as_chunks::<4>().0 {
        histogram[luminance(pixel[0], pixel[1], pixel[2]) as usize] += 1;
    }
    let total: usize = histogram.iter().sum();
    let at = |fraction: f32| {
        let target = (total as f32 * fraction) as usize;
        let mut seen = 0;
        for (level, count) in histogram.iter().enumerate() {
            seen += count;
            if seen > target {
                return level as f32;
            }
        }
        255.0
    };
    let (ground, ink) = (at(0.9), at(0.03));
    (ground - ink >= MIN_CONTRAST).then_some((ground, ink))
}

/// Least difference between the ground and the ink of a sheet for the glyphs to be told apart.
const MIN_CONTRAST: f32 = 24.0;
/// How much of the picture is pulled toward the opposite of the ink, so the glyphs stay readable on a busy picture.
const SCRIM: f32 = 0.38;

/// Redraw the `slots` cells (`(x0, y0, x1, y1)` each, in order) of `pixels` (RGBA, `width` wide) that already hold the glyphs
/// of the text on the sheet's own ground: the ground becomes `ground`, stretched over the cells side by side, and the glyphs
/// keep their shape in the ink of `ground`. `levels` is [`levels`] of the sheet.
pub(super) fn lay_on_ground(
    pixels: &mut [u8],
    width: usize,
    slots: &[(usize, usize, usize, usize)],
    levels: (f32, f32),
    ground: &Ground,
) {
    let (light, dark) = levels;
    let ink = ground.ink.map(f32::from);
    // A dark ink lightens the picture, a light ink darkens it.
    let pole = match luminance(ground.ink[0], ground.ink[1], ground.ink[2]) < 128.0 {
        true => 255.0,
        false => 0.0,
    };
    for (slot, &(x0, y0, x1, y1)) in slots.iter().enumerate() {
        let (cell_width, cell_height) = (x1.saturating_sub(x0), y1.saturating_sub(y0));
        for y in 0..cell_height {
            let v = (y as f32 + 0.5) / cell_height as f32;
            for x in 0..cell_width {
                let u = (slot as f32 + (x as f32 + 0.5) / cell_width as f32) / slots.len() as f32;
                let at = ((y0 + y) * width + x0 + x) * 4;
                let glyph = luminance(pixels[at], pixels[at + 1], pixels[at + 2]);
                // How much of the pixel is glyph: a little dead zone keeps the sheet's own grain out of the ink.
                let cover = (((light - glyph) / (light - dark)) * 1.1 - 0.1).clamp(0.0, 1.0);
                let behind = ground.sample(u, v);
                for channel in 0..3 {
                    let eased = behind[channel] * (1.0 - SCRIM) + pole * SCRIM;
                    let value = eased * (1.0 - cover) + ink[channel] * cover;
                    pixels[at + channel] = value.round().clamp(0.0, 255.0) as u8;
                }
                pixels[at + 3] = 255;
            }
        }
    }
}

#[cfg(test)]
#[path = "plate_design_tests.rs"]
mod tests;
