//! Writer tests: round trips through the reader, validation, the JDLZ encoder and (when present) a real install.

mod errors;
mod install;
mod jdlz;
mod layout;
mod roundtrip;

use crate::{PixelFormat, Texture, TextureSource, mip_level_size};

/// Deterministic bytes, distinct per seed.
pub fn bytes(seed: u8, len: usize) -> Vec<u8> {
    (0..len).map(|i| seed.wrapping_mul(31).wrapping_add((i as u8).wrapping_mul(7)) ^ (i >> 8) as u8).collect()
}

/// A texture with every mip filled from `seed`.
pub fn source(name: &str, w: u32, h: u32, format: PixelFormat, mips: u32, seed: u8) -> TextureSource {
    let len: usize = (0..mips).map(|l| mip_level_size(format, w, h, l)).sum();
    TextureSource::new(name, w, h, format, mips, bytes(seed, len))
}

/// What the reader must hand back for `s`.
pub fn assert_same(t: &Texture, s: &TextureSource) {
    assert_eq!(t.name, s.name.chars().take(23).collect::<String>());
    assert_eq!(t.name_hash, s.hash(), "{}", s.name);
    assert_eq!((t.width, t.height), (s.width, s.height), "{}", s.name);
    assert_eq!(t.format, s.format, "{}", s.name);
    assert_eq!(t.mip_levels, s.mip_count, "{}", s.name);
    assert_eq!(t.data, s.data, "{}", s.name);
    assert_eq!(t.palette, s.palette, "{}", s.name);
    assert_eq!(t.compression_type, s.compression_type, "{}", s.name);
    assert_eq!(t.alpha_usage, s.alpha_usage, "{}", s.name);
    assert_eq!(t.alpha_sorting, s.alpha_sorting, "{}", s.name);
    assert_eq!(t.alpha_blend, s.alpha_blend, "{}", s.name);
    assert_eq!(t.flags, s.flags, "{}", s.name);
}
