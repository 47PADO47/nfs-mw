//! Which size a font's glyph coordinates are divided by.
//!
//! The glyph table was made for the stock texture. An HD replacement is larger and carries the same art scaled up, so
//! dividing by the replacement's own size would put every glyph in the wrong place. The caller passes the stock size of
//! the font texture when it knows it. Rule: `docs/specs/hd-content-support.md`, section 2.

/// The size the glyph coordinates of a font texture are divided by.
///
/// With `use_native` off, or when the caller has no stock size (`native`), this is the loaded texture's own size, which
/// is the stock behaviour. A zero dimension in `native` counts as unknown.
pub fn uv_size(use_native: bool, native: Option<(u32, u32)>, loaded: (u32, u32)) -> (u32, u32) {
    if !use_native {
        return loaded;
    }
    native.filter(|&(w, h)| w > 0 && h > 0).unwrap_or(loaded)
}
