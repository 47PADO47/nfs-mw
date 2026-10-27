//! The brands of the converted cars as manufacturers of the mod: `manufacturers/<id>.toml` and the logo it names,
//! `files/UI/MAKERS/<ID>.PNG` (`docs/modding/api-content.md`, "Manufacturers").
//!
//! The 2012 game keeps a table of manufacturers in its vehicle list (name, logo picture), and the pictures as small
//! bundles under `UI/IMAGES/STREAMED/` (`docs/formats/chameleon-manufacturers.md`). A car's make, as `names.rs` takes it
//! from the engine's object name, is matched to a manufacturer by name; the logo is drawn on the canvas of `logo.rs` and
//! written as a PNG. A make with no logo in the install gets no manufacturer, and the game draws the name itself.

use std::collections::HashSet;
use std::fmt::Write as _;

use anyhow::{Context, Result};
use chameleon_bundle::{Bundle, ResourceId, ResourceKind};
use chameleon_gfx::Texture;
use chameleon_vehicle::{Manufacturer, localised_text, manufacturers};

use crate::emit::ModFolder;
use crate::logo;
use crate::source::Source;

/// The vehicle list, whose trailing table is the manufacturers.
const VEHICLE_LIST: &str = "VEHICLES/VEHICLELIST.BNDL";
/// The English text tables, where the manufacturers' names are.
const LANGUAGE: &str = "UI/LANGUAGE/0001.BNDL";
/// The folder of the picture bundles, one per picture, named by the decimal id of the picture.
const PICTURES: &str = "UI/IMAGES/STREAMED";
/// Where the logos are written, under the mod's `files/` folder.
const LOGO_FOLDER: &str = "UI/MAKERS";
/// The longest brand name the game takes.
const MAX_NAME_CHARS: usize = 32;
/// The key part of a picture's id: a numeric id has the class byte 1.
const NUMERIC_CLASS: u64 = 1 << 56;

/// A manufacturer of the 2012 install with its logo on the canvas.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Maker {
    /// The name the install gives it (`ASTON MARTIN`).
    pub name: String,
    /// RGBA pixels of the [`logo::CANVAS`].
    pub canvas: Vec<u8>,
}

/// A brand ready to be written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Brand {
    /// The file stem of the manifest: lower case letters, digits and underscores.
    pub id: String,
    /// The brand as the cars name it.
    pub name: String,
    pub png: Vec<u8>,
}

/// A brand name reduced for comparison, the way the game compares them: upper case letters and digits only.
fn key(name: &str) -> String {
    name.chars().filter(|c| c.is_alphanumeric()).flat_map(char::to_uppercase).collect()
}

/// Whether the make of a car and the name of a manufacturer of the install are one brand: the same name, or one that is
/// the other's first part (`Mercedes` and `MERCEDES-BENZ`), at least four characters long.
pub fn same_brand(make: &str, maker: &str) -> bool {
    let (make, maker) = (key(make), key(maker));
    if make.is_empty() || maker.is_empty() {
        return false;
    }
    let (short, long) = if make.len() <= maker.len() { (&make, &maker) } else { (&maker, &make) };
    short == long || (short.len() >= 4 && long.starts_with(short.as_str()))
}

/// The manifest id of a brand: its letters and digits in lower case, runs of anything else as one underscore.
fn brand_id(name: &str) -> String {
    let mut id = String::new();
    for c in name.chars() {
        if c.is_ascii_alphanumeric() {
            id.push(c.to_ascii_lowercase());
            continue;
        }
        if !id.is_empty() && !id.ends_with('_') {
            id.push('_');
        }
    }
    id.trim_end_matches('_').to_owned()
}

/// The text of `manufacturers/<id>.toml` for a brand whose logo is the file `logo` under `files/`.
fn manifest(name: &str, logo: &str) -> String {
    let quote = |text: &str| format!("\"{}\"", text.replace('\\', "\\\\").replace('"', "\\\""));
    let mut text = String::new();
    let _ = writeln!(text, "name = {}", quote(name));
    let _ = writeln!(text, "logo = {}", quote(logo));
    text.push_str("override = false\n");
    text
}

