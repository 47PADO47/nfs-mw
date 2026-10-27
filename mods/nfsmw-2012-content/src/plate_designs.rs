//! The plate designs of the 2012 game as one set shared by every car: `files/PLATES/MW2012/DESIGNS.TOML` (the index) and
//! `files/PLATES/MW2012/DESIGNS.BIN` (the pictures). The cars' `[customization.plate]` tables name the index
//! (`designs = ...`), and the garage offers each design under License Plate, Plate Design.
//! Spec: `docs/formats/chameleon-plate-designs.md`.
//!
//! The 2012 game has 666 designs (Autolog backgrounds the player earns) over 469 pictures of 512 x 256 pixels, a plate over a
//! photograph. Written whole they would be 60 MB, and the car's own plate shows only the glyph strip of the plate (a band about
//! 4.4 times wider than high). So each picture is cut to the band that lies behind the glyphs (`chameleon_vehicle::
//! plate_design_band`), scaled to 256 x 64 and stored as DXT1: 8 KB a picture, about 4 MB for all, written once, not once per
//! car. Designs that share a picture are one design (the first one's name); names that repeat are numbered.

use std::collections::HashMap;
use std::fmt::Write as _;

use anyhow::{Context, Result};
use blackbox_tpk::{PixelFormat, TextureSource, write_tpk};
use chameleon_vehicle::{
    PlateArea, PlateDesign, plate_design_band, plate_design_picture, plate_design_picture_path, plate_design_plate,
};
use rayon::prelude::*;

use crate::emit::ModFolder;
use crate::source::Source;

mod dxt;

/// The folder of the set under the mod's `files/`, and the index in it, which the cars' manifests name.
const FOLDER: &str = "PLATES/MW2012";
pub const INDEX: &str = "PLATES/MW2012/DESIGNS.TOML";
/// The texture pack beside the index.
const PACK: &str = "DESIGNS.BIN";
/// Size of a stored picture: the band behind the glyph strip.
const BAND: (usize, usize) = (256, 64);
/// The design every car wears until the player picks another. The 2012 data does not say which one the game gives a new
/// player (the id is a field of the session's player slot and nothing the install sets), so none is: the plate keeps the
/// ground of its glyph sheet until a design is picked. Set it to a design id (a picture number, `"1863272"` is the first of
/// the game's list, "START ME UP") to dress every car from the start.
pub const DEFAULT_DESIGN: Option<&str> = None;
/// The labels of the five game packs, by pack number (`docs/formats/chameleon-plate.md`).
const GROUPS: [&str; 5] =
    ["Base game", "Ultimate Speed Pack", "Terminal Velocity Pack", "Movie Legends Pack", "NFS Heroes Pack"];

/// One design as the index lists it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    /// The picture's number, as text: the design's id.
    pub id: String,
    pub name: String,
    pub group: &'static str,
    /// The number of the picture in the install.
    pub picture: u32,
}

impl Entry {
    /// The name of the picture's texture in the pack.
    pub fn texture(&self) -> String {
        format!("PD{}", self.picture)
    }
}

/// The entries for `designs`, in the game's order: one for each distinct picture, named after the first design that uses it,
/// with `name_of` giving a name id's text and `group_of` a pack id's label. A name already taken gets its count (`NAME 2`).
pub fn entries(
    designs: &[PlateDesign],
    name_of: impl Fn(u32) -> Option<String>,
    group_of: impl Fn(u32) -> &'static str,
) -> Vec<Entry> {
    let mut pictures: Vec<u32> = Vec::new();
    let mut taken: HashMap<String, usize> = HashMap::new();
    let mut entries = Vec::new();
    for design in designs {
        if pictures.contains(&design.picture) {
            continue;
        }
        pictures.push(design.picture);
        let name = name_of(design.name).map(|text| text.trim().to_owned()).filter(|text| !text.is_empty());
        let name = name.unwrap_or_else(|| format!("Design {}", design.picture));
        let count = taken.entry(name.clone()).or_insert(0);
        *count += 1;
        let name = match *count {
            1 => name,
            n => format!("{name} {n}"),
        };
        entries.push(Entry {
            id: design.picture.to_string(),
            name,
            group: group_of(design.pack),
            picture: design.picture,
        });
    }
    entries
}

/// `rgba` (`stride` pixels a row) over the area `from`, scaled to `to` pixels by averaging the source pixels each one covers.
fn resample(rgba: &[u8], stride: usize, from: PlateArea, to: (usize, usize)) -> Vec<u8> {
    let (x0, y0, width, height) = from;
    let mut out = Vec::with_capacity(to.0 * to.1 * 4);
    for y in 0..to.1 {
        let (top, bottom) = (y0 + y * height / to.1, y0 + ((y + 1) * height / to.1).max(y * height / to.1 + 1));
        for x in 0..to.0 {
            let (left, right) = (x0 + x * width / to.0, x0 + ((x + 1) * width / to.0).max(x * width / to.0 + 1));
            let mut sum = [0u32; 3];
            let mut count = 0;
            for sy in top..bottom {
                for sx in left..right {
                    let at = (sy * stride + sx) * 4;
                    sum.iter_mut().zip(&rgba[at..at + 3]).for_each(|(total, value)| *total += u32::from(*value));
                    count += 1;
                }
            }
            out.extend(sum.map(|total| (total / count) as u8));
            out.push(255);
        }
    }
    out
}

