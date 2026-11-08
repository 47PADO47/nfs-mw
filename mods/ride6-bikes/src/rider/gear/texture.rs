//! The pictures of the gear: `R6TX` files (`docs/formats/ride6-rider-gear.md`), a picture as RIDE 6 stores it, in blocks of a
//! block compression, with its mip levels. The importer does not decode them: it drops the levels above a size and writes the
//! rest unchanged, so a picture costs what it costs in the game and nothing is re-encoded.

use anyhow::{Context, Result, bail, ensure};

const MAGIC: &[u8; 4] = b"R6TX";
/// The most mip levels a picture can have (a 65,536 pixel side).
const MAX_MIPS: usize = 17;

pub const BC1: u32 = 1;
pub const BC2: u32 = 2;
pub const BC3: u32 = 3;
/// A single level that is a PNG file: a format the exporter had to decode.
pub const PNG: u32 = 255;

/// One mip level.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mip {
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>,
}

/// A picture.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Texture {
    pub format: u32,
    pub srgb: bool,
    /// The levels, the largest first.
    pub mips: Vec<Mip>,
}

/// The bytes of one 4 by 4 block of a block format.
fn block_bytes(format: u32) -> Option<usize> {
    match format {
        BC1 => Some(8),
        BC2 | BC3 => Some(16),
        _ => None,
    }
}

/// The bytes a level of `width` by `height` holds in `format`.
fn level_bytes(format: u32, width: u32, height: u32) -> Option<usize> {
    let block = block_bytes(format)?;
    Some(width.div_ceil(4) as usize * height.div_ceil(4) as usize * block)
}

struct Cursor<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl<'a> Cursor<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8]> {
        let end = self.at.checked_add(n).context("length overflow")?;
        let slice = self.bytes.get(self.at..end).context("the picture is cut short")?;
        self.at = end;
        Ok(slice)
    }

    fn u32(&mut self) -> Result<u32> {
        let b = self.take(4)?;
        Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }
}

/// Reads an `R6TX` file, checking every level's size against its format.
pub fn parse(bytes: &[u8]) -> Result<Texture> {
    let mut c = Cursor { bytes, at: 0 };
    ensure!(c.take(4)? == MAGIC, "not an R6TX file");
    ensure!(c.u32()? == 1, "unknown R6TX version");
    let format = c.u32()?;
    let srgb = c.take(1)?[0] != 0;
    let count = c.u32()? as usize;
    ensure!((1..=MAX_MIPS).contains(&count), "{count} mip levels");
    let mut mips = Vec::with_capacity(count);
    for _ in 0..count {
        let (width, height, length) = (c.u32()?, c.u32()?, c.u32()? as usize);
        ensure!(width > 0 && height > 0, "an empty level");
        let data = c.take(length)?.to_vec();
        match level_bytes(format, width, height) {
            Some(expected) => ensure!(
                length == expected,
                "a {width}x{height} level of format {format} holds {length} bytes, not {expected}"
            ),
            None if format == PNG => ensure!(count == 1, "a PNG picture has one level"),
            None => bail!("unknown picture format {format}"),
        }
        mips.push(Mip { width, height, data });
    }
    ensure!(c.at == bytes.len(), "trailing bytes");
    Ok(Texture { format, srgb, mips })
}

/// Serialises a picture.
pub fn to_bytes(texture: &Texture) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(MAGIC);
    out.extend_from_slice(&1u32.to_le_bytes());
    out.extend_from_slice(&texture.format.to_le_bytes());
    out.push(u8::from(texture.srgb));
    out.extend_from_slice(&(texture.mips.len() as u32).to_le_bytes());
    for mip in &texture.mips {
        out.extend_from_slice(&mip.width.to_le_bytes());
        out.extend_from_slice(&mip.height.to_le_bytes());
        out.extend_from_slice(&(mip.data.len() as u32).to_le_bytes());
        out.extend_from_slice(&mip.data);
    }
    out
}

/// Drops the levels whose longest side is above `max` pixels; when every level is above it, only the smallest stays. A PNG has
/// one level and is left as it is.
pub fn limit(texture: &mut Texture, max: u32) {
    if texture.format == PNG {
        return;
    }
    let keep = texture
        .mips
        .iter()
        .position(|mip| mip.width.max(mip.height) <= max)
        .unwrap_or(texture.mips.len().saturating_sub(1));
    texture.mips.drain(..keep);
}
