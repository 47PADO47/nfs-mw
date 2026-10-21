//! The encode cache: hits, the delta-or-full choice, buffer growth, replay of a whole tick and reuse without allocation.

use super::{bit_string, fixed_schema, frame_with, random_schema, random_value, random_values, scratch};
use crate::bits::{BitReader, BitWriter, IdGapDecoder, IdGapEncoder};
use crate::replicate::{
    EncodeCache, FieldDesc, Frame, KindId, OP_DELTA, OP_FULL, ReplicateError, Schema, decode_delta, decode_full,
    encode_full,
};
use crate::rng::SplitMix64;

#[test]
fn full_hit_returns_the_same_span_and_bits() {
    let schema = fixed_schema();
    let values = [100, 5, 9000];
    let frame = frame_with(&schema, 3, &[(7, KindId(0), values.to_vec())]);
    let mut cache = EncodeCache::new();
    cache.reset(3);
    let first = cache.full(&schema, &frame, 7).unwrap();
    let second = cache.full(&schema, &frame, 7).unwrap();
    assert_eq!(first, second);
    assert_eq!((cache.hits(), cache.misses()), (1, 1));
    assert_eq!(first.op, OP_FULL);
    assert_eq!(first.bits, 6 + 12 + 7 + 32);
    assert_eq!(first.start % 8, 0);

    let mut direct = vec![0u8; 64];
    {
        let mut w = BitWriter::new(&mut direct);
        assert_eq!(encode_full(&schema, KindId(0), &values, &mut w), Ok(()));
    }
    let mut copied = vec![0u8; 64];
    {
        let mut w = BitWriter::new(&mut copied);
        assert_eq!(cache.copy(first, &mut w), Ok(()));
        assert_eq!(w.bits_written(), first.bits);
    }
    assert_eq!(bit_string(&copied, first.bits), bit_string(&direct, first.bits));
}

#[test]
fn unknown_entity_and_wrong_tick_are_errors() {
    let schema = fixed_schema();
    let frame = frame_with(&schema, 4, &[(1, KindId(1), vec![1])]);
    let mut cache = EncodeCache::new();
    cache.reset(4);
    assert_eq!(cache.full(&schema, &frame, 2), Err(ReplicateError::UnknownEntity));
    let stale = frame_with(&schema, 3, &[(1, KindId(1), vec![1])]);
    assert_eq!(cache.full(&schema, &stale, 1), Err(ReplicateError::BadOrder));
    assert_eq!(cache.delta(&schema, &frame, &stale, 1), Err(ReplicateError::BadOrder));
    assert_eq!(cache.tick(), 4);
}

#[test]
fn unchanged_delta_is_a_zero_bit_span() {
    let schema = fixed_schema();
    let base = frame_with(&schema, 1, &[(4, KindId(0), vec![1, 2, 3])]);
    let frame = frame_with(&schema, 2, &[(4, KindId(0), vec![1, 2, 3])]);
    let mut cache = EncodeCache::new();
    cache.reset(2);
    let span = cache.delta(&schema, &base, &frame, 4).unwrap();
    assert_eq!(span.op, OP_DELTA);
    assert_eq!(span.bits, 0);
}

#[test]
fn a_delta_as_large_as_the_full_record_is_sent_full() {
    let mut schema = Schema::new();
    let eight_small = [FieldDesc { bits: 4, diff: false }; 8];
    assert_eq!(schema.add_kind(&eight_small), Ok(KindId(0)));
    let full_bits = 6 + 8 * 4;
    let base = frame_with(&schema, 1, &[(9, KindId(0), vec![1; 8])]);
    let mut cache = EncodeCache::new();

    // Every field changes: the mask (8) and the fields (32) make 40 bits, more than the 38-bit full record.
    let all = frame_with(&schema, 2, &[(9, KindId(0), vec![2; 8])]);
    cache.reset(2);
    let span = cache.delta(&schema, &base, &all, 9).unwrap();
    assert_eq!((span.op, span.bits), (OP_FULL, full_bits));

    // One field changes: 8 + 4 bits, smaller than the full record.
    let one = frame_with(&schema, 2, &[(9, KindId(0), vec![1, 1, 1, 1, 1, 1, 1, 2])]);
    cache.reset(2);
    let span = cache.delta(&schema, &base, &one, 9).unwrap();
    assert_eq!((span.op, span.bits), (OP_DELTA, 8 + 4));
}

