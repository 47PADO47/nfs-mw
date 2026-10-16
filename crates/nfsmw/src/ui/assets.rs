//! What the FEng screens read from the install besides their packages: the fonts, the textures and the language
//! strings. Shared by the HUD and the menus.

use std::collections::{HashMap, HashSet};

use anyhow::{Context, Result};
use blackbox_chunk::find_all;
use blackbox_feng::font::uv_size;
use blackbox_feng::package::ResourceKind;
use blackbox_feng::{Font, Package, fe_hash_upper};
use blackbox_text::StringTable;
use blackbox_tpk::{Texture, decode_rgba8, read_texture_packs};
use game_install::{GameDir, Provider};
use nfsmw_data::read_unwrapped;

use crate::settings::hd_content;

const FENG_FONT: u32 = 0x0003_0201;

/// Texture packs the screens draw from, first match wins.
const TEXTURE_FILES: [&str; 8] = [
    "GLOBAL/HUDTEXRACE.BIN",
    "GLOBAL/GLOBALA.BUN",
    "GLOBAL/InGameA.bun",
    "GLOBAL/GlobalB.lzc",
    "FRONTEND/FrontB.lzc",
    "GLOBAL/HUDS_Custom_00.bin",
    "LANGUAGES/LanguageTextures.bin",
    "GLOBAL/INGAMEC.BUN",
];

/// Files with fonts.
const FONT_FILES: [&str; 3] = ["GLOBAL/GLOBALA.BUN", "GLOBAL/InGameB.bun", "FRONTEND/FrontB.lzc"];

/// Textures the packages name but the game swaps in at run time: `(resource name, texture in the packs)`.
const ALIASES: [(&str, &str); 6] = [
    ("TAC_FILL_00", "TACH_FILL_00"),
    ("RPM_NEEDLE", "TACH_NEEDLE_00"),
    ("TURBO_NEEDLE", "TURBO_NEEDLE_00"),
    ("TURBO_LINES", "TURBO_LINES_00"),
    ("3RDPERSON_8000LINES", "8000_LINES_00"),
    ("7500_LINES_00", "7000_LINES_00"),
];

/// The stock sizes of the font textures, by font name: the built-in table of `docs/specs/hd-content-support.md`, section
/// 2.2. Used only for a font whose texture is loaded but has no stock copy in the install's packs (so a replacement).
/// The rows for `font_mw_body`, `font_mw_title` and `font_mw_custom_numbers` agree with the install.
const FONT_TABLE: [(&str, (u32, u32)); 6] = [
    ("font_mw_body", (256, 256)),
    ("font_mw_title", (512, 512)),
    ("font_mw_custom_numbers", (128, 64)),
    ("tac_numbers", (128, 64)),
    ("arial", (256, 256)),
    ("arial12", (256, 256)),
];

/// The table's size for a font, by its name hash.
fn table_size(font_hash: u32) -> Option<(u32, u32)> {
    FONT_TABLE.iter().find(|(name, _)| fe_hash_upper(name) == font_hash).map(|(_, size)| *size)
}

/// A font texture's native size: the stock texture's size when the install has one, else the table's. `None` when
/// neither knows it.
pub fn native_size(stock: Option<(u32, u32)>, table: Option<(u32, u32)>) -> Option<(u32, u32)> {
    stock.or(table)
}

/// The native size of each font's texture (see [`native_size`]). The table is consulted only when the texture is
/// loaded: a loaded texture that is not a stock copy is a replacement, and its stock size is what the table gives.
fn native_font_sizes(
    fonts: &HashMap<u32, Font>,
    stock: &HashMap<u32, (u32, u32)>,
    textures: &HashMap<u32, Texture>,
) -> HashMap<u32, (u32, u32)> {
    fonts
        .values()
        .filter_map(|font| {
            let table = textures.contains_key(&font.texture_hash).then(|| table_size(font.hash)).flatten();
            let size = native_size(stock.get(&font.texture_hash).copied(), table)?;
            Some((font.texture_hash, size))
        })
        .collect()
}

/// A decoded texture, straight (not premultiplied) RGBA.
pub struct Image {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
    /// 1 = alpha blend, 2 = additive.
    pub blend: u8,
}

/// Decodes a bundled RGBA PNG (embedded in the executable, so it is trusted to be the shipped file).
pub fn decode_png_rgba(data: &[u8]) -> Result<Image> {
    let mut reader = png::Decoder::new(std::io::Cursor::new(data)).read_info()?;
    let mut rgba = vec![0; reader.output_buffer_size().context("bundled PNG dimensions")?];
    let frame = reader.next_frame(&mut rgba)?;
    anyhow::ensure!(frame.color_type == png::ColorType::Rgba, "bundled PNGs must be RGBA");
    rgba.truncate(frame.buffer_size());
    Ok(Image { width: frame.width, height: frame.height, rgba, blend: 1 })
}

pub struct UiAssets {
    fonts: HashMap<u32, Font>,
    textures: HashMap<u32, Texture>,
    aliases: HashMap<u32, u32>,
    /// The stock size of each font texture, by texture hash, where known (see [`native_size`]).
    font_native: HashMap<u32, (u32, u32)>,
    input_atlas: Image,
    discord_mark: Image,
    pub strings: Option<StringTable>,
}

