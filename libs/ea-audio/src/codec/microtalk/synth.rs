//! The linear-prediction synthesis filter of MicroTalk.
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

/// Number of reflection / LPC coefficients.
pub const ORDER: usize = 12;

/// Convert 12 reflection coefficients to 12 LPC coefficients (step-up recursion).
pub fn rc_to_lpc(rc: &[f32; ORDER]) -> [f32; ORDER] {
    let mut lpc = [0f32; ORDER];
    let mut tmp1 = [0f32; ORDER];
    let mut tmp2 = [0f32; ORDER];
    for i in (0..ORDER - 1).rev() {
        tmp2[i + 1] = rc[i];
    }
    tmp2[0] = 1.0;
    for i in 0..ORDER {
        let mut x = -(rc[ORDER - 1] * tmp2[ORDER - 1]);
        for j in (0..ORDER - 1).rev() {
            x -= rc[j] * tmp2[j];
            tmp2[j + 1] = x * rc[j] + tmp2[j];
        }
        tmp2[0] = x;
        tmp1[i] = x;
        for j in 0..i {
            x -= tmp1[i - 1 - j] * lpc[j];
        }
        lpc[i] = x;
    }
    lpc
}

/// Run the synthesis filter over `samples` (a multiple of 12 values) in place, updating `history`.
pub fn filter(lpc: &[f32; ORDER], history: &mut [f32; ORDER], samples: &mut [f32]) {
    let (blocks, _) = samples.as_chunks_mut::<ORDER>();
    for block in blocks {
        for j in 0..ORDER {
            let mut x = block[j];
            for k in 0..j {
                x += lpc[k] * history[k + ORDER - j];
            }
            for k in j..ORDER {
                x += lpc[k] * history[k - j];
            }
            history[ORDER - 1 - j] = x;
            block[j] = x;
        }
    }
}

/// Fill the 108 positions of one reduced-bandwidth sub-frame that the bit stream does not code, by
/// interpolating between the coded neighbours. `ex` starts at the first position to write and must have
/// five valid values before and after the 108.
pub fn interpolate_rest(ex: &mut [f32], base: usize) {
    for i in (0..108).step_by(2) {
        let p = base + i;
        let tmp1 = (ex[p - 5] + ex[p + 5]) * 0.018_032_68;
        let tmp2 = (ex[p - 3] + ex[p + 3]) * 0.114_591_56;
        let tmp3 = (ex[p - 1] + ex[p + 1]) * 0.597_385_97;
        ex[p] = tmp1 - tmp2 + tmp3;
    }
}
