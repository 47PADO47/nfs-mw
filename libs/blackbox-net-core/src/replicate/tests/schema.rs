//! Schema limits, the layout arithmetic and the schema fingerprint.

use super::fixed_schema;
use crate::replicate::{FieldDesc, KindId, MAX_FIELDS, MAX_KINDS, ReplicateError, Schema};

#[test]
fn field_and_kind_limits_are_checked() {
    let mut schema = Schema::new();
    let one = FieldDesc { bits: 1, diff: false };
    let wide = FieldDesc { bits: 32, diff: true };
    assert_eq!(schema.add_kind(&[]), Err(ReplicateError::BadSchema));
    assert_eq!(schema.add_kind(&[FieldDesc { bits: 0, diff: false }]), Err(ReplicateError::BadSchema));
    assert_eq!(schema.add_kind(&[FieldDesc { bits: 33, diff: false }]), Err(ReplicateError::BadSchema));
    assert_eq!(schema.add_kind(&[one; MAX_FIELDS + 1]), Err(ReplicateError::BadSchema));
    assert_eq!(schema.kind_count(), 0);
    assert_eq!(schema.add_kind(&[wide; MAX_FIELDS]), Ok(KindId(0)));
    assert_eq!(schema.full_bits(KindId(0)), Some(6 + 32 * 32));
}

#[test]
fn kind_count_is_capped_at_max_kinds() {
    let mut schema = Schema::new();
    let one = FieldDesc { bits: 1, diff: false };
    for index in 0..MAX_KINDS {
        assert_eq!(schema.add_kind(&[one]), Ok(KindId(index as u8)));
    }
    assert_eq!(schema.add_kind(&[one]), Err(ReplicateError::TooManyKinds));
    assert_eq!(schema.kind_count(), MAX_KINDS);
    assert_eq!(schema.fields(KindId(63)).map(<[FieldDesc]>::len), Some(1));
    assert_eq!(schema.fields(KindId(64)), None);
    assert_eq!(schema.full_bits(KindId(64)), None);
}

#[test]
fn full_bits_is_six_plus_the_field_bits() {
    let schema = fixed_schema();
    assert_eq!(schema.kind_count(), 2);
    assert_eq!(schema.full_bits(KindId(0)), Some(6 + 12 + 7 + 32));
    assert_eq!(schema.full_bits(KindId(1)), Some(6 + 1));
    let fields = schema.fields(KindId(0)).map(<[FieldDesc]>::to_vec);
    assert_eq!(
        fields,
        Some(vec![
            FieldDesc { bits: 12, diff: true },
            FieldDesc { bits: 7, diff: false },
            FieldDesc { bits: 32, diff: true },
        ])
    );
}

#[test]
fn schema_id_is_fixed_for_a_fixed_schema() {
    // FNV-1a over b"BBNS", 2 kinds; kind 0: 3 fields (12,1) (7,0) (32,1); kind 1: 1 field (1,0).
    assert_eq!(fixed_schema().schema_id(), FIXED_SCHEMA_ID);
    assert_eq!(Schema::new().schema_id(), EMPTY_SCHEMA_ID);
}

#[test]
fn schema_id_changes_with_the_layout() {
    let base = fixed_schema().schema_id();
    let mut flipped = Schema::new();
    let first = flipped.add_kind(&[
        FieldDesc { bits: 12, diff: false },
        FieldDesc { bits: 7, diff: false },
        FieldDesc { bits: 32, diff: true },
    ]);
    assert_eq!(first, Ok(KindId(0)));
    let second = flipped.add_kind(&[FieldDesc { bits: 1, diff: false }]);
    assert_eq!(second, Ok(KindId(1)));
    assert_ne!(flipped.schema_id(), base);
    assert_eq!(Schema::new().schema_id(), Schema::new().schema_id());
    assert_ne!(Schema::new().schema_id(), base);
}

/// The fingerprint of [`fixed_schema`]: FNV-1a over the layout bytes, checked against an independent computation.
const FIXED_SCHEMA_ID: u64 = 0xcfee_62b8_7e93_a46c;
/// The fingerprint of the empty schema: FNV-1a over `b"BBNS"` and a zero kind count.
const EMPTY_SCHEMA_ID: u64 = 0xc5be_62f5_040b_7d52;
