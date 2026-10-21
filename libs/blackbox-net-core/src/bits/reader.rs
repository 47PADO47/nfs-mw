use super::BitError;
use super::varint::{data_mask, max_groups, zigzag_decode};

/// Reads MSB-first bit fields from a byte slice. The mirror of [`super::BitWriter`]. Never allocates.
///
/// A read that fails leaves the reader position unchanged.
#[derive(Debug, Clone)]
pub struct BitReader<'a> {
    buf: &'a [u8],
    bit_pos: usize,
}

impl<'a> BitReader<'a> {
    /// A reader at bit 0 of `buf`.
    pub fn new(buf: &'a [u8]) -> Self {
        Self { buf, bit_pos: 0 }
    }

    /// Reads `bits` bits (`0..=64`) as an unsigned value, most significant first.
    ///
    /// Too few bits left is [`BitError::Underflow`].
    pub fn read_bits(&mut self, bits: u32) -> Result<u64, BitError> {
        let value = self.peek_bits(bits)?;
        self.bit_pos += bits as usize;
        Ok(value)
    }

    /// Reads one bit as a boolean.
    pub fn read_bool(&mut self) -> Result<bool, BitError> {
        Ok(self.read_bits(1)? != 0)
    }

    /// Reads a varint written with 7 data bits per group.
    pub fn read_varint(&mut self) -> Result<u64, BitError> {
        self.read_varint_n(7)
    }

    /// Reads a varint written with `group_bits` data bits per group (`1..=63`).
    ///
    /// Rejects more groups than a 64-bit value can take, a data overflow past 64 bits, and a final zero group after the
    /// first one ([`BitError::BadVarint`]).
    pub fn read_varint_n(&mut self, group_bits: u32) -> Result<u64, BitError> {
        if !(1..=63).contains(&group_bits) {
            return Err(BitError::BadValue);
        }
        let mask = data_mask(group_bits);
        let mut value = 0u64;
        let mut shift = 0u32;
        let mut scratch = self.clone();
        for index in 0..max_groups(group_bits) {
            let group = scratch.read_bits(group_bits + 1)?;
            let more = (group >> group_bits) != 0;
            let data = group & mask;
            if data > (u64::MAX >> shift) {
                return Err(BitError::BadVarint);
            }
            if !more && index > 0 && data == 0 {
                return Err(BitError::BadVarint);
            }
            value |= data << shift;
            if !more {
                *self = scratch;
                return Ok(value);
            }
            shift += group_bits;
        }
        Err(BitError::BadVarint)
    }

    /// Reads a signed value written by [`super::BitWriter::write_zigzag`].
    pub fn read_zigzag(&mut self) -> Result<i64, BitError> {
        Ok(zigzag_decode(self.read_varint()?))
    }

    /// Skips to the next byte boundary. The skipped bits must be zero or the call fails with [`BitError::BadPadding`].
    pub fn align(&mut self) -> Result<(), BitError> {
        let pad = (8 - self.bit_pos % 8) % 8;
        if self.peek_bits(pad as u32)? != 0 {
            return Err(BitError::BadPadding);
        }
        self.bit_pos += pad;
        Ok(())
    }

    /// Bits not read yet.
    pub fn remaining_bits(&self) -> usize {
        (self.buf.len() * 8).saturating_sub(self.bit_pos)
    }

    /// Bits read so far.
    pub fn bits_read(&self) -> usize {
        self.bit_pos
    }

    /// Reads `bits` bits without moving the position.
    fn peek_bits(&self, bits: u32) -> Result<u64, BitError> {
        if bits > 64 {
            return Err(BitError::BadValue);
        }
        if bits as usize > self.remaining_bits() {
            return Err(BitError::Underflow);
        }
        let mut value = 0u64;
        let mut pos = self.bit_pos;
        let mut left = bits;
        while left > 0 {
            let offset = pos % 8;
            let room = 8 - offset as u32;
            let take = room.min(left);
            let byte = self.buf.get(pos / 8).copied().ok_or(BitError::Underflow)?;
            let chunk = (u64::from(byte) >> (room - take)) & data_mask(take);
            value = (value << take) | chunk;
            left -= take;
            pos += take as usize;
        }
        Ok(value)
    }
}
