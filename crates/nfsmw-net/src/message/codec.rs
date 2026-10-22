//! The byte layer of the reliable messages: postcard's wire format written by hand (module docs of [`super`]).
//!
//! [`Writer`] appends to a caller-owned `Vec<u8>`. [`Reader`] walks a borrowed slice and never panics: every read checks
//! the bytes left and returns [`DecodeError::Truncated`] instead of slicing past the end.

/// Why a reliable message could not be decoded. Every malformed input ends here, never in a panic.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum DecodeError {
    /// The message is larger than [`super::MAX_MESSAGE_BYTES`], or a part of it is larger than its own limit.
    #[error("the message is larger than its limit")]
    TooLarge,
    /// The input ended inside a value.
    #[error("the input ended inside a value")]
    Truncated,
    /// A varint is longer than its type allows or not in canonical form.
    #[error("a varint is too long or not canonical")]
    BadVarint,
    /// A number is outside its field's range, a bool is not 0 or 1, or a float is not finite.
    #[error("a value is outside its range")]
    BadValue,
    /// An enum tag names no variant.
    #[error("an enum tag names no variant")]
    BadEnum,
    /// A string is not UTF-8, has a forbidden character, untrimmed whitespace or a length outside its limits.
    #[error("a string is not valid text")]
    BadText,
    /// A count is above its limit or above the bytes left.
    #[error("a count is above its limit or the bytes left")]
    BadCount,
    /// Bytes follow the last field of a message whose format this build knows.
    #[error("bytes after the last field")]
    TrailingBytes,
    /// The message type is not in this build's catalogue. Skippable: the caller counts it and moves on.
    #[error("unknown message type {0:#06x}")]
    UnknownType(u16),
    /// The format byte is older than any this build decodes for the type.
    #[error("unknown format {format} of message type {type_id:#06x}")]
    BadFormat { type_id: u16, format: u8 },
}

impl DecodeError {
    /// True for an error the receiver skips and counts without a strike of its own (protocol.md section 8.2): an
    /// unknown message type. Every other error is an invalid message.
    pub fn is_skippable(self) -> bool {
        matches!(self, Self::UnknownType(_))
    }
}

/// Why a message could not be encoded.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum EncodeError {
    /// The encoded message is larger than [`super::MAX_MESSAGE_BYTES`].
    #[error("the encoded message is larger than the reliable message limit")]
    TooLarge,
}

/// A value was over its limit when a bounded wire type was built.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("a value is over its limit or not valid text")]
pub struct LimitError;

/// Most bytes a LEB128 varint of a `u64` takes.
const MAX_VARINT_BYTES: usize = 10;

/// Appends postcard-format values to a caller-owned buffer.
#[derive(Debug)]
pub struct Writer<'a> {
    out: &'a mut Vec<u8>,
}

impl<'a> Writer<'a> {
    /// A writer that appends to `out`.
    pub fn new(out: &'a mut Vec<u8>) -> Self {
        Self { out }
    }

    /// One raw byte.
    pub fn u8(&mut self, value: u8) {
        self.out.push(value);
    }

    /// Raw bytes, no length.
    pub fn bytes(&mut self, bytes: &[u8]) {
        self.out.extend_from_slice(bytes);
    }

    /// An unsigned LEB128 varint: 7 data bits per byte, lowest group first, top bit set when more bytes follow.
    pub fn varint(&mut self, value: u64) {
        let mut rest = value;
        loop {
            let low = (rest & 0x7F) as u8;
            rest >>= 7;
            if rest == 0 {
                self.out.push(low);
                return;
            }
            self.out.push(low | 0x80);
        }
    }

    /// A signed value as a zigzag varint.
    pub fn zigzag(&mut self, value: i64) {
        self.varint(((value << 1) ^ (value >> 63)) as u64);
    }

    /// Bytes written to the buffer so far, counting what was there before.
    pub fn len(&self) -> usize {
        self.out.len()
    }

    /// Whether the buffer is empty.
    pub fn is_empty(&self) -> bool {
        self.out.is_empty()
    }
}

/// Reads postcard-format values from a borrowed slice. A failed read leaves the position unchanged.
#[derive(Debug, Clone)]
pub struct Reader<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    /// A reader at the start of `bytes`.
    pub fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, pos: 0 }
    }

    /// One raw byte.
    pub fn u8(&mut self) -> Result<u8, DecodeError> {
        let byte = *self.bytes.get(self.pos).ok_or(DecodeError::Truncated)?;
        self.pos += 1;
        Ok(byte)
    }

    /// The next `n` raw bytes.
    pub fn take(&mut self, n: usize) -> Result<&'a [u8], DecodeError> {
        let end = self.pos.checked_add(n).ok_or(DecodeError::Truncated)?;
        let slice = self.bytes.get(self.pos..end).ok_or(DecodeError::Truncated)?;
        self.pos = end;
        Ok(slice)
    }

    /// An unsigned LEB128 varint no larger than `max`. Rejects more than 10 bytes, bits past 64, a final zero byte
    /// after the first ([`DecodeError::BadVarint`]) and a value above `max` ([`DecodeError::BadValue`]).
    pub fn varint(&mut self, max: u64) -> Result<u64, DecodeError> {
        let mut value = 0u64;
        let mut cursor = self.pos;
        for index in 0..MAX_VARINT_BYTES {
            let byte = *self.bytes.get(cursor).ok_or(DecodeError::Truncated)?;
            cursor += 1;
            let data = u64::from(byte & 0x7F);
            let shift = 7 * index as u32;
            if shift == 63 && data > 1 {
                return Err(DecodeError::BadVarint);
            }
            value |= data << shift;
            if byte & 0x80 != 0 {
                continue;
            }
            if index > 0 && data == 0 {
                return Err(DecodeError::BadVarint);
            }
            if value > max {
                return Err(DecodeError::BadValue);
            }
            self.pos = cursor;
            return Ok(value);
        }
        Err(DecodeError::BadVarint)
    }

    /// A zigzag varint between `min` and `max`.
    pub fn zigzag(&mut self, min: i64, max: i64) -> Result<i64, DecodeError> {
        let mut scratch = self.clone();
        let raw = scratch.varint(u64::MAX)?;
        let value = ((raw >> 1) as i64) ^ -((raw & 1) as i64);
        if value < min || value > max {
            return Err(DecodeError::BadValue);
        }
        *self = scratch;
        Ok(value)
    }

    /// A count of elements no larger than `max` and no larger than the bytes left, so a hostile count cannot reserve
    /// more memory than the message itself (each element takes at least one byte).
    pub fn count(&mut self, max: usize) -> Result<usize, DecodeError> {
        let mut scratch = self.clone();
        let count = scratch.varint(u64::MAX).map_err(|_| DecodeError::BadCount)?;
        let count = usize::try_from(count).map_err(|_| DecodeError::BadCount)?;
        if count > max || count > scratch.remaining() {
            return Err(DecodeError::BadCount);
        }
        *self = scratch;
        Ok(count)
    }

    /// Bytes not read yet.
    pub fn remaining(&self) -> usize {
        self.bytes.len().saturating_sub(self.pos)
    }

    /// Whether every byte was read.
    pub fn is_empty(&self) -> bool {
        self.remaining() == 0
    }

    /// Bytes read so far.
    pub fn position(&self) -> usize {
        self.pos
    }
}
