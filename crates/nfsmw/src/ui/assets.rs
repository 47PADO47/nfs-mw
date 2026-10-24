//! What the FEng screens read from the install besides their packages: the fonts and the textures. Shared by the HUD and
//! the menus. The language's text is `crate::i18n`; the language's own fonts are here ([`UiAssets::font`]).

mod load;
pub(crate) use load::read_wanted;

use std::collections::{HashMap, HashSet};

use anyhow::{Context, Result};
use blackbox_chunk::find_all;
use blackbox_feng::font::uv_size;
use blackbox_feng::package::ResourceKind;
use blackbox_feng::{Font, Package, fe_hash_upper};
use blackbox_tpk::{Texture, decode_rgba8};
use game_install::GameDir;
use nfsmw_career::regions::MapVariant;
use nfsmw_data::languages;
use nfsmw_data::shared_unwrapped;

use super::fonts::CustomFontSet;
use super::fonts::tier::Tier;
use super::late_textures::{LateTextures, TextureRef};
use crate::settings::custom_fonts;
use crate::settings::hd_content;

/// The result of a loader thread; a panic in it goes on in the thread that joins it.
fn joined<T>(handle: std::thread::ScopedJoinHandle<'_, T>) -> T {
    handle.join().unwrap_or_else(|panic| std::panic::resume_unwind(panic))
}

/// The minimap and world map variants whose textures are loaded up front (the career stage picks one at run time).
const MAP_VARIANTS: [MapVariant; 3] = [MapVariant::Full, MapVariant::Unlock1, MapVariant::Unlock2];

const FENG_FONT: u32 = 0x0003_0201;

/// Texture packs the screens draw from, first match wins.
const TEXTURE_FILES: [&str; 11] = [
    "GLOBAL/HUDTEXRACE.BIN",
    // The split-screen HUDs (`hud/split`): the race pair and the drag pair. The pack names of the decomp are swapped; the
    // contents settle it (`docs/specs/split-screen-hud.md` section 2).
    "GLOBAL/HUDTEXSPLIT.BIN",
    "GLOBAL/HUDTEXDRAGSPLIT.BIN",
    "GLOBAL/GLOBALA.BUN",
    "GLOBAL/InGameA.bun",
    "GLOBAL/GlobalB.lzc",
    "FRONTEND/FrontB.lzc",
    "GLOBAL/HUDS_Custom_00.bin",
    "LANGUAGES/LanguageTextures.bin",
    "GLOBAL/INGAMEC.BUN",
    // The backdrops of the win photo (`frontend/photo.rs`): four 1024 x 512 pictures, kept compressed until drawn.
    "GLOBAL/HUDTEXTURESPHOTOFINISH.BIN",
];

/// The rivals' pictures (`RIVAL_NN`, `RIVAL_NN_BG`, `RIVAL_NN_GRAF`; the pack also holds in-game `_IG` variants, which
/// the menus do not use). They are not in [`TEXTURE_FILES`]: the pack has its own file, and the career screens name its
/// textures by rival (`career-regions.md` section 8).
const RIVALS_FILE: &str = "GLOBAL/RIVALS.BIN";

/// The pack with the pictures of the loading tips (`ui::tips`).
const TIPS_FILE: &str = "GLOBAL/DYNTEX.BIN";

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
/// 2.2. It is the first source of a font's native size (see [`native_size`]). The rows for `font_mw_body`,
/// `font_mw_title` and `font_mw_custom_numbers` agree with the unmodified install.
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

/// A font texture's native size: the table's when it names the font (the stock sizes, checked against the game), else
/// the install's stock copy. `None` when neither knows it.
///
/// The table comes first because a mod can replace the stock texture inside the install itself (an HD `FONT_MW_BODY`
/// of 2048 x 2048 sits in the modded `GLOBALA.BUN` where the stock one is 256 x 256). The glyph layout is in stock
/// pixels, so an install copy read here would take the HD size and every glyph would sample the wrong place.
pub fn native_size(stock: Option<(u32, u32)>, table: Option<(u32, u32)>) -> Option<(u32, u32)> {
    table.or(stock)
}

/// The native size of each font's texture (see [`native_size`]). The table is consulted only when the texture is
/// loaded: a loaded texture that is not a stock copy is a replacement, and its stock size is what the table gives.
/// The language tables carry the same fonts again, so each texture is sized (and reported) once.
fn native_font_sizes<'a>(
    fonts: impl Iterator<Item = &'a Font>,
    stock: &HashMap<u32, (u32, u32)>,
    textures: &HashMap<u32, Texture>,
) -> HashMap<u32, (u32, u32)> {
    let mut sizes = HashMap::new();
    for font in fonts {
        if sizes.contains_key(&font.texture_hash) {
            continue;
        }
        let table = textures.contains_key(&font.texture_hash).then(|| table_size(font.hash)).flatten();
        let stock = stock.get(&font.texture_hash).copied();
        if let (Some(t), Some(s)) = (table, stock)
            && t != s
        {
            // An HD font mod: the supported case, laid out on the stock size under the HD fonts option.
            log::debug!("font {:#x}: the install's texture is {s:?}, the stock size is {t:?}; using {t:?}", font.hash);
        }
        let Some(size) = native_size(stock, table) else { continue };
        sizes.insert(font.texture_hash, size);
    }
    sizes
}