/// The stored picture of a design picture `rgba` (`width` x `height`): its band, scaled and encoded. `None` for a picture with
/// no solid pixel.
fn band_texture(name: &str, rgba: &[u8], width: usize, height: usize) -> Option<TextureSource> {
    let plate = plate_design_plate(rgba, width, height)?;
    let band = plate_design_band(plate);
    let pixels = resample(rgba, width, band, BAND);
    let data = dxt::encode(&pixels, BAND.0, BAND.1);
    Some(TextureSource::new(name, BAND.0 as u32, BAND.1 as u32, PixelFormat::Dxt1, 1, data))
}

/// What a run of the writer produced.
#[derive(Debug)]
pub struct Built {
    /// The designs whose picture is in the pack.
    pub entries: Vec<Entry>,
    /// The texture pack.
    pub pack: Vec<u8>,
}

/// The designs of the install with their pictures cut, encoded and packed. `None` when the install lists no designs.
pub fn build(source: &Source) -> Option<Built> {
    let plates = source.plates()?;
    if plates.designs().is_empty() {
        return None;
    }
    let group_of = |pack: u32| GROUPS[usize::from(plates.pack_number(pack)).min(GROUPS.len() - 1)];
    let listed = entries(plates.designs(), |id| source.text(id), group_of);
    let made: Vec<Option<TextureSource>> = listed.par_iter().map(|entry| picture(source, entry)).collect();
    let (entries, textures): (Vec<Entry>, Vec<TextureSource>) =
        listed.into_iter().zip(made).filter_map(|(entry, texture)| Some((entry, texture?))).unzip();
    if entries.is_empty() {
        return None;
    }
    let pack = write_tpk("designs", &textures).map_err(|error| log::warn!("the plate design pack: {error}")).ok()?;
    Some(Built { entries, pack })
}

/// The stored picture of `entry`; a picture that does not read costs only its design.
fn picture(source: &Source, entry: &Entry) -> Option<TextureSource> {
    let result = (|| -> Result<TextureSource> {
        let data = source
            .install_file(&plate_design_picture_path(entry.picture))
            .with_context(|| format!("picture {} is not in the install", entry.picture))?;
        let texture = plate_design_picture(&data, entry.picture)?;
        let rgba = texture.to_rgba8().context("the picture's format cannot be read")?;
        band_texture(&entry.texture(), &rgba, texture.width as usize, texture.height as usize)
            .context("the picture has no plate in it")
    })();
    result.map_err(|error| log::warn!("plate design {:?} is left out: {error:#}", entry.name)).ok()
}

/// A TOML basic string.
fn quote(text: &str) -> String {
    let escaped: String = text
        .chars()
        .flat_map(|c| match c {
            '\\' => vec!['\\', '\\'],
            '"' => vec!['\\', '"'],
            c if c.is_control() => Vec::new(),
            c => vec![c],
        })
        .collect();
    format!("\"{escaped}\"")
}

/// The text of the index file for `entries` (`custom::plate_design`).
pub fn index_text(entries: &[Entry]) -> String {
    let mut text = String::from(
        "# The plate designs of the NFS Most Wanted (2012) install, written by nfsmw-2012-content: do not edit.\n",
    );
    let _ = writeln!(text, "pack = {}", quote(PACK));
    for entry in entries {
        let _ = write!(
            text,
            "\n[[design]]\nid = {}\nname = {}\npicture = {}\ngroup = {}\n",
            quote(&entry.id),
            quote(&entry.name),
            quote(&entry.texture()),
            quote(entry.group)
        );
    }
    text
}

/// Write the set into the mod folder: the index and the pack. Returns how many designs were written. A failure costs the
/// designs only: the cars are converted, and their plates keep the sheet's own ground.
pub fn write(source: &Source, folder: &ModFolder) -> usize {
    let Some(built) = build(source) else {
        log::warn!("the 2012 install has no plate designs that can be read: the plates have none");
        return 0;
    };
    let target = folder.root().join("files").join(FOLDER);
    let written = std::fs::create_dir_all(&target)
        .and_then(|()| std::fs::write(target.join("DESIGNS.TOML"), index_text(&built.entries)))
        .and_then(|()| std::fs::write(target.join(PACK), &built.pack));
    if let Err(error) = written {
        log::warn!("the plate designs are not written: {error}");
        return 0;
    }
    log::info!("{} plate designs written ({} KB)", built.entries.len(), built.pack.len() / 1024);
    built.entries.len()
}

/// The keys the car's `[customization.plate]` table gets for the designs: the index, and the default design if there is one.
/// Empty when the install has no designs (so a plate that cannot offer any names none).
pub fn manifest_keys(source: &Source) -> String {
    if !source.plates().is_some_and(|plates| !plates.designs().is_empty()) {
        return String::new();
    }
    let mut keys = format!("designs = {}\n", quote(INDEX));
    if let Some(design) = DEFAULT_DESIGN {
        let _ = writeln!(keys, "default_design = {}", quote(design));
    }
    keys
}

#[cfg(test)]
#[path = "plate_designs_tests.rs"]
mod tests;
