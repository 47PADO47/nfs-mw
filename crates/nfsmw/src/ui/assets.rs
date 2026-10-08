//! What the FEng screens read from the install besides their packages: the fonts, the textures and the language
//! strings. Shared by the HUD and the menus.

use std::collections::HashMap;

use anyhow::Result;
use blackbox_chunk::find_all;
use blackbox_feng::package::ResourceKind;
use blackbox_feng::{Font, Package, fe_hash_upper};
use blackbox_text::StringTable;
use blackbox_tpk::{Texture, decode_rgba8, read_texture_packs};
use game_install::GameDir;
use nfsmw_data::read_unwrapped;

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

/// A decoded texture, straight (not premultiplied) RGBA.
pub struct Image {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
    /// 1 = alpha blend, 2 = additive.
    pub blend: u8,
}

pub struct UiAssets {
    fonts: HashMap<u32, Font>,
    textures: HashMap<u32, Texture>,
    aliases: HashMap<u32, u32>,
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

        let mut textures: HashMap<u32, Texture> = HashMap::new();
        for rel in TEXTURE_FILES {
            let Ok(data) = read_unwrapped(dir, rel) else { continue };
            match read_texture_packs(&data) {
                Ok(packs) => {
                    for t in packs.into_iter().flat_map(|p| p.textures) {
                        textures.entry(t.name_hash).or_insert(t);
                    }
                }
                Err(e) => log::warn!("{rel}: texture packs do not read: {e}"),
            }
        }

        let strings = read_unwrapped(dir, "LANGUAGES/English.bin")
            .ok()
            .and_then(|d| StringTable::from_file(&d).map_err(|e| log::warn!("English.bin: {e}")).ok());

        let aliases = ALIASES.iter().map(|(a, b)| (fe_hash_upper(a), fe_hash_upper(b))).collect();
        let assets = Self { fonts, textures, aliases, strings };
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
        if hash == fe_hash_upper("BASEPOLY") {
            return Some(Image { width: 2, height: 2, rgba: vec![255; 16], blend: 1 });
        }
        let t = self.texture(hash)?;
        let rgba = decode_rgba8(t)?;
        Some(Image { width: t.width, height: t.height, rgba, blend: t.alpha_blend })
    }

    /// Size of a texture in pixels.
    pub fn texture_size(&self, hash: u32) -> Option<(u32, u32)> {
        self.texture(hash).map(|t| (t.width, t.height)).or((hash == fe_hash_upper("BASEPOLY")).then_some((2, 2)))
    }
}
