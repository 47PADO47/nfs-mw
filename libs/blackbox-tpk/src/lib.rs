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

mod decode;
mod error;
mod format;
mod info;
pub mod layout;
mod reader;
mod texture;

#[cfg(test)]
mod tests;

pub use decode::{decode_rgba8, mip_level_size};
pub use error::{Error, Result};
pub use format::{AlphaUsage, PixelFormat};
pub use reader::read_texture_packs;
pub use texture::{Texture, TexturePack};
