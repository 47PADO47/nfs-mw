//! Bit packing (protocol.md section 6.1, ADR 0009 section 1).
//!
//! Bits are written most significant bit first with no alignment, so a field can start at any bit. Bytes are
//! cleared as the writer reaches them, so the caller does not need to zero the buffer.
//!
//! - [`BitWriter`] and [`BitReader`]: fixed-width fields, booleans, varints, zigzag integers and byte alignment.
//!   Neither allocates. A failed write leaves the writer unchanged.
//! - Varints use groups of `n` data bits (7 by default). Each group is one continuation bit (1 = more groups follow)
//!   then the data bits, lowest group first. The reader accepts only the canonical form: no trailing zero group and no
//!   more groups than the 64-bit value needs.
//! - [`IdGapEncoder`] and [`IdGapDecoder`]: strictly increasing `u16` entity ids, first id as a 7-bit varint, then
//!   each gap minus one as a 3-bit varint. Consecutive ids cost 4 bits.
//! - Quantisers: [`Quant`] for uniform floats, [`encode_angle`] and [`decode_angle`] for wrapped angles, and
//!   [`write_quat`] and [`read_quat`] for unit quaternions (smallest-three).

mod ids;
mod quant;
mod reader;
mod varint;
mod writer;

#[cfg(test)]
mod tests;

pub use ids::{IdGapDecoder, IdGapEncoder};
pub use quant::{Quant, decode_angle, encode_angle, read_quat, write_quat};
pub use reader::BitReader;
pub use writer::BitWriter;

use std::fmt;

/// Why a bit stream could not be written or read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BitError {
    /// Not enough room left in the output buffer.
    Overflow,
    /// The input ended before the value.
    Underflow,
    /// A value outside the field's range.
    BadValue,
    /// A varint that is too long or not in canonical form.
    BadVarint,
    /// Nonzero bits where zero padding was expected.
    BadPadding,
}

impl fmt::Display for BitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let text = match self {
            BitError::Overflow => "bit stream overflow",
            BitError::Underflow => "bit stream underflow",
            BitError::BadValue => "value out of range",
            BitError::BadVarint => "malformed varint",
            BitError::BadPadding => "nonzero padding bits",
        };
        f.write_str(text)
    }
}

impl std::error::Error for BitError {}
