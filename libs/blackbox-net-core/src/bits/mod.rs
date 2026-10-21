//! Bit packing (protocol.md section 6.1, ADR 0009 section 1).
//!
//! CONTRACT (to implement; the skeleton only fixes the API):
//!
//! - `writer.rs`: `pub struct BitWriter<'a>` over a caller-owned `&'a mut [u8]`.
//!   - `new(buf) -> Self`; bytes are cleared as the writer reaches them (the caller need not zero the buffer).
//!   - `write_bits(&mut self, value: u64, bits: u32) -> Result<(), BitError>`: `bits` in `0..=64`, most significant bit
//!     first, no alignment. Only the low `bits` bits of `value` are used; a higher bit set is `BitError::BadValue`.
//!     Not enough room is `BitError::Overflow` and leaves the writer unchanged.
//!   - `write_bool`, `write_varint(u64)` (7 data bits per group), `write_varint_n(u64, group_bits: u32)`,
//!     `write_zigzag(i64)` (zigzag then 7-bit varint), `align()` (pad with zero bits to a byte boundary).
//!   - `bits_written() -> usize`, `bytes_written() -> usize` (rounded up), `remaining_bits() -> usize`.
//! - `reader.rs`: `pub struct BitReader<'a>` over `&'a [u8]`, the mirror of the writer: `read_bits(bits) -> Result<u64>`,
//!   `read_bool`, `read_varint`, `read_varint_n(group_bits)`, `read_zigzag`, `align()` (the skipped bits must be zero,
//!   else `BitError::BadPadding`), `remaining_bits()`, `bits_read()`. Reading past the end is `BitError::Underflow`.
//! - `varint.rs` (helpers used by both): a varint with `n` data bits per group writes groups lowest first, each group
//!   being 1 continuation bit followed by `n` data bits. Canonical form only: the reader rejects a last group of zero
//!   after the first (`BitError::BadVarint`) and more groups than `ceil(64 / n)` (`BitError::BadVarint`).
//! - `ids.rs`: gap coding of strictly increasing `u16` entity ids. `IdGapEncoder { prev: Option<u16> }` with
//!   `write(&mut self, w: &mut BitWriter, id: u16) -> Result<(), BitError>`: the first id as `write_varint_n(id, 7)`, each
//!   next one as `write_varint_n(id - prev - 1, 3)`; an id not above `prev` is `BitError::BadValue`. `IdGapDecoder` mirrors
//!   it and rejects a sum past `u16::MAX` with `BitError::BadValue`. A gap of 1 costs 4 bits.
//! - `quant.rs`:
//!   - `pub struct Quant { min: f32, max: f32, bits: u32 }` with `const fn new(min, max, bits)` (bits `1..=32`),
//!     `encode(&self, x: f32) -> u32` (clamps to the range, rounds to nearest, NaN encodes as the value nearest 0.0 in
//!     range), `decode(&self, q: u32) -> Result<f32, BitError>` (`q` above the top step is `BitError::BadValue`),
//!     `step(&self) -> f32`, `bits(&self) -> u32`, and `write(&self, w, x)` / `read(&self, r)` helpers.
//!   - Angles: `encode_angle(radians: f32, bits: u32) -> u32` wraps to `[0, 2π)`; `decode_angle(q, bits) -> f32` returns
//!     `[-π, π)`.
//!   - Unit quaternions `[x, y, z, w]`: `write_quat(w, q: [f32; 4], bits: u32)` writes the index of the largest
//!     component in 2 bits, then the other three (sign flipped so the largest is positive) quantised over
//!     `[-1/√2, 1/√2]` with `bits` each; `read_quat(r, bits) -> Result<[f32; 4], BitError>` rebuilds and normalises.
//! - `tests.rs`: seeded round trips with `crate::rng::SplitMix64` (at least 10,000 cases per codec): every width 0..=64,
//!   odd offsets, overflow and underflow, canonical varints, ids, quantisation error at most half a step, angles across
//!   the wrap, quaternions within 0.002 per component for 10 bits.
//!
//! `pub enum BitError { Overflow, Underflow, BadValue, BadVarint, BadPadding }` lives here, implements `Display` and
//! `std::error::Error`, and is re-exported with the types above.

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