/// The `FEngFont` chunks of every language table of the install, by the table's file in lower case.
fn language_fonts(dir: &GameDir) -> HashMap<String, HashMap<u32, Font>> {
    languages::installed(dir)
        .into_iter()
        .filter_map(|table| {
            let data = languages::bytes(dir, &table.file).ok()?;
            let mut fonts = HashMap::new();
            for chunk in find_all(&data, FENG_FONT) {
                match Font::parse(chunk.payload) {
                    Ok(font) => {
                        fonts.entry(font.hash).or_insert(font);
                    }
                    Err(e) => log::warn!("{}: a font does not parse: {e}", table.file),
                }
            }
            Some((table.file.to_ascii_lowercase(), fonts))
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
    /// The user's custom fonts (`ui/fonts`). They win over the stock fonts and textures of the same name while the
    /// custom fonts are on (see [`UiAssets::font`]).
    custom: CustomFontSet,
    /// Textures added after the load and dropped again (the minimap tiles of a race; see [`UiAssets::add_textures`]).
    late: LateTextures,
    input_atlas: Image,
    discord_mark: Image,
    mod_mark: Image,
    /// The fonts each language table carries, by its file in lower case (`docs/specs/localization.md` section 5).
    language_fonts: HashMap<String, HashMap<u32, Font>>,
}

impl UiAssets {
    /// The stock assets alone, with no custom fonts, and English as the current language so the screens read the game's
    /// labels (the tests' fixture; the app uses [`UiAssets::load_with`] after `crate::i18n::start`).
    #[cfg(test)]
    pub fn load(dir: &GameDir) -> Result<Self> {
        crate::i18n::start(dir, "en".parse().ok());
        Self::load_with(dir, CustomFontSet::default())
    }

    /// Assets with nothing in them (the tests' fixture for code that only adds and looks up textures).
    #[cfg(test)]
    pub fn empty() -> Self {
        let blank = || Image { width: 0, height: 0, rgba: Vec::new(), blend: 1 };
        Self {
            fonts: HashMap::new(),
            textures: HashMap::new(),
            aliases: HashMap::new(),
            font_native: HashMap::new(),
            custom: CustomFontSet::default(),
            late: LateTextures::default(),
            input_atlas: blank(),
            discord_mark: blank(),
            mod_mark: blank(),
            language_fonts: HashMap::new(),
        }
    }

    /// Loads the stock assets and keeps `custom` as the user's font set.
    pub fn load_with(dir: &GameDir, custom: CustomFontSet) -> Result<Self> {
        let mut fonts = HashMap::new();
        for rel in FONT_FILES.map(|rel| nfsmw_data::shared::pack_file(dir, rel)) {
            let Ok(data) = shared_unwrapped(dir, rel) else { continue };
            for chunk in find_all(&data, FENG_FONT) {
                match Font::parse(chunk.payload) {
                    Ok(font) => {
                        fonts.entry(font.hash).or_insert(font);
                    }
                    Err(e) => log::warn!("{rel}: a font does not parse: {e}"),
                }
            }
        }

        let language_fonts = language_fonts(dir);
        // The stock sizes of the font textures come from the install's own packs, not from an overlay (which may hold an
        // HD copy): only a pack the install serves counts. The first such pack wins, as in the loaded textures below.
        let all_fonts = || fonts.values().chain(language_fonts.values().flat_map(HashMap::values));
        let font_textures: HashSet<u32> = all_fonts().map(|f| f.texture_hash).collect();
        // The groups of textures are independent reads, so they run side by side; they are merged in the order below,
        // which is their priority (first name wins).
        let (mut textures, stock, rivals_and_tips, minimap_tiles, world_maps) = std::thread::scope(|scope| {
            let rivals_and_tips = scope.spawn(|| load::rivals_and_tips(dir));
            let minimap_tiles = scope.spawn(|| load::minimap_tiles(dir));
            let world_maps = scope.spawn(|| load::world_maps(dir));
            let (textures, stock) = load::texture_files(dir, &font_textures);
            (textures, stock, joined(rivals_and_tips), joined(minimap_tiles), joined(world_maps))
        });
        load::merge(&mut textures, rivals_and_tips);
        load::merge(&mut textures, minimap_tiles);
        load::merge(&mut textures, world_maps);

        let font_native = native_font_sizes(all_fonts(), &stock, &textures);
        let aliases = ALIASES.iter().map(|(a, b)| (fe_hash_upper(a), fe_hash_upper(b))).collect();
        let input_atlas = super::input_icons::load()?;
        let discord_mark = super::discord_icon::load()?;
        let mod_mark = super::mod_icon::load()?;
        let assets = Self {
            fonts,
            textures,
            aliases,
            font_native,
            custom,
            late: LateTextures::default(),
            language_fonts,
            input_atlas,
            discord_mark,
            mod_mark,
        };
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
                ResourceKind::Font => self.font(r.handle).is_none(),
                _ => r.handle != basepoly && self.texture(r.handle).is_none(),
            })
            .map(|r| r.name.as_str())
            .collect()
    }

    /// The font for a name hash: the custom font for the current tier when the custom fonts are on and a file supplies
    /// one, else the current language's own font, else English's, else the bundles' stock font.
    pub fn font(&self, hash: u32) -> Option<&Font> {
        self.custom_font(hash)
            .map(|(_, font)| font)
            .or_else(|| self.language_font(hash))
            .or_else(|| self.fonts.get(&hash))
    }

    /// The font of the current language's table, else of `English.bin`.
    fn language_font(&self, hash: u32) -> Option<&Font> {
        let current = crate::i18n::current().table.as_ref().map(|table| table.file.to_ascii_lowercase());
        let english = languages::ENGLISH_FILE.to_ascii_lowercase();
        current
            .iter()
            .chain(std::iter::once(&english))
            .find_map(|file| self.language_fonts.get(file).and_then(|fonts| fonts.get(&hash)))
    }

    /// The custom font for a name hash in use now, with its tier. `None` when the custom fonts are off or no file has it.
    fn custom_font(&self, hash: u32) -> Option<(Tier, &Font)> {
        let active = custom_fonts::active();
        if !active.enabled {
            return None;
        }
        self.custom.font(hash, active.tier)
    }

    /// The scale a font is drawn at: the custom font's tier scale when a custom font is in use, else 1.0.
    pub fn font_scale(&self, hash: u32) -> f32 {
        match self.custom_font(hash) {
            Some((tier, _)) => custom_fonts::active().scale_of(tier),
            None => 1.0,
        }
    }

    /// The raw texture for a key, following the custom fonts, the run-time swaps and the late textures.
    pub fn texture(&self, hash: u32) -> Option<TextureRef<'_>> {
        if let Some(texture) = self.custom_texture(hash).or_else(|| self.textures.get(&hash)) {
            return Some(TextureRef::Stock(texture));
        }
        if let Some(texture) = self.late.get(hash) {
            return Some(TextureRef::Late(texture));
        }
        self.aliases.get(&hash).and_then(|alias| self.textures.get(alias)).map(TextureRef::Stock)
    }

    /// Adds textures to the assets for the screens to draw, from any thread. They are looked up after the stock
    /// textures, so a stock texture of the same name wins. Used for the per-event minimap tiles.
    pub fn add_textures(&self, textures: impl IntoIterator<Item = Texture>) {
        self.late.add(textures);
    }

    /// Drops textures added with [`UiAssets::add_textures`] (the stock ones are not touched).
    pub fn remove_textures(&self, hashes: &[u32]) {
        self.late.remove(hashes);
    }

    /// How many added textures are held now.
    #[cfg(test)]
    pub fn late_count(&self) -> usize {
        self.late.len()
    }

    /// The custom texture for a key in use now (a custom font's own texture), if any.
    fn custom_texture(&self, hash: u32) -> Option<&Texture> {
        let active = custom_fonts::active();
        if !active.enabled {
            return None;
        }
        self.custom.texture(hash, active.tier)
    }

    /// The texture key that finally supplies a key (after aliases).
    pub fn resolve(&self, hash: u32) -> u32 {
        let known =
            self.textures.contains_key(&hash) || self.late.contains(hash) || self.custom_texture(hash).is_some();
        if known { hash } else { self.aliases.get(&hash).copied().unwrap_or(hash) }
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
        if hash == super::mod_icon::TEXTURE {
            return Some(Image {
                width: self.mod_mark.width,
                height: self.mod_mark.height,
                rgba: self.mod_mark.rgba.clone(),
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
        let rgba = decode_rgba8(&t)?;
        Some(Image { width: t.width, height: t.height, rgba, blend: t.alpha_blend })
    }

    /// The size a font texture's glyph coordinates are divided by (the layout's texture size): its native (stock) size
    /// under the HD fonts option, else the loaded texture's size. The font layouts are its only users.
    pub fn texture_size(&self, hash: u32) -> Option<(u32, u32)> {
        let loaded = self.texture(hash).map(|t| (t.width, t.height));
        let loaded = loaded.or((hash == fe_hash_upper("BASEPOLY")).then_some((2, 2)))?;
        // A custom font's texture is the font's own: its glyphs are laid out on its own size, not the stock one.
        let native = match self.custom_texture(hash) {
            Some(_) => None,
            None => self.font_native.get(&hash).copied(),
        };
        Some(uv_size(hd_content::active().fonts, native, loaded))
    }
}

#[cfg(test)]
#[path = "assets_tests.rs"]
mod tests;
