//! EA MicroTalk (UTK): a multi-pulse / RELP speech codec, 432 samples per frame, one decoder per channel.
//!
//! The streams of the PCM-block revision start every frame with a marker byte; `0xEE` announces raw PCM
//! samples that overwrite part of the decoded frame.
//!
// Derived from vgmstream's src/coding/libs/utkdec.c and ea_mt_decoder.c (ISC-style licence), itself
// derived from utkencode by Andrew D'Addesio (Unlicense, public domain):
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

mod bits;
mod excitation;
mod synth;
mod tables;

use crate::error::{Error, Result};
use bits::BitReader;
use tables::RC_TABLE;

/// Samples produced by one frame.
pub const FRAME_SAMPLES: usize = 432;
/// Marker byte that announces a PCM patch in the PCM-block revision.
pub const PCM_MARKER: u8 = 0xEE;

const SUBFRAME: usize = 108;
/// Length of the adaptive code book that sits in front of the output in `buffer`.
const ADAPT_LEN: usize = 324;
/// Zero padding on each side of the excitation (needed by the interpolation filter).
const PAD: usize = 5;

/// The state of one MicroTalk channel.
pub struct Decoder {
    pcm_blocks: bool,
    parsed_header: bool,
    reduced_bandwidth: bool,
    multipulse_threshold: i32,
    fixed_gains: [f32; 64],
    rc: [f32; synth::ORDER],
    synth_history: [f32; synth::ORDER],
    /// Adaptive code book (`ADAPT_LEN` values) followed by the 432 output samples.
    buffer: [f32; ADAPT_LEN + FRAME_SAMPLES],
}

impl Decoder {
    /// A decoder in its initial state. `pcm_blocks` selects the revision with marker bytes and PCM patches.
    pub fn new(pcm_blocks: bool) -> Self {
        Self {
            pcm_blocks,
            parsed_header: false,
            reduced_bandwidth: false,
            multipulse_threshold: 0,
            fixed_gains: [0.0; 64],
            rc: [0.0; synth::ORDER],
            synth_history: [0.0; synth::ORDER],
            buffer: [0.0; ADAPT_LEN + FRAME_SAMPLES],
        }
    }

    /// Forget everything: the next frame parses the stream header again.
    pub fn reset(&mut self) {
        *self = Self::new(self.pcm_blocks);
    }

    /// The samples of the last decoded frame.
    pub fn samples(&self) -> &[f32] {
        &self.buffer[ADAPT_LEN..]
    }

    /// Decode all frames of `data` (the data of one channel in one block, after the leading flag byte) until
    /// `sample_count` samples have been produced, and append them as `i16`.
    pub fn decode_channel(&mut self, data: &[u8], sample_count: usize, out: &mut Vec<i16>) -> Result<()> {
        let start = out.len();
        let mut reader = BitReader::new(data);
        while out.len() - start < sample_count {
            self.decode_frame(&mut reader)?;
            out.extend(self.samples().iter().map(|&s| to_i16(s)));
        }
        out.truncate(start + sample_count);
        Ok(())
    }

    fn decode_frame(&mut self, br: &mut BitReader) -> Result<()> {
        if !self.pcm_blocks {
            self.decode_main(br);
            return Ok(());
        }
        let patched = br.read_byte() == PCM_MARKER;
        self.decode_main(br);
        br.release_lookahead();
        if !patched {
            return Ok(());
        }
        let offset = br.read_i16_be();
        let count = br.read_i16_be();
        if !(0..=FRAME_SAMPLES as i16).contains(&offset) {
            return Err(Error::Corrupt("MicroTalk PCM patch offset"));
        }
        if count < 0 || count as usize > FRAME_SAMPLES - offset as usize {
            return Err(Error::Corrupt("MicroTalk PCM patch length"));
        }
        for i in 0..count as usize {
            self.buffer[ADAPT_LEN + offset as usize + i] = br.read_i16_be() as f32;
        }
        Ok(())
    }

    fn parse_header(&mut self, br: &mut BitReader) {
        self.reduced_bandwidth = br.read(1) == 1;
        let thre = br.read(4) as i32;
        let gain = br.read(4) as f32;
        let mult = br.read(6) as f32;
        self.multipulse_threshold = 32 - thre;
        self.fixed_gains[0] = 8.0 * (1.0 + gain);
        let multiplier = 1.04 + mult * 0.001;
        for i in 1..64 {
            self.fixed_gains[i] = self.fixed_gains[i - 1] * multiplier;
        }
    }

    fn decode_main(&mut self, br: &mut BitReader) {
        br.init();
        if !self.parsed_header {
            self.parse_header(br);
            self.parsed_header = true;
        }
        let mut multipulse = false;
        let mut rc_delta = [0f32; synth::ORDER];
        for (i, delta) in rc_delta.iter_mut().enumerate() {
            let index = match i {
                0 => {
                    let v = br.read(6) as usize;
                    multipulse = (v as i32) < self.multipulse_threshold;
                    v
                }
                1..=3 => br.read(6) as usize,
                _ => 16 + br.read(5) as usize,
            };
            *delta = (RC_TABLE[index] - self.rc[i]) * 0.25;
        }

        let mut ex = [0f32; PAD + SUBFRAME + PAD];
        for i in 0..4 {
            let pitch_lag = br.read(8) as usize;
            let pitch_gain = br.read(4) as f32 / 15.0;
            let mut fixed_gain = self.fixed_gains[br.read(6) as usize];
            fixed_gain *= excitation::read(br, multipulse, self.reduced_bandwidth, &mut ex);
            for j in 0..SUBFRAME {
                let from = (SUBFRAME * i + 216 + j).saturating_sub(pitch_lag);
                let pulse = fixed_gain * ex[PAD + j];
                self.buffer[ADAPT_LEN + SUBFRAME * i + j] = pulse + pitch_gain * self.buffer[from];
            }
        }
        self.buffer.copy_within(ADAPT_LEN + SUBFRAME.., 0);

        for i in 0..4 {
            for (rc, delta) in self.rc.iter_mut().zip(&rc_delta) {
                *rc += delta;
            }
            let lpc = synth::rc_to_lpc(&self.rc);
            let blocks = if i < 3 { 1 } else { 33 };
            let from = ADAPT_LEN + synth::ORDER * i;
            synth::filter(&lpc, &mut self.synth_history, &mut self.buffer[from..from + blocks * synth::ORDER]);
        }
    }
}

/// Round half away from zero, then clamp to `i16`.
fn to_i16(sample: f32) -> i16 {
    let rounded = if sample >= 0.0 { sample + 0.5 } else { sample - 0.5 };
    (rounded as i32).clamp(i16::MIN as i32, i16::MAX as i32) as i16
}

#[cfg(test)]
mod tests;
