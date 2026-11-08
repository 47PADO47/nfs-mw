//! The `[[texture]]` entries of a replacement set: a texture of the game replaced, or a new one added, with where its pixels come from.

use serde::Deserialize;

use super::set::Target;
use crate::content::ContentError;
use crate::content::model_swap::field;

/// The least and the most a mod may cap a texture at (`max_size`), per side, in pixels.
const MIN_SIZE: u32 = 4;
const MAX_SIZE: u32 = 16384;

/// Where the pixels of a texture come from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TextureSource {
    /// A file of the mod's `files/` layer: `.png` (RGBA8, any size), `.dds` (BC1, BC2, BC3 as they are; RGBA8, BGRA8, BC4, BC5 and
    /// BC7 converted to RGBA8) or `.bin` (a texture pack: the texture of the entry's name, or the only one). The file may be a lazy
    /// file (one the mod makes when the game reads it): the game reads it when the texture is first needed.
    File(String),
    /// A texture of the pack `[files].textures` of the set, by name.
    Pack(String),
}

/// Whose draw state the replacing texture has: the part of a texture that is not pixels (alpha mode, blending, two-sidedness, tiling).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum StateRule {
    /// The state of the texture it replaces: the game's materials were authored for it (a cut-out leaf stays cut out).
    /// For an added texture there is nothing to keep, so it is the file's.
    #[default]
    Keep,
    /// The state the file or pack gives (a PNG with an alpha channel is a modulated texture; a DDS or pack texture keeps its own).
    File,
}

/// How a picture that is not already a game texture is stored.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Convert {
    /// BC1 for a picture with no alpha, BC3 with alpha, RGBA8 when a side is not a multiple of 4. A DDS that is BC1, BC2 or BC3 is not converted.
    #[default]
    Auto,
    /// Force BC1 (alpha dropped).
    Bc1,
    /// Force BC3.
    Bc3,
    /// Keep every pixel: uncompressed, four times the memory of BC3 on the GPU.
    Rgba8,
}

/// One texture replaced or added.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextureEntry {
    /// The texture of the game it takes the place of (`Some`), or `None` for an added one named by [`TextureEntry::name`].
    pub replaces: Option<Target>,
    /// The name an added texture has, which the set's models refer to; for a replacement, the target's name when given by name.
    pub name: String,
    pub source: TextureSource,
    pub state: StateRule,
    pub convert: Convert,
    /// Pictures larger than this on a side are halved until they fit (the mod's own limit; the game has its own too).
    pub max_size: Option<u32>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct RawTexture {
    target: Option<String>,
    target_hash: Option<u32>,
    name: Option<String>,
    file: Option<String>,
    texture: Option<String>,
    #[serde(default)]
    state: StateRule,
    #[serde(default)]
    format: Convert,
    max_size: Option<u32>,
}

/// A texture name: letters, digits, `_`, `-`, `.`, 1 to 63 bytes (the format keeps 23 in the pack but hashes the whole name).
fn is_texture_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 63
        && name.bytes().all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.'))
}

/// A texture file of the mod: a relative path of letters, digits and `_ - . /`, ending in `.png`, `.dds` or `.bin`.
fn is_texture_file(path: &str) -> bool {
    let safe = path.bytes().all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.' | b'/'));
    let parts_ok = path.split('/').all(|part| !part.is_empty() && part != "." && part != "..");
    let lower = path.to_ascii_lowercase();
    let kind_ok = [".png", ".dds", ".bin"].iter().any(|extension| lower.ends_with(extension));
    safe && parts_ok && kind_ok && path.len() <= 128
}

impl RawTexture {
    /// The checked entry. `has_pack`: the set names a texture pack, which `texture = ...` reads from.
    pub(super) fn check(self, has_pack: bool) -> Result<TextureEntry, ContentError> {
        let source = match (self.file, self.texture) {
            (Some(file), None) => {
                if !is_texture_file(&file) {
                    return Err(ContentError::InvalidText(file));
                }
                TextureSource::File(file)
            }
            (None, Some(name)) => {
                if !has_pack {
                    return Err(field("texture.texture", "the set has no [files].textures pack to read it from"));
                }
                if !is_texture_name(&name) {
                    return Err(ContentError::InvalidText(name));
                }
                TextureSource::Pack(name)
            }
            _ => {
                return Err(field(
                    "texture",
                    "give the picture as `file` or as `texture` (a texture of the pack), not both",
                ));
            }
        };
        let (replaces, name) = match (self.target, self.target_hash, self.name) {
            (Some(target), None, None) if is_texture_name(&target) => (Some(Target::Name(target.clone())), target),
            (None, Some(hash), None) => (Some(Target::Hash(hash)), format!("{hash:08X}")),
            (None, None, Some(name)) if is_texture_name(&name) => (None, name),
            (Some(bad), None, None) | (None, None, Some(bad)) => return Err(ContentError::InvalidText(bad)),
            _ => {
                return Err(field(
                    "texture",
                    "name the texture to replace with `target` or `target_hash`, or the one to add with `name`: exactly one",
                ));
            }
        };
        if let Some(size) = self.max_size
            && !(MIN_SIZE..=MAX_SIZE).contains(&size)
        {
            return Err(field("texture.max_size", "4 to 16384"));
        }
        Ok(TextureEntry { replaces, name, source, state: self.state, convert: self.format, max_size: self.max_size })
    }
}
