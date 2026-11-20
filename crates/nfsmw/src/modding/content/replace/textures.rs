//! Textures of a replacement set: where a replacing picture comes from, how it becomes a texture of the game's data model, and the
//! rules it follows when it takes the place of one (`docs/modding/api-replace.md`, "Textures").
//!
//! - **Resolution.** Any size. A texture is drawn over the surfaces that name it with normalised coordinates, so a picture larger
//!   or smaller than the original is stretched over the same surface; nothing is rescaled by the game unless the mod caps it
//!   (`max_size`) or the player's texture detail drops top mips, which applies to a mod's texture as to the game's.
//! - **Format.** BC1, BC2 and BC3 pass through with their mips; PNG, uncompressed DDS and the block formats the engine has no
//!   slot for are decoded to RGBA8 and stored as BC1 (no alpha), BC3 (alpha) or A8R8G8B8 (`format`); a PNG gets a full mip chain.
//! - **State.** The replacement keeps the name hash and, by default, the draw state of what it replaces (alpha mode, blend, sidedness,
//!   tiling), because the game's materials were authored for it ([`StateRule`]).
//! - **When.** A file is read and decoded **the first time a section holding the texture is loaded**, on a loader thread, and kept
//!   (one copy for every section). A file that cannot be read or decoded is logged once and the game's texture stays.

use std::sync::{Arc, OnceLock};

use anyhow::{Context, Result, bail};
use blackbox_modding::content::{TextureConvert, TextureState};
use blackbox_tpk::{AlphaUsage, Level, PixelFormat, Texture, TextureSource, full_mip_count, halve, read_dds};
use game_install::GameDir;

/// The most a picture may have on a side, the pack format's own limit.
const MAX_SIDE: u32 = blackbox_tpk::MAX_DIMENSION;

/// Where a slot's picture comes from.
enum Source {
    /// Already a texture: a texture of the set's pack, or a file that was loaded when the set was.
    Ready(Arc<Texture>),
    /// A file of the install's layers, read when first needed.
    File(String),
}

/// One texture a set replaces: the picture and the rules that make it a replacement.
pub struct TextureSlot {
    owner: String,
    name: String,
    source: Source,
    state: TextureState,
    convert: TextureConvert,
    max_size: Option<u32>,
    dir: Option<GameDir>,
    loaded: OnceLock<Option<Arc<Texture>>>,
}

impl TextureSlot {
    /// A slot whose picture is already a texture.
    pub fn ready(owner: &str, name: &str, texture: Texture, state: TextureState) -> Self {
        Self::new(owner, name, Source::Ready(Arc::new(texture)), state, TextureConvert::Auto, None, None)
    }

    /// A slot whose picture is the file `path`, read through `dir` the first time a section needs the texture.
    pub fn file(
        owner: &str,
        name: &str,
        path: &str,
        dir: GameDir,
        state: TextureState,
        convert: TextureConvert,
        max_size: Option<u32>,
    ) -> Self {
        Self::new(owner, name, Source::File(path.to_owned()), state, convert, max_size, Some(dir))
    }

    fn new(
        owner: &str,
        name: &str,
        source: Source,
        state: TextureState,
        convert: TextureConvert,
        max_size: Option<u32>,
        dir: Option<GameDir>,
    ) -> Self {
        Self {
            owner: owner.to_owned(),
            name: name.to_owned(),
            source,
            state,
            convert,
            max_size,
            dir,
            loaded: OnceLock::new(),
        }
    }

    /// The picture, loaded once. `None` (logged once) when the file cannot be read or decoded.
    fn picture(&self) -> Option<&Arc<Texture>> {
        let loaded = self.loaded.get_or_init(|| match &self.source {
            Source::Ready(texture) => Some(texture.clone()),
            Source::File(path) => {
                let loaded =
                    self.dir.as_ref().map(|dir| read_picture(dir, path, &self.name, self.convert, self.max_size));
                match loaded {
                    Some(Ok(texture)) => Some(Arc::new(texture)),
                    Some(Err(error)) => {
                        log::warn!(
                            "mod {}: the texture {} of {path} is not used, the game's stays: {error:#}",
                            self.owner,
                            self.name
                        );
                        None
                    }
                    None => None,
                }
            }
        });
        loaded.as_ref()
    }

    /// Make the picture now if it is not made (on the calling thread), so a later [`made`](Self::made) finds it.
    pub fn make(&self) {
        let _ = self.picture();
    }

    /// The picture as it is, without waiting: `None` while nothing has made it yet, `Some(None)` when it could not be made.
    pub fn made(&self) -> Option<Option<Arc<Texture>>> {
        self.loaded.get().cloned()
    }

    /// The texture that takes the place of `original`: the slot's picture with `original`'s name hash and, under
    /// [`TextureState::Keep`], its draw state. `None` when the picture cannot be had.
    pub fn replacement(&self, original: &Texture) -> Option<Texture> {
        let picture = self.picture()?;
        let mut texture = Texture::clone(picture);
        texture.name.clone_from(&original.name);
        texture.name_hash = original.name_hash;
        if self.state == TextureState::Keep {
            texture.alpha_usage = original.alpha_usage;
            texture.alpha_sorting = original.alpha_sorting;
            texture.alpha_blend = original.alpha_blend;
            texture.flags = original.flags;
            texture.tilable_uv = original.tilable_uv;
        }
        Some(texture)
    }

    /// The picture as a new texture called `name` (an added texture): its own state, name and hash.
    pub fn added(&self, name: &str) -> Option<Texture> {
        let mut texture = Texture::clone(self.picture()?);
        texture.name = name.to_owned();
        texture.name_hash = blackbox_hash::bstring_hash(name);
        Some(texture)
    }
}

