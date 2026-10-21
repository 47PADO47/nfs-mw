//! Tests of the codec layer: schema limits, frames and history, entry bodies, the encode cache and the spatial hash.

mod cache;
mod codec;
mod frame;
mod garbage;
mod history;
mod priority;
mod schema;
mod snapshot;
mod spatial;

use crate::bits::BitReader;
use crate::replicate::{FieldDesc, Frame, KindId, MAX_FIELDS, Schema};
use crate::rng::SplitMix64;

/// The largest value a field of `bits` bits holds.
fn max_value(bits: u8) -> u32 {
    match bits {
        32.. => u32::MAX,
        _ => (1u32 << bits) - 1,
    }
}

/// A value for a field of `bits` bits: often 0 or the top of the range, otherwise uniform.
fn random_value(rng: &mut SplitMix64, bits: u8) -> u32 {
    match rng.below(8) {
        0 => 0,
        1 => max_value(bits),
        _ => (rng.next_u64() as u32) & max_value(bits),
    }
}

/// A random schema of 1..=6 kinds, each with 1..=32 fields of 1..=32 bits, some `diff`.
fn random_schema(rng: &mut SplitMix64) -> Schema {
    let mut schema = Schema::new();
    let kinds = 1 + rng.below(6);
    for _ in 0..kinds {
        let count = 1 + rng.below(32) as usize;
        let fields: Vec<FieldDesc> =
            (0..count).map(|_| FieldDesc { bits: (1 + rng.below(32)) as u8, diff: rng.below(2) == 1 }).collect();
        let added = schema.add_kind(&fields);
        assert!(added.is_ok());
    }
    schema
}

/// A random value for every field of `kind`.
fn random_values(schema: &Schema, kind: KindId, rng: &mut SplitMix64) -> Vec<u32> {
    let fields = schema.fields(kind).unwrap_or(&[]);
    fields.iter().map(|field| random_value(rng, field.bits)).collect()
}

/// A schema with two kinds: kind 0 has fields 12 bits diff, 7 bits raw and 32 bits diff; kind 1 has one 1-bit field.
fn fixed_schema() -> Schema {
    let mut schema = Schema::new();
    let first = schema.add_kind(&[
        FieldDesc { bits: 12, diff: true },
        FieldDesc { bits: 7, diff: false },
        FieldDesc { bits: 32, diff: true },
    ]);
    assert_eq!(first, Ok(KindId(0)));
    let second = schema.add_kind(&[FieldDesc { bits: 1, diff: false }]);
    assert_eq!(second, Ok(KindId(1)));
    schema
}

/// A frame at `tick` holding `entries` (ids must ascend).
fn frame_with(schema: &Schema, tick: u32, entries: &[(u16, KindId, Vec<u32>)]) -> Frame {
    let mut frame = Frame::new();
    frame.clear(tick);
    for (id, kind, values) in entries {
        let pushed = frame.push(schema, *id, *kind, values);
        assert!(pushed.is_ok());
    }
    frame
}

/// The first `bits` bits of `buf`, one bool per bit.
fn bit_string(buf: &[u8], bits: usize) -> Vec<bool> {
    let mut r = BitReader::new(buf);
    (0..bits).map(|_| r.read_bool().unwrap_or(false)).collect()
}

/// A full set of `MAX_FIELDS` zeros, for decoders.
fn scratch() -> [u32; MAX_FIELDS] {
    [0; MAX_FIELDS]
}
