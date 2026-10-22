use super::BitError;
use super::varint::{data_mask, group_count, zigzag_encode};

/// Writes MSB-first bit fields into a caller-owned byte slice. Never allocates.
///
/// Bytes are cleared as the writer reaches them, so the buffer need not be zeroed first. A write that does not fit
/// returns [`BitError::Overflow`] and leaves the writer unchanged.
#[derive(Debug)]
pub struct BitWriter<'a> {
    buf: &'a mut [u8],
    bit_pos: usize,
}

impl<'a> BitWriter<'a> {
    /// A writer at bit 0 of `buf`.
    pub fn new(buf: &'a mut [u8]) -> Self {
        Self { buf, bit_pos: 0 }
    }

    /// Writes the low `bits` bits of `value`, most significant first.
    ///
    /// `bits` must be in `0..=64`. A set bit above `bits` is [`BitError::BadValue`]; too little room is
    /// [`BitError::Overflow`].
    pub fn write_bits(&mut self, value: u64, bits: u32) -> Result<(), BitError> {
        if bits > 64 {
            return Err(BitError::BadValue);
        }
        if bits < 64 && (value >> bits) != 0 {
            return Err(BitError::BadValue);
        }
        if bits as usize > self.remaining_bits() {
            return Err(BitError::Overflow);
        }
        let mut left = bits;
        while left > 0 {
            let offset = self.bit_pos % 8;
            let byte_index = self.bit_pos / 8;
            if offset == 0 {
                self.set_byte(byte_index, 0)?;
            }
            let room = 8 - offset as u32;
            let take = room.min(left);
            let chunk = (value >> (left - take)) & data_mask(take);
            let current = self.byte(byte_index)?;
            self.set_byte(byte_index, current | ((chunk << (room - take)) as u8))?;
            left -= take;
            self.bit_pos += take as usize;
        }
        Ok(())
    }

    /// Writes one bit: `true` as 1, `false` as 0.
    pub fn write_bool(&mut self, value: bool) -> Result<(), BitError> {
        self.write_bits(u64::from(value), 1)
    }

    /// Writes `value` as a varint with 7 data bits per group.
    pub fn write_varint(&mut self, value: u64) -> Result<(), BitError> {
        self.write_varint_n(value, 7)
    }

    /// Writes `value` as a varint with `group_bits` data bits per group (`1..=63`).
    ///
    /// Each group is one continuation bit (1 = more groups follow) then the data bits, lowest group first. Value 0 is
    /// a single group.
    pub fn write_varint_n(&mut self, value: u64, group_bits: u32) -> Result<(), BitError> {
        if !(1..=63).contains(&group_bits) {
            return Err(BitError::BadValue);
        }
        let groups = group_count(value, group_bits);
        let total = groups as usize * (group_bits as usize + 1);
        if total > self.remaining_bits() {
            return Err(BitError::Overflow);
        }
        let mask = data_mask(group_bits);
        let mut rest = value;
        for index in 0..groups {
            let more = u64::from(index + 1 < groups);
            let data = rest & mask;
            rest >>= group_bits;
            self.write_bits((more << group_bits) | data, group_bits + 1)?;
        }
        Ok(())
    }

    /// Writes a signed value: zigzag encoding, then a 7-bit varint.
    pub fn write_zigzag(&mut self, value: i64) -> Result<(), BitError> {
        self.write_varint(zigzag_encode(value))
    }

    /// Pads with zero bits up to the next byte boundary. Does nothing when already aligned.
    pub fn align(&mut self) -> Result<(), BitError> {
        let pad = (8 - self.bit_pos % 8) % 8;
        self.write_bits(0, pad as u32)
    }

    /// Bits written so far.
    pub fn bits_written(&self) -> usize {
        self.bit_pos
    }

    /// Bytes touched so far, counting a partly written last byte as a whole byte.
    pub fn bytes_written(&self) -> usize {
        self.bit_pos.div_ceil(8)
    }

    /// Bits still free in the buffer.
    pub fn remaining_bits(&self) -> usize {
        (self.buf.len() * 8).saturating_sub(self.bit_pos)
    }

    fn byte(&self, index: usize) -> Result<u8, BitError> {
        self.buf.get(index).copied().ok_or(BitError::Overflow)
    }

    fn set_byte(&mut self, index: usize, value: u8) -> Result<(), BitError> {
        let slot = self.buf.get_mut(index).ok_or(BitError::Overflow)?;
        *slot = value;
        Ok(())
    }
}