/// Read and make the texture a file holds. `name` is what the pack texture of a `.bin` is looked up by.
pub fn read_picture(
    dir: &GameDir,
    path: &str,
    name: &str,
    convert: TextureConvert,
    max_size: Option<u32>,
) -> Result<Texture> {
    let bytes = dir.read(path).with_context(|| format!("reading {path}"))?;
    let extension = path.rsplit('.').next().unwrap_or("").to_ascii_lowercase();
    picture(&extension, name, &bytes, convert, max_size)
}

/// The texture a picture file of kind `extension` (`png`, `dds`, `bin`) holds.
pub fn picture(
    extension: &str,
    name: &str,
    bytes: &[u8],
    convert: TextureConvert,
    max_size: Option<u32>,
) -> Result<Texture> {
    let mut texture = match extension {
        "png" => from_png(name, bytes, convert, max_size)?,
        "dds" => read_dds(name, bytes).map_err(|error| anyhow::anyhow!("{error}"))?,
        "bin" => from_pack(name, bytes)?,
        other => bail!("a .{other} file is not a picture the game reads"),
    };
    if let Some(limit) = max_size {
        shrink(&mut texture, limit);
    }
    Ok(texture)
}

/// The texture called `name` in a texture pack file, or the only texture it holds.
fn from_pack(name: &str, bytes: &[u8]) -> Result<Texture> {
    let hash = blackbox_hash::bstring_hash(name);
    let packs = blackbox_tpk::read_texture_packs(bytes).context("the file is not a texture pack")?;
    let mut all: Vec<Texture> = packs.into_iter().flat_map(|pack| pack.textures).collect();
    let at = all.iter().position(|texture| texture.name_hash == hash).or((all.len() == 1).then_some(0));
    match at {
        Some(at) => Ok(all.swap_remove(at)),
        None => bail!("the pack has no texture called {name} (it holds {})", all.len()),
    }
}

/// A PNG of any colour type and bit depth as RGBA8, converted as `convert` says (`Auto`: BC1, BC3, or A8R8G8B8 when a side is not
/// a multiple of 4), with a full mip chain.
fn from_png(name: &str, bytes: &[u8], convert: TextureConvert, max_size: Option<u32>) -> Result<Texture> {
    let (mut level, has_alpha) = decode_png(bytes)?;
    if let Some(limit) = max_size {
        while level.width > limit || level.height > limit {
            level = halve(&level);
        }
    }
    let blocks = level.width % 4 == 0 && level.height % 4 == 0;
    let format = match (convert, blocks, has_alpha) {
        (TextureConvert::Rgba8, ..) | (TextureConvert::Auto, false, _) => PixelFormat::Argb8888,
        (TextureConvert::Bc1 | TextureConvert::Bc3, false, _) => {
            bail!(
                "{} by {} is not a multiple of 4: block formats need it (use format = \"rgba8\")",
                level.width,
                level.height
            )
        }
        (TextureConvert::Bc1, true, _) | (TextureConvert::Auto, true, false) => PixelFormat::Dxt1,
        (TextureConvert::Bc3, true, _) | (TextureConvert::Auto, true, true) => PixelFormat::Dxt5,
    };
    let mips = full_mip_count(level.width, level.height);
    let mut source = TextureSource::from_rgba(name, level, format, mips).context("the picture cannot be encoded")?;
    if has_alpha && format != PixelFormat::Dxt1 {
        source.alpha_usage = AlphaUsage::Modulated;
    }
    Ok(source.into_texture())
}

/// A PNG as straight RGBA8 rows, and whether any pixel is not opaque.
fn decode_png(bytes: &[u8]) -> Result<(Level, bool)> {
    let mut decoder = png::Decoder::new(std::io::Cursor::new(bytes));
    decoder.set_transformations(png::Transformations::normalize_to_color8());
    let mut reader = decoder.read_info().context("not a PNG")?;
    let mut data = vec![0; reader.output_buffer_size().context("the PNG has no size")?];
    let frame = reader.next_frame(&mut data).context("the PNG does not decode")?;
    let (width, height) = (frame.width, frame.height);
    if width == 0 || height == 0 || width > MAX_SIDE || height > MAX_SIDE {
        bail!("the picture is {width} by {height}: 1 to {MAX_SIDE} on a side");
    }
    let data = &data[..frame.buffer_size()];
    let rgba: Vec<u8> = match frame.color_type {
        png::ColorType::Rgba => data.to_vec(),
        png::ColorType::Rgb => data.chunks(3).flat_map(|p| [p[0], p[1], p[2], 255]).collect(),
        png::ColorType::GrayscaleAlpha => data.chunks(2).flat_map(|p| [p[0], p[0], p[0], p[1]]).collect(),
        png::ColorType::Grayscale => data.iter().flat_map(|&g| [g, g, g, 255]).collect(),
        png::ColorType::Indexed => bail!("an indexed PNG was not expanded"),
    };
    let has_alpha = rgba.as_chunks::<4>().0.iter().any(|px| px[3] != 255);
    Ok((Level { width, height, rgba }, has_alpha))
}

/// Drop top mip levels until neither side is above `limit` or only one level is left (a texture with no mips cannot be shrunk).
fn shrink(texture: &mut Texture, limit: u32) {
    while (texture.width > limit || texture.height > limit) && texture.mip_levels > 1 {
        let first = blackbox_tpk::mip_level_size(texture.format, texture.width, texture.height, 0);
        texture.data.drain(..first);
        texture.width = (texture.width / 2).max(1);
        texture.height = (texture.height / 2).max(1);
        texture.mip_levels -= 1;
    }
}

#[cfg(test)]
#[path = "textures_tests.rs"]
mod tests;
