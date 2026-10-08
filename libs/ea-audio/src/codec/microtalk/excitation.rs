//! Reading the excitation signal of one MicroTalk sub-frame.
//!
// Derived from vgmstream's src/coding/libs/utkdec.c (ISC-style licence) which is derived from
// utkencode by Andrew D'Addesio (Unlicense, public domain):
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

use super::bits::BitReader;
use super::synth::interpolate_rest;
use super::tables::{CODEBOOKS, COMMANDS};
use super::{PAD, SUBFRAME};

/// Read the 108 excitation values of a sub-frame into `ex[PAD..PAD + 108]`. Returns the factor the sub-frame's
/// fixed gain must be multiplied by (0.5 after interpolation, else 1).
pub fn read(br: &mut BitReader, multipulse: bool, reduced_bandwidth: bool, ex: &mut [f32]) -> f32 {
    if !reduced_bandwidth {
        read_values(br, multipulse, &mut ex[PAD..], 1);
        return 1.0;
    }
    let align = br.read(1) as usize;
    let zero_flag = br.read(1) == 1;
    read_values(br, multipulse, &mut ex[PAD + align..], 2);
    let other = PAD + 1 - align;
    if zero_flag {
        for j in 0..SUBFRAME / 2 {
            ex[other + 2 * j] = 0.0;
        }
        return 1.0;
    }
    ex[..PAD].fill(0.0);
    ex[PAD + SUBFRAME..].fill(0.0);
    interpolate_rest(ex, other);
    0.5
}

fn read_values(br: &mut BitReader, multipulse: bool, out: &mut [f32], stride: usize) {
    match multipulse {
        true => read_multipulse(br, out, stride),
        false => read_relp(br, out, stride),
    }
}

/// Multi-pulse model: pulses and runs of zeros coded with variable-length codes.
fn read_multipulse(br: &mut BitReader, out: &mut [f32], stride: usize) {
    let mut model = 0usize;
    let mut i = 0usize;
    while i < SUBFRAME {
        let command = CODEBOOKS[model][br.peek(8) as usize] as usize;
        let info = COMMANDS[command];
        model = info.next_model as usize;
        br.read(info.code_size);
        if command > 3 {
            out[i] = info.pulse;
            i += stride;
            continue;
        }
        if command > 1 {
            let mut count = 7 + br.read(6) as usize;
            if i + count * stride > SUBFRAME {
                count = (SUBFRAME - i) / stride;
            }
            for _ in 0..count {
                out[i] = 0.0;
                i += stride;
            }
            continue;
        }
        let mut magnitude = 7i32;
        while br.read(1) == 1 {
            magnitude += 1;
        }
        if br.read(1) == 0 {
            magnitude = -magnitude;
        }
        out[i] = magnitude as f32;
        i += stride;
    }
}

/// RELP model: every value is coded, as 0 (1 bit) or plus/minus 2 (2 bits).
fn read_relp(br: &mut BitReader, out: &mut [f32], stride: usize) {
    let mut i = 0usize;
    while i < SUBFRAME {
        let (value, bits) = match br.peek(2) {
            0 | 2 => (0.0, 1),
            1 => (-2.0, 2),
            _ => (2.0, 2),
        };
        br.read(bits);
        out[i] = value;
        i += stride;
    }
}