/// The brands for `makes` (a make once, in the order first seen): those that have a logo among `makers`.
pub fn brands_for<'a>(makes: impl IntoIterator<Item = &'a str>, makers: &[Maker]) -> Result<Vec<Brand>> {
    let mut seen = HashSet::new();
    let mut brands = Vec::new();
    for make in makes {
        let make = make.trim();
        let id = brand_id(make);
        if id.is_empty() || make.chars().count() > MAX_NAME_CHARS || make.chars().any(char::is_control) {
            continue;
        }
        if !seen.insert(key(make)) {
            continue;
        }
        let Some(maker) = makers.iter().find(|maker| same_brand(make, &maker.name)) else { continue };
        let png = logo::encode_png(&maker.canvas, logo::CANVAS.0, logo::CANVAS.1)?;
        brands.push(Brand { id, name: make.to_owned(), png });
    }
    Ok(brands)
}

/// The manufacturers of the install that have a logo, drawn on the canvas. Empty when the install has no table that
/// reads; a manufacturer whose name, picture or bundle does not read is left out.
pub fn makers(source: &Source) -> Vec<Maker> {
    let Some(table) = source.install_file(VEHICLE_LIST).and_then(|data| manufacturers(&data)) else {
        return Vec::new();
    };
    let Some(names) = source.install_file(LANGUAGE).and_then(|data| text_block(&data)) else { return Vec::new() };
    table.iter().filter_map(|entry| maker(source, entry, &names)).collect()
}

fn maker(source: &Source, entry: &Manufacturer, names: &[u8]) -> Option<Maker> {
    if entry.logo == 0 {
        return None;
    }
    let name = localised_text(names, entry.name)?;
    let data = source.install_file(&format!("{PICTURES}/{}.BNDL", entry.logo))?;
    let texture = logo_texture(&data, entry.logo).map_err(|error| log::warn!("the logo of {name}: {error:#}")).ok()?;
    let rgba = texture.to_rgba8()?;
    let canvas = logo::on_canvas(&rgba, texture.width as usize, texture.height as usize)?;
    Some(Maker { name, canvas })
}

/// The main block of the text table in the bundle `data`.
fn text_block(data: &[u8]) -> Option<Vec<u8>> {
    let bundle = Bundle::parse(data).ok()?;
    let kind = ResourceKind::LocalisedText.id();
    let index = bundle.entries().iter().position(|entry| entry.kind == kind)?;
    Some(bundle.resource(index).ok()?.blocks[0].clone())
}

/// The texture with numeric id `logo` in the picture bundle `data`.
fn logo_texture(data: &[u8], logo: u32) -> Result<Texture> {
    let bundle = Bundle::parse(data)?;
    let kind = ResourceKind::Texture.id();
    let index = bundle
        .find(NUMERIC_CLASS | u64::from(logo))
        .or_else(|| bundle.entries().iter().position(|e| e.kind == kind && ResourceId(e.id).key() == logo))
        .context("the bundle has no such picture")?;
    let resource = bundle.resource(index)?;
    Ok(Texture::parse(&resource.blocks[0], &resource.blocks[1])?)
}

/// Write `brand` into the mod folder: its manifest and its logo.
fn write(folder: &ModFolder, brand: &Brand) -> Result<()> {
    let file = format!("{LOGO_FOLDER}/{}.PNG", brand.id.to_ascii_uppercase());
    let png_path = folder.root().join("files").join(&file);
    std::fs::create_dir_all(png_path.parent().context("a logo path has a folder")?)?;
    std::fs::write(&png_path, &brand.png).with_context(|| format!("writing {}", png_path.display()))?;
    let manifests = folder.root().join("manufacturers");
    std::fs::create_dir_all(&manifests)?;
    let path = manifests.join(format!("{}.toml", brand.id));
    std::fs::write(&path, manifest(&brand.name, &file)).with_context(|| format!("writing {}", path.display()))
}

/// Write a manufacturer for every make of `makes` that the install has a logo for. A failure costs the logos only, the cars
/// are already converted. Returns how many manufacturers were written.
pub fn write_all(source: &Source, folder: &ModFolder, makes: &[&str]) -> usize {
    let found = makers(source);
    if found.is_empty() {
        log::warn!("the 2012 install has no manufacturer logos that can be read: the brands have none");
        return 0;
    }
    let brands = match brands_for(makes.iter().copied(), &found) {
        Ok(brands) => brands,
        Err(error) => {
            log::warn!("the brand logos are not written: {error:#}");
            return 0;
        }
    };
    let mut written = 0;
    for brand in &brands {
        match write(folder, brand) {
            Ok(()) => written += 1,
            Err(error) => log::warn!("the logo of {} is not written: {error:#}", brand.name),
        }
    }
    written
}

