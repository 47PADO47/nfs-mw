//! `FEngFont` chunks: a glyph table with kerning over a font texture, and the text layout the game uses.
//! Format: `docs/formats/frontend.md#fonts-fengfont-decomp--verified`.

mod layout;

pub use layout::{GlyphQuad, TextLayout, TextStyle};

use crate::error::{Error, Result};
use crate::hash::fe_hash_upper;

const NAME_LEN: usize = 0x100;
const FONT_DATA_AT: usize = 0x200;

/// One glyph.
#[derive(Clone, Copy, Debug)]
pub struct Glyph {
    pub unicode: u16,
    pub width: u8,
    pub height: u8,
    /// Position of the glyph in the font texture, in pixels.
    pub u: u16,
    pub v: u16,
    pub advance_y: i8,
    pub offset_x: i8,
    pub offset_y: i8,
    pub kern_count: u8,
    pub kern_index: u16,
    pub advance_x: i16,
}

#[derive(Clone, Copy, Debug)]
struct Kern {
    previous: u16,
    amount: i8,
}

/// A bitmap font.
#[derive(Clone, Debug)]
pub struct Font {
    pub name: String,
    pub texture_name: String,
    /// `FEHashUpper(name)`: how packages refer to the font (the resource handle of `NAME.ffn`).
    pub hash: u32,
    /// `FEHashUpper(texture_name)`: the key of the font texture.
    pub texture_hash: u32,
    pub version: u16,
    pub flags: i32,
    pub ascent: u8,
    pub descent: u8,
    glyphs: Vec<Glyph>,
    kerns: Vec<Kern>,
}

fn cstr(bytes: &[u8]) -> String {
    let end = bytes.iter().position(|b| *b == 0).unwrap_or(bytes.len());
    String::from_utf8_lossy(&bytes[..end]).into_owned()
}

fn le16(data: &[u8], at: usize) -> Option<u16> {
    Some(u16::from_le_bytes(data.get(at..at + 2)?.try_into().ok()?))
}

fn le32(data: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(data.get(at..at + 4)?.try_into().ok()?))
}

impl Font {
    /// Parses the payload of an `FEngFont` chunk (`0x00030201`).
    pub fn parse(payload: &[u8]) -> Result<Self> {
        if payload.len() < FONT_DATA_AT + 0x80 {
            return Err(Error::Truncated("font"));
        }
        let name = cstr(&payload[..NAME_LEN]);
        let texture_name = cstr(&payload[NAME_LEN..2 * NAME_LEN]);
        let f = &payload[FONT_DATA_AT..];
        if &f[..4] != b"FNTF" && &f[..4] != b"FTNF" {
            return Err(Error::NotAFont("no FNTF signature"));
        }
        let version = le16(f, 8).ok_or(Error::Truncated("font"))?;
        let count = le16(f, 10).ok_or(Error::Truncated("font"))? as usize;
        let flags = le32(f, 12).ok_or(Error::Truncated("font"))? as i32;
        let (ascent, descent) = (f[18], f[19]);
        let glyph_table = le32(f, 20).ok_or(Error::Truncated("font"))? as usize;
        let kern_table = le32(f, 24).ok_or(Error::Truncated("font"))? as usize;
        let stride = if flags & 0x40000 != 0 { 16 } else { 12 };
        if stride != 16 {
            return Err(Error::NotAFont("12-byte glyphs are not supported"));
        }
        let mut glyphs = Vec::with_capacity(count);
        for i in 0..count {
            let g = f
                .get(glyph_table + i * stride..glyph_table + (i + 1) * stride)
                .ok_or(Error::Truncated("glyph table"))?;
            glyphs.push(Glyph {
                unicode: u16::from_le_bytes([g[0], g[1]]),
                width: g[2],
                height: g[3],
                u: u16::from_le_bytes([g[4], g[5]]),
                v: u16::from_le_bytes([g[6], g[7]]),
                advance_y: g[8] as i8,
                offset_x: g[9] as i8,
                offset_y: g[10] as i8,
                kern_count: g[11],
                kern_index: u16::from_le_bytes([g[12], g[13]]),
                advance_x: i16::from_le_bytes([g[14], g[15]]),
            });
        }
        let mut kerns = Vec::new();
        if kern_table != 0 && kern_table + 4 <= f.len() {
            let n = le32(f, kern_table).unwrap_or(0) as usize;
            for i in 0..n {
                let Some(k) = f.get(kern_table + 4 + i * 4..kern_table + 8 + i * 4) else { break };
                kerns.push(Kern { previous: u16::from_le_bytes([k[0], k[1]]), amount: k[2] as i8 });
            }
        }
        Ok(Self {
            hash: fe_hash_upper(&name),
            texture_hash: fe_hash_upper(&texture_name),
            name,
            texture_name,
            version,
            flags,
            ascent,
            descent,
            glyphs,
            kerns,
        })
    }

    /// The glyph for a UTF-16 unit, found by binary search (the table is sorted by unicode).
    pub fn glyph(&self, unicode: u16) -> Option<&Glyph> {
        self.glyphs.binary_search_by_key(&unicode, |g| g.unicode).ok().map(|i| &self.glyphs[i])
    }

    pub fn glyphs(&self) -> &[Glyph] {
        &self.glyphs
    }

    /// Kerning (pixels) between `previous` and `glyph`.
    pub fn kern(&self, glyph: &Glyph, previous: u16) -> i32 {
        let start = glyph.kern_index as usize;
        let end = (start + glyph.kern_count as usize).min(self.kerns.len());
        self.kerns
            .get(start..end)
            .unwrap_or_default()
            .iter()
            .find(|k| k.previous == previous)
            .map_or(0, |k| k.amount as i32)
    }

    /// Line height in pixels (ascent plus descent).
    pub fn height(&self) -> f32 {
        self.ascent as f32 + self.descent as f32
    }
}

#[cfg(test)]
mod tests;