impl UiAssets {
    pub fn load(dir: &GameDir) -> Result<Self> {
        let mut fonts = HashMap::new();
        for rel in FONT_FILES {
            let Ok(data) = read_unwrapped(dir, rel) else { continue };
            for chunk in find_all(&data, FENG_FONT) {
                match Font::parse(chunk.payload) {
                    Ok(font) => {
                        fonts.entry(font.hash).or_insert(font);
                    }
                    Err(e) => log::warn!("{rel}: a font does not parse: {e}"),
                }
            }
        }

        // The stock sizes of the font textures come from the install's own packs, not from an overlay (which may hold an
        // HD copy): only a pack the install serves counts. The first such pack wins, as in the loaded textures below.
        let font_textures: HashSet<u32> = fonts.values().map(|f| f.texture_hash).collect();
        let mut stock: HashMap<u32, (u32, u32)> = HashMap::new();
        let mut textures: HashMap<u32, Texture> = HashMap::new();
        for rel in TEXTURE_FILES {
            let Ok(data) = read_unwrapped(dir, rel) else { continue };
            let from_install = matches!(dir.provider(rel), Some(Provider::Install));
            match read_texture_packs(&data) {
                Ok(packs) => {
                    for t in packs.into_iter().flat_map(|p| p.textures) {
                        if from_install && font_textures.contains(&t.name_hash) {
                            stock.entry(t.name_hash).or_insert((t.width, t.height));
                        }
                        textures.entry(t.name_hash).or_insert(t);
                    }
                }
                Err(e) => log::warn!("{rel}: texture packs do not read: {e}"),
            }
        }

        // The minimap's tiles are textures too: the HUD swaps them into its pieces by name.
        match nfsmw_data::minimap::read_tiles(dir, nfsmw_data::minimap::FULL_MAP) {
            Ok(tiles) => {
                for t in tiles.tiles {
                    textures.entry(t.name_hash).or_insert(t);
                }
            }
            Err(e) => log::warn!("the minimap tiles do not read: {e:#}"),
        }

        let strings = read_unwrapped(dir, "LANGUAGES/English.bin")
            .ok()
            .and_then(|d| StringTable::from_file(&d).map_err(|e| log::warn!("English.bin: {e}")).ok());

        let font_native = native_font_sizes(&fonts, &stock, &textures);
        let aliases = ALIASES.iter().map(|(a, b)| (fe_hash_upper(a), fe_hash_upper(b))).collect();
        let input_atlas = super::input_icons::load()?;
        let discord_mark = super::discord_icon::load()?;
        let assets = Self { fonts, textures, aliases, font_native, strings, input_atlas, discord_mark };
        log::info!("UI: {} fonts, {} textures", assets.fonts.len(), assets.textures.len());
        Ok(assets)
    }

    /// Names of the package resources with no texture or font (they draw nothing).
    pub fn missing_resources<'a>(&self, package: &'a Package) -> Vec<&'a str> {
        let basepoly = fe_hash_upper("BASEPOLY");
        package
            .resources
            .iter()
            .filter(|r| match r.kind {
                ResourceKind::Font => !self.fonts.contains_key(&r.handle),
                _ => r.handle != basepoly && self.texture(r.handle).is_none(),
            })
            .map(|r| r.name.as_str())
            .collect()
    }

    pub fn font(&self, hash: u32) -> Option<&Font> {
        self.fonts.get(&hash)
    }

    /// The raw texture for a key, following the run-time swaps.
    pub fn texture(&self, hash: u32) -> Option<&Texture> {
        self.textures.get(&hash).or_else(|| self.aliases.get(&hash).and_then(|a| self.textures.get(a)))
    }

    /// The texture key that finally supplies a key (after aliases).
    pub fn resolve(&self, hash: u32) -> u32 {
        if self.textures.contains_key(&hash) { hash } else { self.aliases.get(&hash).copied().unwrap_or(hash) }
    }

    /// Decodes a texture to RGBA. `BASEPOLY` (a plain coloured polygon, not in any pack) is white.
    pub fn image(&self, hash: u32) -> Option<Image> {
        if hash == super::input_icons::ATLAS {
            return Some(Image {
                width: self.input_atlas.width,
                height: self.input_atlas.height,
                rgba: self.input_atlas.rgba.clone(),
                blend: 1,
            });
        }
        if hash == super::discord_icon::TEXTURE {
            return Some(Image {
                width: self.discord_mark.width,
                height: self.discord_mark.height,
                rgba: self.discord_mark.rgba.clone(),
                blend: 1,
            });
        }
        if hash == fe_hash_upper("BASEPOLY") {
            return Some(Image { width: 2, height: 2, rgba: vec![255; 16], blend: 1 });
        }
        let t = self.texture(hash)?;
        let rgba = decode_rgba8(t)?;
        Some(Image { width: t.width, height: t.height, rgba, blend: t.alpha_blend })
    }

    /// The size a font texture's glyph coordinates are divided by (the layout's texture size): its native (stock) size
    /// under the HD fonts option, else the loaded texture's size. The font layouts are its only users.
    pub fn texture_size(&self, hash: u32) -> Option<(u32, u32)> {
        let loaded = self.texture(hash).map(|t| (t.width, t.height));
        let loaded = loaded.or((hash == fe_hash_upper("BASEPOLY")).then_some((2, 2)))?;
        Some(uv_size(hd_content::active().fonts, self.font_native.get(&hash).copied(), loaded))
    }
}

#[cfg(test)]
#[path = "assets_tests.rs"]
mod tests;
