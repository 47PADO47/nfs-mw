//! TPK texture packs from EA Black Box games. Spec: `docs/formats/textures.md`.
//!
//! A pack comes in one of two forms:
//!
//! - **Plain:** one `TextureInfo` record and one platform record per texture;
//!   the pixels live in `TexturePackDataArray` at `ImagePlacement`.
//! - **Compressed:** 24-byte streaming entries that point (by file offset) at
//!   one JDLZ or HUFF blob per texture. Each blob inflates to the pixels
//!   followed by the `TextureInfo` and the platform record.
//!
//! Record layouts differ between TPK versions (and games); they are selected by
//! the version in `TexturePackInfoHeader` (see [`layout`]). Supported today:
//! version 5 (NFS: Most Wanted, PC). Other versions fail with
//! [`Error::UnsupportedVersion`] until their layout is added.
//!
//! [`TextureSource::from_rgba`] builds a texture from RGBA8 pixels with a mip chain and a small DXT1/DXT5 encoder.
//!
//! [`write_tpk`] goes the other way: it builds a version 5 pack (the car `TEXTURES.BIN` form, or the plain form)
//! from [`TextureSource`]s, and the result reads back to the same textures.

mod anim;
mod decode;
mod encode;
mod error;
mod format;
mod info;
pub mod layout;
mod reader;
mod scroll;
mod texture;
mod writer;

#[cfg(test)]
mod tests;

pub use anim::{TextureAnim, read_texture_anims};
pub use decode::{decode_mip_rgba8, decode_rgba8, decode_rgba8_cutout, mip_level_size};
pub use encode::{Level, encode_level, full_mip_count, halve, mip_chain};
pub use error::{Error, Result};
pub use format::{AlphaUsage, PixelFormat};
pub use reader::{read_texture_packs, read_texture_packs_filtered, read_texture_packs_ranged};
pub use scroll::{TextureScroll, read_texture_scrolls};
pub use texture::{Texture, TexturePack};
pub use writer::{
    DEFAULT_RENDERING_ORDER, MAX_DIMENSION, MAX_STORED_NAME, PackForm, TextureSource, WriteError, WriteOptions,
    WriteResult, default_compression_type, write_tpk, write_tpk_with,
};