#[cfg(test)]
mod tests {
    use super::*;

    fn maker(name: &str, shade: u8) -> Maker {
        let mut canvas = vec![0u8; logo::CANVAS.0 * logo::CANVAS.1 * 4];
        canvas[..4].copy_from_slice(&[shade, shade, shade, 255]);
        Maker { name: name.to_owned(), canvas }
    }

    #[test]
    fn brands_match_by_name_ignoring_case_spaces_and_punctuation() {
        assert!(same_brand("Aston Martin", "ASTON MARTIN"));
        assert!(same_brand("AstonMartin", "ASTON MARTIN"));
        assert!(same_brand("Land Rover", "LAND ROVER"));
        assert!(same_brand("Mercedes", "MERCEDES-BENZ"));
        assert!(same_brand("Alfa Romeo", "alfa romeo"));
        assert!(!same_brand("Mini", "MITSUBISHI"));
        assert!(!same_brand("BMW", "BMWX"), "a short name only matches itself");
        assert!(!same_brand("", "BMW"));
        assert!(!same_brand("Lotus", "LAMBORGHINI"));
    }

    #[test]
    fn ids_are_lower_case_words_joined_by_underscores() {
        assert_eq!(brand_id("Aston Martin"), "aston_martin");
        assert_eq!(brand_id("Mercedes-Benz"), "mercedes_benz");
        assert_eq!(brand_id("  BAC "), "bac");
        assert_eq!(brand_id("---"), "");
    }

    #[test]
    fn the_manifest_is_valid_toml_the_game_reads() {
        let text = manifest("Aston \"Martin\"", "UI/MAKERS/ASTON_MARTIN.PNG");
        let parsed = blackbox_modding::content::parse_manufacturer("aston_martin", &text).unwrap();
        assert_eq!(parsed.name, "Aston \"Martin\"");
        assert_eq!(parsed.logo, "UI/MAKERS/ASTON_MARTIN.PNG");
        assert!(!parsed.overriding);
    }

    #[test]
    fn a_brand_is_made_once_for_each_make_with_a_logo_and_none_for_the_rest() {
        let makers = [maker("BENTLEY", 10), maker("MERCEDES-BENZ", 20), maker("BAC", 30)];
        let makes = ["Bentley", "Lotus", "Mercedes", "bentley", "", "BAC", "Mercedes"];
        let brands = brands_for(makes, &makers).unwrap();
        let ids: Vec<&str> = brands.iter().map(|brand| brand.id.as_str()).collect();
        assert_eq!(ids, ["bentley", "mercedes", "bac"]);
        assert_eq!(brands[1].name, "Mercedes", "the name is the one the cars carry, which is what the game matches");
        assert!(brands.iter().all(|brand| brand.png.starts_with(b"\x89PNG")));
    }

    #[test]
    fn a_name_the_game_would_refuse_is_left_out() {
        let makers = [maker("LONG", 1)];
        let long = "L".repeat(33);
        assert!(brands_for([long.as_str(), "Lo\nng"], &makers).unwrap().is_empty());
    }

    #[test]
    fn brands_are_written_where_the_manifest_says_and_the_game_reads_them() {
        let dir = std::env::temp_dir().join(format!("nfsmw-import2012-brands-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let folder = ModFolder::create(&dir).unwrap();
        let brand = brands_for(["Aston Martin"], &[maker("ASTON MARTIN", 5)]).unwrap().remove(0);
        write(&folder, &brand).unwrap();
        let text = std::fs::read_to_string(dir.join("manufacturers/aston_martin.toml")).unwrap();
        let parsed = blackbox_modding::content::parse_manufacturer("aston_martin", &text).unwrap();
        assert_eq!(std::fs::read(dir.join("files").join(&parsed.logo)).unwrap(), brand.png);
        ModFolder::create(&dir).unwrap();
        assert!(!dir.join("manufacturers").exists() && !dir.join("files").exists(), "a new run clears them");
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
