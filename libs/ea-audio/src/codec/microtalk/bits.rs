//! The LSB-first bit reader of MicroTalk.
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

/// Reads bits from a byte slice, low bits first. After every read the reader holds at least 8 bits,
/// which means it is always one byte ahead of what the decoder has used. Bytes past the end read as zero.
pub struct BitReader<'a> {
    data: &'a [u8],
    pos: usize,
    value: u32,
    count: u32,
}

impl<'a> BitReader<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self { data, pos: 0, value: 0, count: 0 }
    }

    /// Next whole byte, bypassing the bit buffer.
    pub fn read_byte(&mut self) -> u8 {
        let byte = self.data.get(self.pos).copied().unwrap_or(0);
        self.pos += 1;
        byte
    }

    /// Load the first byte if the buffer is empty.
    pub fn init(&mut self) {
        if self.count != 0 {
            return;
        }
        self.value = self.read_byte() as u32;
        self.count = 8;
    }

    /// The next `n` bits (1..=8) without consuming them.
    pub fn peek(&self, n: u32) -> u32 {
        self.value & ((1 << n) - 1)
    }

    /// Consume and return `n` bits (1..=8).
    pub fn read(&mut self, n: u32) -> u32 {
        let bits = self.peek(n);
        self.value >>= n;
        self.count -= n;
        if self.count < 8 {
            self.value |= (self.read_byte() as u32) << self.count;
            self.count += 8;
        }
        bits
    }

    /// A big-endian `i16` read as two plain bytes.
    pub fn read_i16_be(&mut self) -> i16 {
        let hi = self.read_byte();
        let lo = self.read_byte();
        i16::from_be_bytes([hi, lo])
    }

    /// Bytes read so far, counting the lookahead.
    #[cfg(test)]
    pub fn position(&self) -> usize {
        self.pos
    }

    /// Give back the byte the reader looked ahead and drop the buffered bits (end of a frame in PCM-block streams).
    pub fn release_lookahead(&mut self) {
        self.pos = self.pos.saturating_sub(1);
        self.count = 0;
        self.value = 0;
    }
}