#[test]
fn kind_change_or_missing_baseline_sends_full() {
    let schema = fixed_schema();
    let base = frame_with(&schema, 1, &[(3, KindId(1), vec![1])]);
    let frame = frame_with(&schema, 2, &[(3, KindId(0), vec![1, 1, 1]), (5, KindId(1), vec![0])]);
    let mut cache = EncodeCache::new();
    cache.reset(2);
    let changed_kind = cache.delta(&schema, &base, &frame, 3).unwrap();
    assert_eq!(changed_kind.op, OP_FULL);
    let missing = cache.delta(&schema, &base, &frame, 5).unwrap();
    assert_eq!(missing.op, OP_FULL);
    assert_eq!(missing.bits, 7);
    let hit = cache.delta(&schema, &base, &frame, 5).unwrap();
    assert_eq!(hit, missing);
}

#[test]
fn buffer_grows_when_a_tick_outgrows_it() {
    let mut schema = Schema::new();
    assert_eq!(schema.add_kind(&[FieldDesc { bits: 32, diff: false }; 8]), Ok(KindId(0)));
    let mut rng = SplitMix64(5);
    let entries: Vec<(u16, KindId, Vec<u32>)> =
        (0..2_000u16).map(|id| (id, KindId(0), (0..8).map(|_| random_value(&mut rng, 32)).collect())).collect();
    let frame = frame_with(&schema, 9, &entries);
    let mut cache = EncodeCache::new();
    cache.reset(9);
    let mut spans = Vec::new();
    for id in 0..2_000u16 {
        spans.push(cache.full(&schema, &frame, id).unwrap());
    }
    assert!(cache.capacity_bytes() >= 2_000 * 33);
    let mut stream = vec![0u8; 200_000];
    {
        let mut w = BitWriter::new(&mut stream);
        for span in &spans {
            assert_eq!(span.start % 8, 0);
            assert_eq!(cache.copy(*span, &mut w), Ok(()));
        }
    }
    let mut r = BitReader::new(&stream);
    let mut out = scratch();
    for (_, _, values) in &entries {
        assert_eq!(decode_full(&schema, &mut r, &mut out).map(|(_, n)| &out[..n]), Ok(values.as_slice()));
    }
}

#[test]
fn second_round_on_a_warm_cache_does_not_grow_the_buffer() {
    let mut schema = Schema::new();
    assert_eq!(
        schema.add_kind(&[FieldDesc { bits: 12, diff: true }, FieldDesc { bits: 9, diff: false }]),
        Ok(KindId(0))
    );
    let mut rng = SplitMix64(8);
    let base_entries: Vec<(u16, KindId, Vec<u32>)> =
        (0..300u16).map(|id| (id, KindId(0), random_values(&schema, KindId(0), &mut rng))).collect();
    let next_entries: Vec<(u16, KindId, Vec<u32>)> = base_entries
        .iter()
        .map(|(id, kind, values)| {
            let changed: Vec<u32> = values.iter().map(|v| v ^ 1).collect();
            (*id, *kind, changed)
        })
        .collect();
    let base = frame_with(&schema, 1, &base_entries);
    let frame = frame_with(&schema, 2, &next_entries);
    let mut cache = EncodeCache::new();
    let round = |cache: &mut EncodeCache| {
        cache.reset(2);
        for id in 0..300u16 {
            let _ = cache.delta(&schema, &base, &frame, id).unwrap();
            let _ = cache.full(&schema, &frame, id).unwrap();
        }
    };
    round(&mut cache);
    let warm = cache.capacity_bytes();
    assert!(warm > 0);
    round(&mut cache);
    round(&mut cache);
    assert_eq!(cache.capacity_bytes(), warm);
}

