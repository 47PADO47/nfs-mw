//! Bit-level reading and writing, most significant bit first, the order MP3 and the EA stream headers use.

use crate::error::{Error, Result};

/// Reads bits from a byte slice.
pub struct BitReader<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> BitReader<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self { data, pos: 0 }
    }

    /// Bits read so far.
    pub fn position(&self) -> usize {
        self.pos
    }

    /// Bits left.
    pub fn remaining(&self) -> usize {
        (self.data.len() * 8).saturating_sub(self.pos)
    }

    /// The next `count` bits (at most 32) as a number.
    pub fn read(&mut self, count: u32) -> Result<u32> {
        debug_assert!(count <= 32);
        if self.remaining() < count as usize {
            return Err(Error::Truncated { offset: (self.pos / 8) as u64, needed: (count as usize).div_ceil(8) });
        }
        let mut value = 0u64;
        for _ in 0..count {
            let bit = (self.data[self.pos / 8] >> (7 - self.pos % 8)) & 1;
            value = (value << 1) | u64::from(bit);
            self.pos += 1;
        }
        Ok(value as u32)
    }

    /// The next `count` bits (at most 64) as a number.
    pub fn read_wide(&mut self, count: u32) -> Result<u64> {
        debug_assert!(count <= 64);
        let high = count.saturating_sub(32);
        let top = u64::from(self.read(high)?);
        let low = u64::from(self.read(count - high)?);
        Ok((top << (count - high)) | low)
    }
}

/// Collects bits into bytes.
#[derive(Default)]
pub struct BitWriter {
    bytes: Vec<u8>,
    used: usize,
}

impl BitWriter {
    /// Append the low `count` bits of `value`.
    pub fn write(&mut self, value: u64, count: u32) {
        for shift in (0..count).rev() {
            if self.used.is_multiple_of(8) {
                self.bytes.push(0);
            }
            let bit = ((value >> shift) & 1) as u8;
            let last = self.bytes.len() - 1;
            self.bytes[last] |= bit << (7 - self.used % 8);
            self.used += 1;
        }
    }

    /// Append `count` bits of `data` starting at bit `from`.
    pub fn copy(&mut self, data: &[u8], from: usize, count: usize) {
        for at in from..from + count {
            let bit = (data[at / 8] >> (7 - at % 8)) & 1;
            self.write(u64::from(bit), 1);
        }
    }

    pub fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }
}
