//! The sample data of granular engine-loop files (`.gin`, magic `Gnsu`).
//!
//! The file starts with a 0x20-byte header (min and max frequency, table sizes, sample count and rate), followed by
//! two position tables and EA-XAS version 0 audio. This module reads the audio only; the tables belong to the
//! synthesiser that plays the loop. Layout: `docs/formats/audio.md`.

use crate::bytes::{slice, u32_le};
use crate::codec::xas;
use crate::error::{Error, Result};
use crate::pcm::Pcm;

/// Size of the fixed header.
const HEADER: usize = 0x20;

/// Decode a whole `.gin` file to mono PCM.
pub fn decode(data: &[u8]) -> Result<Pcm> {
    match data.get(..4) {
        Some(b"Gnsu") | Some(b"Octn") => {}
        _ => return Err(Error::BadMagic { expected: "Gnsu" }),
    }
    let segments = u32_le(data, 0x10)? as usize;
    let cycles = u32_le(data, 0x14)? as usize;
    let sample_count = u32_le(data, 0x18)? as usize;
    let sample_rate = u32_le(data, 0x1C)?;
    let start = segments
        .checked_add(1)
        .and_then(|s| s.checked_add(cycles + 1))
        .and_then(|n| n.checked_mul(4))
        .and_then(|n| n.checked_add(HEADER))
        .ok_or(Error::BadHeader("gin table sizes"))?;
    let audio = slice(data, start, data.len().saturating_sub(start))?;
    let samples = xas::decode_v0(audio, sample_count)?;
    Ok(Pcm { sample_rate, channels: 1, samples, loop_range: None })
}
