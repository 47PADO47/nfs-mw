//! Record layouts per TPK version.
//!
//! Each supported version gets one file with a [`PackLayout`] constant. Add a
//! game by adding its version here once its layout is documented in
//! `docs/formats/textures.md`.

mod v5;

use std::ops::Range;

/// Field offsets inside one `TextureInfo` record.
#[derive(Debug, Clone)]
pub struct InfoLayout {
    pub len: usize,
    pub name: Range<usize>,
    pub name_hash: usize,
    pub image_placement: usize,
    pub palette_placement: usize,
    pub image_size: usize,
    pub palette_size: usize,
    /// `u16` each.
    pub width: usize,
    pub height: usize,
    pub compression_type: usize,
    pub mip_levels: usize,
    /// `ApplyAlphaSorting`: set on textures drawn alpha-blended and depth-sorted (glass, leaf cards).
    pub alpha_sorting: usize,
    pub alpha_usage: usize,
    pub alpha_blend: usize,
}

/// Everything version-specific about a pack.
#[derive(Debug, Clone)]
pub struct PackLayout {
    pub version: u32,
    /// Which game(s) are known to use it, for messages and docs.
    pub used_by: &'static str,
    pub info: InfoLayout,
    /// Size of one platform (`Comps`) record and the offset of its D3DFORMAT.
    pub plat_len: usize,
    pub plat_format: usize,
    /// Size of one streaming entry (compressed packs).
    pub entry_len: usize,
    /// Alignment of `TexturePackDataArray`'s payload.
    pub data_align: usize,
}

/// All supported layouts.
pub const LAYOUTS: &[&PackLayout] = &[&v5::V5];

/// The layout for a `TexturePackInfoHeader` version.
pub fn for_version(version: u32) -> Option<&'static PackLayout> {
    LAYOUTS.iter().copied().find(|l| l.version == version)
}
