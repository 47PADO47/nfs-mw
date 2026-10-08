//! What the HUD reads from the install: the package, the fonts, the textures and the language strings.

use std::collections::HashMap;

use anyhow::{Result, bail};
use blackbox_chunk::find_all;
use blackbox_feng::{Font, Package, fe_hash_upper};
use blackbox_text::StringTable;
use blackbox_tpk::{Texture, decode_rgba8, read_texture_packs};
use game_install::GameDir;
use nfsmw_data::read_unwrapped;

const FENG_PACKAGE: u32 = 0x0003_0203;
const FENG_COMPRESSED: u32 = 0x0003_0210;
const FENG_FONT: u32 = 0x0003_0201;

/// The package that is the in-game HUD.
pub const HUD_PACKAGE: &str = "HUD_SingleRace.fng";

/// Texture packs the HUD draws from, first match wins.
const TEXTURE_FILES: [&str; 7] = [
    "GLOBAL/HUDTEXRACE.BIN",
    "GLOBAL/GLOBALA.BUN",
    "GLOBAL/InGameA.bun",
    "GLOBAL/GlobalB.lzc",
    "GLOBAL/HUDS_Custom_00.bin",
    "LANGUAGES/LanguageTextures.bin",
    "GLOBAL/INGAMEC.BUN",
];

/// Files with fonts.
const FONT_FILES: [&str; 3] = ["GLOBAL/GLOBALA.BUN", "GLOBAL/InGameB.bun", "FRONTEND/FrontB.lzc"];

/// Textures the package names but the game swaps in at run time: `(resource name, texture in the packs)`.
const ALIASES: [(&str, &str); 6] = [
    ("TAC_FILL_00", "TACH_FILL_00"),
    ("RPM_NEEDLE", "TACH_NEEDLE_00"),
    ("TURBO_NEEDLE", "TURBO_NEEDLE_00"),
    ("TURBO_LINES", "TURBO_LINES_00"),
    ("3RDPERSON_8000LINES", "8000_LINES_00"),
    ("7500_LINES_00", "7000_LINES_00"),
];

/// A font and where its texture is.
pub struct FontAsset {
    pub font: Font,
}

/// A decoded texture, straight (not premultiplied) RGBA.
pub struct Image {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
    /// 1 = alpha blend, 2 = additive.
    pub blend: u8,
}

pub struct HudAssets {
    pub package: Package,
    fonts: HashMap<u32, FontAsset>,
    textures: HashMap<u32, Texture>,
    aliases: HashMap<u32, u32>,
    pub strings: Option<StringTable>,
}

impl HudAssets {
    pub fn load(dir: &GameDir) -> Result<Self> {
        let package = find_hud_package(dir)?;

        let mut fonts = HashMap::new();
        for rel in FONT_FILES {
            let Ok(data) = read_unwrapped(dir, rel) else { continue };
            for chunk in find_all(&data, FENG_FONT) {
                match Font::parse(chunk.payload) {
                    Ok(font) => {
                        fonts.entry(font.hash).or_insert(FontAsset { font });
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
        let assets = Self { package, fonts, textures, aliases, strings };
        let missing: Vec<String> = assets.missing_resources().into_iter().map(String::from).collect();
        log::info!(
            "HUD: {} objects, {} fonts, {} textures, {} resources without a texture",
            assets.package.objects.len(),
            assets.fonts.len(),
            assets.textures.len(),
            missing.len()
        );
        log::debug!("HUD resources without a texture or font: {missing:?}");
        Ok(assets)
    }

    /// Names of the package resources with no texture or font (they draw nothing).
    pub fn missing_resources(&self) -> Vec<&str> {
        let basepoly = fe_hash_upper("BASEPOLY");
        use blackbox_feng::package::ResourceKind;
        self.package
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
        self.fonts.get(&hash).map(|f| &f.font)
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

fn find_hud_package(dir: &GameDir) -> Result<Package> {
    for rel in ["GLOBAL/InGameB.bun", "GLOBAL/INGAMEC.BUN"] {
        let data = read_unwrapped(dir, rel)?;
        for chunk in find_all(&data, FENG_PACKAGE) {
            if let Ok(p) = Package::parse(chunk.payload)
                && p.name.eq_ignore_ascii_case(HUD_PACKAGE)
            {
                return Ok(p);
            }
        }
        for chunk in find_all(&data, FENG_COMPRESSED) {
            if let Ok((_, p)) = Package::parse_compressed(chunk.payload)
                && p.name.eq_ignore_ascii_case(HUD_PACKAGE)
            {
                return Ok(p);
            }
        }
    }
    bail!("the install has no {HUD_PACKAGE}")
}

/// The tachometer face texture for a scale ending at `max_rpm`: `7000_LINES_NN` … `10000_LINES_NN`.
pub fn tach_face_texture(max_rpm: f32, skin: u8) -> u32 {
    let n = ((max_rpm / 1000.0).ceil() as i32 * 1000).clamp(7000, 10000);
    fe_hash_upper(&format!("{n}_LINES_{skin:02}"))
}

/// The needle texture of a skin.
pub fn needle_texture(skin: u8) -> u32 {
    fe_hash_upper(&format!("TACH_NEEDLE_{skin:02}"))
}

/// The tachometer fill texture of a skin.
pub fn fill_texture(skin: u8) -> u32 {
    fe_hash_upper(&format!("TACH_FILL_{skin:02}"))
}
