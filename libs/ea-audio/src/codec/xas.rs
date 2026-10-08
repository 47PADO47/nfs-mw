//! EA-XAS version 0: mono ADPCM in independent 0x13-byte frames of 32 samples.
//!
//! The frame header carries the history, so any frame can be decoded without its neighbours. The
//! engine-loop files of EA's granular synthesiser store their audio this way.
//!
// Derived from vgmstream (https://github.com/vgmstream/vgmstream), ISC-style licence:
//
//   Copyright (c) 2008-2025 Adam Gashlin, Fastelbja, Ronny Elfert, bnnm, Christopher Snowhill,
//   NicknineTheEagle, bxaimc, Thealexbarney, CyberBotX, EdnessP, et al.
//
//   Permission to use, copy, modify, and distribute this software for any purpose with or without
//   fee is hereby granted, provided that the above copyright notice and this permission notice
//   appear in all copies.
//
//   THE SOFTWARE IS PROVIDED "AS IS" AND THE AUTHOR DISCLAIMS ALL WARRANTIES WITH REGARD TO THIS
//   SOFTWARE INCLUDING ALL IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS. IN NO EVENT SHALL
//   THE AUTHOR BE LIABLE FOR ANY SPECIAL, DIRECT, INDIRECT, OR CONSEQUENTIAL DAMAGES OR ANY DAMAGES
//   WHATSOEVER RESULTING FROM LOSS OF USE, DATA OR PROFITS, WHETHER IN AN ACTION OF CONTRACT,
//   NEGLIGENCE OR OTHER TORTIOUS ACTION, ARISING OUT OF OR IN CONNECTION WITH THE USE OR
//   PERFORMANCE OF THIS SOFTWARE.

use crate::error::{Error, Result};

/// Samples per frame (two from the header, 30 from the nibbles).
pub const FRAME_SAMPLES: usize = 32;
/// Bytes per frame: a `u32` header and 15 bytes of nibbles.
pub const FRAME_BYTES: usize = 0x13;

/// The CD-XA filter pairs; indexes above 3 select no filter.
const K0: [f32; 4] = [0.0, 0.9375, 1.796875, 1.53125];
const K1: [f32; 4] = [0.0, 0.0, -0.8125, -0.859375];

/// Decode one frame.
pub fn decode_frame(frame: &[u8; FRAME_BYTES], out: &mut [i16; FRAME_SAMPLES]) {
    let header = u32::from_le_bytes([frame[0], frame[1], frame[2], frame[3]]);
    let index = (header & 0x0F) as usize;
    let k0 = K0.get(index).copied().unwrap_or(0.0);
    let k1 = K1.get(index).copied().unwrap_or(0.0);
    let mut h2 = (header & 0xFFF0) as u16 as i16;
    let mut h1 = ((header >> 16) & 0xFFF0) as u16 as i16;
    let shift = (header >> 16) & 0x0F;
    out[0] = h2;
    out[1] = h1;
    for i in 0..FRAME_SAMPLES - 2 {
        let byte = frame[4 + i / 2];
        let nibble = if i % 2 == 0 { byte >> 4 } else { byte & 0x0F };
        let delta = ((nibble as u16) << 12) as i16 >> shift;
        let value = delta as f32 + h1 as f32 * k0 + h2 as f32 * k1;
        let sample = (value as i32).clamp(i16::MIN as i32, i16::MAX as i32) as i16;
        out[2 + i] = sample;
        h2 = h1;
        h1 = sample;
    }
}

/// Decode `sample_count` samples from consecutive frames at the start of `data`.
pub fn decode_v0(data: &[u8], sample_count: usize) -> Result<Vec<i16>> {
    let frames = sample_count.div_ceil(FRAME_SAMPLES);
    let needed = frames * FRAME_BYTES;
    if data.len() < needed {
        return Err(Error::Truncated { offset: data.len() as u64, needed: needed - data.len() });
    }
    let mut samples = Vec::with_capacity(frames * FRAME_SAMPLES);
    let mut out = [0i16; FRAME_SAMPLES];
    let (frames_data, _) = data[..needed].as_chunks::<FRAME_BYTES>();
    for frame in frames_data {
        decode_frame(frame, &mut out);
        samples.extend_from_slice(&out);
    }
    samples.truncate(sample_count);
    Ok(samples)
}
