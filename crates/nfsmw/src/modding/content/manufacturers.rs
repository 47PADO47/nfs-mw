//! The manufacturers of the mods: each `manufacturers/<id>.toml` is read with the logo it names, and the brands are
//! published for the car screens (`frontend/car_title.rs`). A manufacturer that fails any check is logged and left out.

use std::path::Path;
use std::sync::{PoisonError, RwLock};

use blackbox_modding::content::{MANUFACTURERS_FOLDER, Manufacturer, brand_key, parse_manufacturer};

use super::ModDir;

/// The largest logo picture a mod may give, in pixels on a side: a logo is drawn at about 150 by 75.
const MAX_SIDE: u32 = 2048;

/// A published manufacturer: a brand and its decoded logo.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Maker {
    /// The id of the mod that declared it.
    pub owner: String,
    pub name: String,
    /// The brand reduced for comparison (`blackbox_modding::content::brand_key`).
    pub key: String,
    /// Replaces the logo of an existing manufacturer of the name.
    pub overriding: bool,
    pub width: u32,
    pub height: u32,
    /// The logo as straight RGBA, 8 bits a channel.
    pub rgba: Vec<u8>,
}

static MAKERS: RwLock<&'static [Maker]> = RwLock::new(&[]);

/// The manufacturers of this run so far: empty before [`register`], or when no mod adds one.
pub(crate) fn published() -> &'static [Maker] {
    *MAKERS.read().unwrap_or_else(PoisonError::into_inner)
}

/// The manufacturer of `brand` (compared as [`brand_key`] does) whose `overriding` is as asked.
pub(crate) fn find(brand: &str, overriding: bool) -> Option<&'static Maker> {
    let key = brand_key(brand);
    published().iter().find(|maker| maker.overriding == overriding && maker.key == key)
}

/// Read the manufacturers the mods declare and publish them: at start-up for every loaded mod, and again for one mod that
/// added some later. A brand already published with the same override setting is skipped, so a repeated call adds only what
/// is new, and the first mod to declare a brand keeps it. Returns how many were added.
pub(super) fn register(mods: &[ModDir]) -> usize {
    let mut all: Vec<Maker> = published().to_vec();
    let before = all.len();
    for maker in mods.iter().flat_map(read_mod) {
        if all.iter().any(|known| known.key == maker.key && known.overriding == maker.overriding) {
            continue;
        }
        log::info!("mod {}: manufacturer {} added", maker.owner, maker.name);
        all.push(maker);
    }
    let added = all.len() - before;
    if added > 0 {
        *MAKERS.write().unwrap_or_else(PoisonError::into_inner) = Box::leak(all.into_boxed_slice());
    }
    added
}

/// The manufacturers one mod declares.
fn read_mod(m: &ModDir) -> Vec<Maker> {
    let Ok(entries) = std::fs::read_dir(m.dir.join(MANUFACTURERS_FOLDER)) else { return Vec::new() };
    let mut files: Vec<_> = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext.eq_ignore_ascii_case("toml")))
        .collect();
    files.sort();
    files.iter().filter_map(|path| read_one(m, path)).collect()
}

fn read_one(m: &ModDir, path: &Path) -> Option<Maker> {
    let stem = path.file_stem()?.to_str()?;
    let refuse = |why: &dyn std::fmt::Display| {
        log::warn!("mod {}: manufacturer {stem} is not added: {why}", m.id);
        None
    };
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) => return refuse(&error),
    };
    let manifest: Manufacturer = match parse_manufacturer(stem, &text) {
        Ok(manifest) => manifest,
        Err(error) => return refuse(&error),
    };
    let logo = m.dir.join(crate::modding::layers::FILES_FOLDER).join(&manifest.logo);
    let bytes = match std::fs::read(&logo) {
        Ok(bytes) => bytes,
        Err(error) => return refuse(&format_args!("the logo {} cannot be read: {error}", manifest.logo)),
    };
    let Some((width, height, rgba)) = decode_logo(&bytes) else {
        return refuse(&format_args!("the logo {} is not a PNG the game reads", manifest.logo));
    };
    Some(Maker {
        owner: m.id.clone(),
        key: brand_key(&manifest.name),
        name: manifest.name,
        overriding: manifest.overriding,
        width,
        height,
        rgba,
    })
}

/// A PNG as straight RGBA, of any colour type and bit depth, up to [`MAX_SIDE`] on a side.
fn decode_logo(bytes: &[u8]) -> Option<(u32, u32, Vec<u8>)> {
    let mut decoder = png::Decoder::new(std::io::Cursor::new(bytes));
    decoder.set_transformations(png::Transformations::normalize_to_color8());
    let mut reader = decoder.read_info().ok()?;
    let mut data = vec![0; reader.output_buffer_size()?];
    let frame = reader.next_frame(&mut data).ok()?;
    let (width, height) = (frame.width, frame.height);
    if width == 0 || height == 0 || width > MAX_SIDE || height > MAX_SIDE {
        return None;
    }
    let data = &data[..frame.buffer_size()];
    let rgba = match frame.color_type {
        png::ColorType::Rgba => data.to_vec(),
        png::ColorType::Rgb => data.chunks(3).flat_map(|p| [p[0], p[1], p[2], 255]).collect(),
        png::ColorType::GrayscaleAlpha => data.chunks(2).flat_map(|p| [p[0], p[0], p[0], p[1]]).collect(),
        png::ColorType::Grayscale => data.iter().flat_map(|&g| [g, g, g, 255]).collect(),
        png::ColorType::Indexed => return None,
    };
    Some((width, height, rgba))
}

#[cfg(test)]
#[path = "manufacturers_tests.rs"]
mod tests;
