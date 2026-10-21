//! The encode cache: the entry bodies of one tick, each encoded once and then copied to every client that needs it.
//!
//! Every body starts on a byte boundary in one growing byte buffer. A span records where its bits sit. Keys are
//! `(id, u32::MAX)` for a full record and `(id, base tick)` for a delta, so the tick `u32::MAX` is reserved for full
//! records.

use std::collections::HashMap;

use super::ReplicateError;
use super::codec::{OP_DELTA, OP_FULL, encode_delta, encode_full};
use super::frame::Frame;
use super::schema::{KindId, Schema};
use crate::bits::{BitError, BitReader, BitWriter};

/// Bytes the bit buffer grows by at least, and the room a body is given before it is tried.
const MIN_ROOM: usize = 4096;

/// A cached entry body: its operation code and where its bits sit in the cache buffer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    /// The operation code: [`OP_DELTA`] or [`OP_FULL`].
    pub op: u64,
    /// The first bit in the cache buffer. Spans made by the cache start on a byte boundary.
    pub start: usize,
    /// The number of bits. A delta of an unchanged entity has 0.
    pub bits: usize,
}

/// The entry bodies of one tick, with hit and miss counters.
#[derive(Debug)]
pub struct EncodeCache {
    tick: u32,
    buf: Vec<u8>,
    bit_len: usize,
    spans: HashMap<(u16, u32), Span>,
    hits: u64,
    misses: u64,
}

impl Default for EncodeCache {
    fn default() -> Self {
        Self::new()
    }
}

impl EncodeCache {
    /// An empty cache at tick 0.
    pub fn new() -> Self {
        Self { tick: 0, buf: Vec::new(), bit_len: 0, spans: HashMap::new(), hits: 0, misses: 0 }
    }

    /// Starts `tick`: forgets every span and rewinds the bit buffer. Keeps the capacity of both, so a warm cache does not
    /// allocate. The hit and miss counters keep counting.
    pub fn reset(&mut self, tick: u32) {
        self.tick = tick;
        self.bit_len = 0;
        self.spans.clear();
    }

    /// The tick the cache holds.
    pub fn tick(&self) -> u32 {
        self.tick
    }

    /// Calls answered from the cache since [`EncodeCache::new`].
    pub fn hits(&self) -> u64 {
        self.hits
    }

    /// Bodies encoded since [`EncodeCache::new`].
    pub fn misses(&self) -> u64 {
        self.misses
    }

    /// The bytes the bit buffer has room for. Does not shrink on reset.
    pub fn capacity_bytes(&self) -> usize {
        self.buf.capacity()
    }

    /// The full record of entity `id` in `frame`, from the cache or encoded now.
    ///
    /// The frame must carry the cache's tick, else [`ReplicateError::BadOrder`]. An id the frame does not hold is
    /// [`ReplicateError::UnknownEntity`].
    pub fn full(&mut self, schema: &Schema, frame: &Frame, id: u16) -> Result<Span, ReplicateError> {
        self.check_tick(frame)?;
        let key = (id, u32::MAX);
        if let Some(span) = self.cached(key) {
            return Ok(span);
        }
        let (kind, values) = frame.get(id).ok_or(ReplicateError::UnknownEntity)?;
        self.store_full(schema, key, kind, values)
    }

