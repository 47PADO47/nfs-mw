//! EA-XA ADPCM.
//!
//! Each frame holds 28 samples per channel and is decoded with two history samples (`h1` newest). The
//! history carries from frame to frame and from block to block.
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

/// Samples in one frame.
pub const FRAME_SAMPLES: usize = 28;
/// Size of an ADPCM frame in bytes (mono).
pub const FRAME_BYTES: usize = 15;
/// Size of a PCM frame (revision 2) in bytes: marker, two history words, 28 samples.
pub const PCM_FRAME_BYTES: usize = 1 + 2 * 2 + 28 * 2;
/// Size of a stereo-flavour frame in bytes: two header bytes and one byte per sample pair.
pub const STEREO_FRAME_BYTES: usize = 2 + FRAME_SAMPLES;
/// First byte of a PCM frame.
pub const PCM_MARKER: u8 = 0xEE;

/// Filter table; `c1 = TABLE[i]`, `c2 = TABLE[i + 4]` for the 4-bit index `i`.
const TABLE: [i32; 20] = [0, 240, 460, 392, 0, 0, -208, -220, 0, 1, 3, 4, 7, 8, 10, 11, 0, -1, -3, -4];

/// Which frame format to decode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Revision {
    /// Original: no PCM frames, rounds with `+ 128` before the final shift.
    V1,
    /// Later revision with `0xEE` PCM frames; no rounding term.
    V2,
}

impl Revision {
    fn round(self) -> i32 {
        match self {
            Revision::V1 => 128,
            Revision::V2 => 0,
        }
    }
}

/// The decoder state of one channel.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct XaState {
    h1: i32,
    h2: i32,
}

fn clamp16(v: i32) -> i32 {
    v.clamp(i16::MIN as i32, i16::MAX as i32)
}

impl XaState {
    /// A fresh channel: both history samples zero.
    pub fn new() -> Self {
        Self::default()
    }

    fn step(&mut self, nibble: u8, index: usize, shift: u32, round: i32) -> i16 {
        let (c1, c2) = (TABLE[index], TABLE[index + 4]);
        let scaled = ((nibble as i32) << 28) >> shift;
        let sample = clamp16((scaled + c1 * self.h1 + c2 * self.h2 + round) >> 8);
        self.h2 = self.h1;
        self.h1 = sample;
        sample as i16
    }

    /// Decode one mono frame from the start of `data` into `out`; returns the bytes it used (15, or 61 for
    /// a PCM frame of revision 2).
    pub fn decode_frame(&mut self, data: &[u8], revision: Revision, out: &mut [i16; FRAME_SAMPLES]) -> Result<usize> {
        let truncated = Error::Truncated { offset: 0, needed: FRAME_BYTES };
        let info = *data.first().ok_or(truncated.clone())?;
        if info == PCM_MARKER && revision == Revision::V2 {
            return self.decode_pcm_frame(data, out);
        }
        let frame = data.get(..FRAME_BYTES).ok_or(truncated)?;
        let index = (info >> 4) as usize;
        let shift = (info & 0x0F) as u32 + 8;
        for (i, slot) in out.iter_mut().enumerate() {
            let byte = frame[1 + i / 2];
            let nibble = if i % 2 == 0 { byte >> 4 } else { byte & 0x0F };
            *slot = self.step(nibble, index, shift, revision.round());
        }
        Ok(FRAME_BYTES)
    }

    fn decode_pcm_frame(&mut self, data: &[u8], out: &mut [i16; FRAME_SAMPLES]) -> Result<usize> {
        let frame = data.get(..PCM_FRAME_BYTES).ok_or(Error::Truncated { offset: 0, needed: PCM_FRAME_BYTES })?;
        let word = |at: usize| i16::from_be_bytes([frame[at], frame[at + 1]]);
        self.h1 = word(1) as i32;
        self.h2 = word(3) as i32;
        for (i, slot) in out.iter_mut().enumerate() {
            *slot = word(5 + 2 * i);
        }
        Ok(PCM_FRAME_BYTES)
    }

    /// Decode enough consecutive mono frames for `sample_count` samples and append exactly that many to
    /// `out`. Returns the bytes consumed.
    pub fn decode_run(
        &mut self,
        data: &[u8],
        revision: Revision,
        sample_count: usize,
        out: &mut Vec<i16>,
    ) -> Result<usize> {
        let frames = sample_count.div_ceil(FRAME_SAMPLES);
        let start = out.len();
        let mut at = 0;
        let mut frame = [0i16; FRAME_SAMPLES];
        for _ in 0..frames {
            let rest = data.get(at..).ok_or(Error::Truncated { offset: at as u64, needed: FRAME_BYTES })?;
            at += self.decode_frame(rest, revision, &mut frame)?;
            out.extend_from_slice(&frame);
        }
        out.truncate(start + sample_count);
        Ok(at)
    }
}

/// Decode one stereo-flavour frame (30 bytes shared by both channels). The left channel uses the high
/// nibbles, the right channel the low nibbles. Rounds with `+ 128` like revision 1.
pub fn decode_stereo_frame(
    left: &mut XaState,
    right: &mut XaState,
    data: &[u8],
    out_left: &mut [i16; FRAME_SAMPLES],
    out_right: &mut [i16; FRAME_SAMPLES],
) -> Result<usize> {
    let frame = data.get(..STEREO_FRAME_BYTES).ok_or(Error::Truncated { offset: 0, needed: STEREO_FRAME_BYTES })?;
    let (coefs, shifts) = (frame[0], frame[1]);
    let (li, ri) = ((coefs >> 4) as usize, (coefs & 0x0F) as usize);
    let (ls, rs) = ((shifts >> 4) as u32 + 8, (shifts & 0x0F) as u32 + 8);
    for i in 0..FRAME_SAMPLES {
        let byte = frame[2 + i];
        out_left[i] = left.step(byte >> 4, li, ls, 128);
        out_right[i] = right.step(byte & 0x0F, ri, rs, 128);
    }
    Ok(STEREO_FRAME_BYTES)
}
