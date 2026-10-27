//! The `H` chunk that starts every SPS stream: the codec, the layout and the length.
//!
//! Layout, from the game's own reader (bits, most significant first): version (4), codec (4), channels minus one (6),
//! sample rate (18), kind (2), "has loop" flag (1), sample count (29). A set loop flag adds the loop start (32 bits);
//! kind 2 adds a 32-bit field; kind 1 or 2 with the flag adds one more (32 bits).

use super::bits::BitReader;
use crate::error::{Error, Result};

/// The codec of the stream, as the header numbers it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Codec {
    /// EA's single-granule MPEG-1 Layer III frames (`EL31`).
    EaLayer3,
    /// Any other number: not decoded here.
    Other(u8),
}

impl Codec {
    fn from_number(number: u8) -> Self {
        match number {
            5 => Self::EaLayer3,
            other => Self::Other(other),
        }
    }
}

/// What the `H` chunk says about the stream.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Header {
    pub version: u8,
    pub codec: Codec,
    pub channels: u16,
    pub sample_rate: u32,
    /// Samples per channel in the whole stream.
    pub samples: u32,
    /// Where the stream loops back to, when it loops.
    pub loop_start: Option<u32>,
}

/// Read the `H` chunk at the start of `data`: the header and the offset of the first chunk after it.
pub fn read(data: &[u8]) -> Result<(Header, usize)> {
    if data.first() != Some(&b'H') {
        return Err(Error::BadMagic { expected: "an H chunk" });
    }
    let size = chunk_size(data)?;
    let body = data.get(4..size).ok_or(Error::Truncated { offset: 4, needed: size.saturating_sub(4) })?;
    let mut bits = BitReader::new(body);
    let version = bits.read(4)? as u8;
    let codec = Codec::from_number(bits.read(4)? as u8);
    let channels = bits.read(6)? as u16 + 1;
    let sample_rate = bits.read(18)?;
    let kind = bits.read(2)?;
    let looped = bits.read(1)? != 0;
    let samples = bits.read(29)?;
    let loop_start = match looped {
        true => Some(bits.read(32)?),
        false => None,
    };
    if kind == 2 {
        bits.read(32)?;
    }
    if looped && (kind == 1 || kind == 2) {
        bits.read(32)?;
    }
    Ok((Header { version, codec, channels, sample_rate, samples, loop_start }, size))
}

/// The size field of the chunk at the start of `data`: the whole chunk, header included.
pub fn chunk_size(data: &[u8]) -> Result<usize> {
    let field = data.get(1..4).ok_or(Error::Truncated { offset: 1, needed: 3 })?;
    Ok(u32::from_be_bytes([0, field[0], field[1], field[2]]) as usize)
}
