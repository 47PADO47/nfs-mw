//! Entry bodies: layouts, random round trips and the error paths. Every encoder writes nothing when it fails.

use super::{fixed_schema, max_value, random_schema, random_value, random_values, scratch};
use crate::bits::{BitError, BitReader, BitWriter};
use crate::replicate::{
    FieldDesc, KindId, OP_DELTA, OP_FULL, OP_REMOVE, ReplicateError, Schema, decode_delta, decode_full, encode_delta,
    encode_full,
};
use crate::rng::SplitMix64;

/// Encodes one full record into a fresh buffer. Returns the buffer and the bits written.
fn full_record(schema: &Schema, kind: KindId, values: &[u32]) -> (Vec<u8>, usize) {
    let mut buf = vec![0u8; 512];
    let bits = {
        let mut w = BitWriter::new(&mut buf);
        assert_eq!(encode_full(schema, kind, values, &mut w), Ok(()));
        w.bits_written()
    };
    (buf, bits)
}

/// Encodes one delta into a fresh buffer. Returns whether anything changed, the buffer and the bits written.
fn delta_record(schema: &Schema, kind: KindId, old: &[u32], new: &[u32]) -> (bool, Vec<u8>, usize) {
    let mut buf = vec![0u8; 512];
    let (changed, bits) = {
        let mut w = BitWriter::new(&mut buf);
        let changed = encode_delta(schema, kind, old, new, &mut w).unwrap();
        (changed, w.bits_written())
    };
    (changed, buf, bits)
}

#[test]
fn op_codes_are_the_contract_values() {
    assert_eq!(OP_DELTA, 0b01);
    assert_eq!(OP_FULL, 0b10);
    assert_eq!(OP_REMOVE, 0b11);
}

#[test]
fn full_record_is_the_kind_then_raw_fields() {
    let schema = fixed_schema();
    let (buf, bits) = full_record(&schema, KindId(0), &[0xABC, 0x7F, 0x1234_5678]);
    assert_eq!(bits, 6 + 12 + 7 + 32);
    let mut r = BitReader::new(&buf);
    assert_eq!(r.read_bits(6), Ok(0));
    assert_eq!(r.read_bits(12), Ok(0xABC));
    assert_eq!(r.read_bits(7), Ok(0x7F));
    assert_eq!(r.read_bits(32), Ok(0x1234_5678));
}

#[test]
fn full_round_trip_random() {
    let mut rng = SplitMix64(11);
    for _ in 0..2_000 {
        let schema = random_schema(&mut rng);
        let kind = KindId(rng.below(schema.kind_count() as u64) as u8);
        let values = random_values(&schema, kind, &mut rng);
        let (buf, bits) = full_record(&schema, kind, &values);
        assert_eq!(schema.full_bits(kind), Some(bits));
        let mut r = BitReader::new(&buf);
        let mut out = scratch();
        let (got_kind, count) = decode_full(&schema, &mut r, &mut out).unwrap_or((KindId(255), 0));
        assert_eq!(got_kind, kind);
        assert_eq!(&out[..count], values.as_slice());
        assert_eq!(r.bits_read(), bits);
    }
}

#[test]
fn delta_layout_is_mask_then_zigzag_diff_or_raw() {
    let schema = fixed_schema();
    // Only the 12-bit diff field changes, by -1: mask 100, zigzag(-1) = 1 as a 4-bit-group varint (5 bits).
    let (changed, buf, bits) = delta_record(&schema, KindId(0), &[100, 5, 1000], &[99, 5, 1000]);
    assert!(changed);
    assert_eq!(bits, 3 + 5);
    let mut r = BitReader::new(&buf);
    assert_eq!(r.read_bits(3), Ok(0b100));
    assert_eq!(r.read_varint_n(4), Ok(1));

    // Only the 7-bit raw field changes: mask 010, then the 7 bits.
    let (changed, buf, bits) = delta_record(&schema, KindId(0), &[100, 5, 1000], &[100, 9, 1000]);
    assert!(changed);
    assert_eq!(bits, 3 + 7);
    let mut r = BitReader::new(&buf);
    assert_eq!(r.read_bits(3), Ok(0b010));
    assert_eq!(r.read_bits(7), Ok(9));
}

#[test]
fn delta_round_trip_random() {
    let mut rng = SplitMix64(21);
    for _ in 0..5_000 {
        let schema = random_schema(&mut rng);
        let kind = KindId(rng.below(schema.kind_count() as u64) as u8);
        let Some(fields) = schema.fields(kind).map(<[FieldDesc]>::to_vec) else {
            continue;
        };
        let old = random_values(&schema, kind, &mut rng);
        let new: Vec<u32> = fields
            .iter()
            .zip(&old)
            .map(|(field, &before)| match rng.below(3) {
                0 => random_value(&mut rng, field.bits),
                _ => before,
            })
            .collect();
        let (changed, buf, bits) = delta_record(&schema, kind, &old, &new);
        assert_eq!(changed, old != new);
        if !changed {
            assert_eq!(bits, 0);
            continue;
        }
        let mut r = BitReader::new(&buf);
        let mut out = scratch();
        let count = decode_delta(&schema, kind, &old, &mut r, &mut out);
        assert_eq!(count.map(|n| &out[..n]), Ok(new.as_slice()));
        assert_eq!(r.bits_read(), bits);
    }
}