#[test]
fn replay_rebuilds_every_tick_from_cached_bodies() {
    let mut rng = SplitMix64(42);
    for _ in 0..40 {
        let schema = random_schema(&mut rng);
        let base_entries = random_entities(&schema, &mut rng, 0.5);
        let next_entries = mutate_entities(&schema, &mut rng, &base_entries);
        let base = frame_with(&schema, 10, &base_entries);
        let frame = frame_with(&schema, 11, &next_entries);

        let mut cache = EncodeCache::new();
        cache.reset(11);
        let mut stream = vec![0u8; 1 << 20];
        let mut ids = IdGapEncoder::new();
        let written = {
            let mut w = BitWriter::new(&mut stream);
            for (id, _, _) in frame.iter() {
                let span = cache.delta(&schema, &base, &frame, id).unwrap();
                assert_eq!(span.start % 8, 0);
                if span.bits == 0 {
                    continue;
                }
                assert_eq!(ids.write(&mut w, id), Ok(()));
                assert_eq!(w.write_bits(span.op, 2), Ok(()));
                assert_eq!(cache.copy(span, &mut w), Ok(()));
            }
            w.bits_written()
        };

        let decoded = decode_stream(&schema, &base, &stream, written);
        for (id, kind, values) in frame.iter() {
            let (got_kind, got) = match decoded.iter().find(|(got_id, _, _)| *got_id == id) {
                Some((_, got_kind, got)) => (*got_kind, got.clone()),
                None => {
                    let (base_kind, base_values) = base.get(id).unwrap();
                    (base_kind, base_values.to_vec())
                }
            };
            assert_eq!((got_kind, got.as_slice()), (kind, values));
        }
    }
}

/// Reads the entries written by the replay test: id gap, 2-bit op, body.
fn decode_stream(schema: &Schema, base: &Frame, stream: &[u8], written: usize) -> Vec<(u16, KindId, Vec<u32>)> {
    let mut r = BitReader::new(stream);
    let mut ids = IdGapDecoder::new();
    let mut out_entries = Vec::new();
    let mut scratch_values = scratch();
    while r.bits_read() < written {
        let id = ids.read(&mut r).unwrap();
        let op = r.read_bits(2).unwrap();
        let (kind, count) = match op {
            OP_FULL => decode_full(schema, &mut r, &mut scratch_values).unwrap(),
            OP_DELTA => {
                let (kind, old) = base.get(id).unwrap();
                let count = decode_delta(schema, kind, old, &mut r, &mut scratch_values).unwrap();
                (kind, count)
            }
            _ => panic!("unexpected op {op}"),
        };
        out_entries.push((id, kind, scratch_values[..count].to_vec()));
    }
    out_entries
}

/// Random entities: each id in 0..600 is present with probability `density`; kinds and values are random.
fn random_entities(schema: &Schema, rng: &mut SplitMix64, density: f64) -> Vec<(u16, KindId, Vec<u32>)> {
    let mut entries = Vec::new();
    for id in 0..600u16 {
        if rng.next_f64() >= density {
            continue;
        }
        let kind = KindId(rng.below(schema.kind_count() as u64) as u8);
        entries.push((id, kind, random_values(schema, kind, rng)));
    }
    entries
}

/// The next tick of `base`: most entities stay, some change field by field, some leave, a few new ones join.
fn mutate_entities(
    schema: &Schema,
    rng: &mut SplitMix64,
    base: &[(u16, KindId, Vec<u32>)],
) -> Vec<(u16, KindId, Vec<u32>)> {
    let mut next = Vec::new();
    for (id, kind, values) in base {
        if rng.below(10) == 0 {
            continue;
        }
        let fields = schema.fields(*kind).unwrap_or(&[]);
        let mut changed = values.clone();
        if rng.below(2) == 0 {
            for (slot, field) in changed.iter_mut().zip(fields) {
                if rng.below(2) == 0 {
                    *slot = random_value(rng, field.bits);
                }
            }
        }
        next.push((*id, *kind, changed));
    }
    let extra = random_entities(schema, rng, 0.02);
    for (id, kind, values) in extra {
        let present = next.iter().any(|(other, _, _)| *other == id);
        if present {
            continue;
        }
        next.push((id, kind, values));
    }
    next.sort_by_key(|(id, _, _)| *id);
    next
}

#[test]
fn spans_from_a_reset_cache_are_aligned_and_reused() {
    let schema = fixed_schema();
    let mut cache = EncodeCache::new();
    let frame = frame_with(&schema, 1, &[(1, KindId(1), vec![1]), (2, KindId(0), vec![3, 4, 5])]);
    cache.reset(1);
    let first: Vec<_> = (1..=2u16).map(|id| cache.full(&schema, &frame, id).unwrap()).collect();
    assert!(first.iter().all(|span| span.start % 8 == 0));
    cache.reset(1);
    let again: Vec<_> = (1..=2u16).map(|id| cache.full(&schema, &frame, id).unwrap()).collect();
    assert_eq!(first, again);
    assert_eq!(cache.hits(), 0);
    assert_eq!(cache.misses(), 4);
}