    /// The body of entity `id` in `frame` relative to its `base` frame, from the cache or encoded now.
    ///
    /// The result is a delta with `OP_DELTA`, or a full record with `OP_FULL` when the kinds differ, `id` is not in
    /// `base`, or the delta is not smaller than the full record. An unchanged entity gives `OP_DELTA` with 0 bits.
    /// The frame must carry the cache's tick, else [`ReplicateError::BadOrder`].
    pub fn delta(&mut self, schema: &Schema, base: &Frame, frame: &Frame, id: u16) -> Result<Span, ReplicateError> {
        self.check_tick(frame)?;
        let key = (id, base.tick());
        if let Some(span) = self.cached(key) {
            return Ok(span);
        }
        let (kind, new) = frame.get(id).ok_or(ReplicateError::UnknownEntity)?;
        let old = match base.get(id) {
            Some((base_kind, old)) if base_kind == kind => old,
            _ => return self.store_full(schema, key, kind, new),
        };
        let full_bits = schema.full_bits(kind).ok_or(ReplicateError::UnknownKind)?;
        let prev_len = self.bit_len;
        let span = self.append(OP_DELTA, |w| encode_delta(schema, kind, old, new, w).map(|_| ()))?;
        if span.bits == 0 {
            self.bit_len = prev_len;
            self.misses += 1;
            self.spans.insert(key, span);
            return Ok(span);
        }
        if span.bits >= full_bits {
            self.bit_len = prev_len;
            return self.store_full(schema, key, kind, new);
        }
        self.misses += 1;
        self.spans.insert(key, span);
        Ok(span)
    }

    /// Writes the bits of `span` to `w`. Fails with [`BitError::Overflow`] before writing when `w` has too little room.
    pub fn copy(&self, span: Span, w: &mut BitWriter<'_>) -> Result<(), BitError> {
        if w.remaining_bits() < span.bits {
            return Err(BitError::Overflow);
        }
        let bytes = self.buf.get(span.start / 8..).ok_or(BitError::Underflow)?;
        let mut r = BitReader::new(bytes);
        r.read_bits((span.start % 8) as u32)?;
        let mut left = span.bits;
        while left > 0 {
            let take = left.min(64);
            let value = r.read_bits(take as u32)?;
            w.write_bits(value, take as u32)?;
            left -= take;
        }
        Ok(())
    }

    /// Encodes a full record of `kind` under `key` and caches it.
    fn store_full(
        &mut self,
        schema: &Schema,
        key: (u16, u32),
        kind: KindId,
        values: &[u32],
    ) -> Result<Span, ReplicateError> {
        let span = self.append(OP_FULL, |w| encode_full(schema, kind, values, w))?;
        self.misses += 1;
        self.spans.insert(key, span);
        Ok(span)
    }

    /// The cached span under `key`, counting a hit.
    fn cached(&mut self, key: (u16, u32)) -> Option<Span> {
        let span = self.spans.get(&key).copied()?;
        self.hits += 1;
        Some(span)
    }

    /// Fails with [`ReplicateError::BadOrder`] unless `frame` carries the cache's tick.
    fn check_tick(&self, frame: &Frame) -> Result<(), ReplicateError> {
        if frame.tick() != self.tick {
            return Err(ReplicateError::BadOrder);
        }
        Ok(())
    }

    /// Encodes one body at the next byte boundary and returns its span. The body is encoded into the spare tail of
    /// the buffer; when it overflows, the buffer doubles and the body is encoded again. A body must write nothing when
    /// it fails.
    fn append<F>(&mut self, op: u64, encode: F) -> Result<Span, ReplicateError>
    where
        F: Fn(&mut BitWriter<'_>) -> Result<(), ReplicateError>,
    {
        let start = self.bit_len.div_ceil(8) * 8;
        let byte = start / 8;
        if self.buf.len() < byte + MIN_ROOM {
            self.buf.resize(byte + MIN_ROOM, 0);
        }
        loop {
            let Some(room) = self.buf.get_mut(byte..) else {
                return Err(ReplicateError::Bits(BitError::Overflow));
            };
            let mut w = BitWriter::new(room);
            match encode(&mut w) {
                Ok(()) => {
                    let bits = w.bits_written();
                    self.bit_len = start + bits;
                    return Ok(Span { op, start, bits });
                }
                Err(ReplicateError::Bits(BitError::Overflow)) => {
                    let doubled = self.buf.len() * 2;
                    self.buf.resize(doubled, 0);
                }
                Err(err) => return Err(err),
            }
        }
    }
}