#[test]
fn diff_fields_cross_the_whole_range() {
    let mut schema = Schema::new();
    let kind = schema.add_kind(&[FieldDesc { bits: 32, diff: true }]);
    assert_eq!(kind, Ok(KindId(0)));
    for (old, new) in [(0u32, u32::MAX), (u32::MAX, 0), (0, 1), (u32::MAX, u32::MAX - 1), (1 << 31, 0)] {
        let (changed, buf, bits) = delta_record(&schema, KindId(0), &[old], &[new]);
        assert!(changed);
        let mut r = BitReader::new(&buf);
        let mut out = scratch();
        assert_eq!(decode_delta(&schema, KindId(0), &[old], &mut r, &mut out), Ok(1));
        assert_eq!(out[0], new);
        assert_eq!(r.bits_read(), bits);
    }
}

#[test]
fn unchanged_entity_writes_no_bits() {
    let schema = fixed_schema();
    let values = [5, 6, 7];
    let (changed, _, bits) = delta_record(&schema, KindId(0), &values, &values);
    assert!(!changed);
    assert_eq!(bits, 0);
}

#[test]
fn decode_delta_rejects_a_value_outside_its_field() {
    let mut schema = Schema::new();
    assert_eq!(schema.add_kind(&[FieldDesc { bits: 4, diff: true }]), Ok(KindId(0)));
    // Baseline 3, delta -5 (zigzag 9): -2 is outside 0..16.
    let mut buf = vec![0u8; 8];
    {
        let mut w = BitWriter::new(&mut buf);
        assert_eq!(w.write_bits(1, 1), Ok(()));
        assert_eq!(w.write_varint_n(9, 4), Ok(()));
    }
    let mut out = scratch();
    let mut r = BitReader::new(&buf);
    assert_eq!(decode_delta(&schema, KindId(0), &[3], &mut r, &mut out), Err(ReplicateError::BadValue));
    // Baseline 15, delta +1 (zigzag 2): 16 is outside 0..16.
    let mut buf = vec![0u8; 8];
    {
        let mut w = BitWriter::new(&mut buf);
        assert_eq!(w.write_bits(1, 1), Ok(()));
        assert_eq!(w.write_varint_n(2, 4), Ok(()));
    }
    let mut r = BitReader::new(&buf);
    assert_eq!(decode_delta(&schema, KindId(0), &[15], &mut r, &mut out), Err(ReplicateError::BadValue));
}

#[test]
fn decode_full_rejects_an_unknown_kind() {
    let schema = fixed_schema();
    let mut buf = vec![0u8; 8];
    {
        let mut w = BitWriter::new(&mut buf);
        assert_eq!(w.write_bits(5, 6), Ok(()));
    }
    let mut out = scratch();
    let mut r = BitReader::new(&buf);
    assert_eq!(decode_full(&schema, &mut r, &mut out), Err(ReplicateError::UnknownKind));
}

#[test]
fn truncated_input_is_an_error_not_a_panic() {
    let schema = fixed_schema();
    let (buf, bits) = full_record(&schema, KindId(0), &[1, 2, 3]);
    let mut out = scratch();
    for bytes in 0..bits.div_ceil(8) {
        let mut r = BitReader::new(&buf[..bytes]);
        let result = decode_full(&schema, &mut r, &mut out);
        assert_eq!(result, Err(ReplicateError::Bits(BitError::Underflow)), "{bytes} bytes");
    }
    let mut r = BitReader::new(&[0xFF, 0xFF]);
    assert!(decode_delta(&schema, KindId(0), &[0, 0, 0], &mut r, &mut out).is_err());
}

#[test]
fn decode_delta_checks_the_baseline_length() {
    let schema = fixed_schema();
    let buf = [0u8; 4];
    let mut out = scratch();
    let mut r = BitReader::new(&buf);
    assert_eq!(decode_delta(&schema, KindId(0), &[1, 2], &mut r, &mut out), Err(ReplicateError::BadSchema));
}

#[test]
fn encoders_reject_bad_input_and_write_nothing() {
    let schema = fixed_schema();
    let mut buf = [0u8; 64];
    let mut w = BitWriter::new(&mut buf);
    assert_eq!(encode_full(&schema, KindId(0), &[4096, 0, 0], &mut w), Err(ReplicateError::BadValue));
    assert_eq!(encode_full(&schema, KindId(0), &[1, 2], &mut w), Err(ReplicateError::BadSchema));
    assert_eq!(encode_full(&schema, KindId(7), &[1], &mut w), Err(ReplicateError::UnknownKind));
    assert_eq!(encode_delta(&schema, KindId(0), &[1, 2, 3], &[4096, 2, 3], &mut w), Err(ReplicateError::BadValue));
    assert_eq!(encode_delta(&schema, KindId(0), &[1, 2], &[1, 2, 3], &mut w), Err(ReplicateError::BadSchema));
    assert_eq!(w.bits_written(), 0);
}

#[test]
fn encoders_overflow_without_partial_writes() {
    let schema = fixed_schema();
    let mut small = [0u8; 4];
    let mut w = BitWriter::new(&mut small);
    let full = encode_full(&schema, KindId(0), &[1, 2, 3], &mut w);
    assert_eq!(full, Err(ReplicateError::Bits(BitError::Overflow)));
    assert_eq!(w.bits_written(), 0);
    let delta = encode_delta(&schema, KindId(0), &[0, 0, 0], &[4000, 127, u32::MAX], &mut w);
    assert_eq!(delta, Err(ReplicateError::Bits(BitError::Overflow)));
    assert_eq!(w.bits_written(), 0);
}

#[test]
fn field_limits_round_trip() {
    let schema = fixed_schema();
    let values = [max_value(12), max_value(7), u32::MAX];
    let (buf, _) = full_record(&schema, KindId(0), &values);
    let mut out = scratch();
    let mut r = BitReader::new(&buf);
    assert_eq!(decode_full(&schema, &mut r, &mut out), Ok((KindId(0), 3)));
    assert_eq!(&out[..3], &values);
}
