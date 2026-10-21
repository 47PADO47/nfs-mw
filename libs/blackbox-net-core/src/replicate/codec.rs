//! Entry bodies. A full record is the kind id (6 bits) then every field raw, most significant bit first. A delta record
//! is a changed-field mask (one bit per field, field 0 first, 1 = changed) then each changed field: a `diff` field as
//! the zigzag of `new - old` in a varint with 4 data bits per group, any other field as its raw bits.
//!
//! The caller writes the entry id and the 2-bit operation code around the body. Nothing here allocates.
//! Each encoder checks its whole body first, so a body that does not fit writes nothing.

use super::schema::{FieldDesc, KindId, Schema, fits};
use super::{MAX_FIELDS, ReplicateError};
use crate::bits::{BitError, BitReader, BitWriter};

/// Operation code: a delta against the baseline.
pub const OP_DELTA: u64 = 0b01;
/// Operation code: a full record.
pub const OP_FULL: u64 = 0b10;
/// Operation code: the entity left the view.
pub const OP_REMOVE: u64 = 0b11;

/// Bits the kind id takes in a full record.
const KIND_BITS: u32 = 6;
/// Data bits per group of a difference varint.
const DIFF_GROUP_BITS: u32 = 4;

/// Writes a full record of `kind`: the kind id, then every field raw. `values` must match the kind's fields and fit
/// their widths.
pub fn encode_full(schema: &Schema, kind: KindId, values: &[u32], w: &mut BitWriter<'_>) -> Result<(), ReplicateError> {
    let fields = schema.fields(kind).ok_or(ReplicateError::UnknownKind)?;
    check_values(fields, values)?;
    let total = schema.full_bits(kind).ok_or(ReplicateError::UnknownKind)?;
    if w.remaining_bits() < total {
        return Err(ReplicateError::Bits(BitError::Overflow));
    }
    w.write_bits(u64::from(kind.0), KIND_BITS)?;
    for (field, &value) in fields.iter().zip(values) {
        w.write_bits(u64::from(value), u32::from(field.bits))?;
    }
    Ok(())
}

/// Writes the delta from `old` to `new` of a `kind` entity. Returns `false` and writes nothing when they are equal;
/// otherwise writes the mask and the changed fields and returns `true`.
///
/// `new` must match the kind's fields and fit their widths. `old` must have the same field count.
pub fn encode_delta(
    schema: &Schema,
    kind: KindId,
    old: &[u32],
    new: &[u32],
    w: &mut BitWriter<'_>,
) -> Result<bool, ReplicateError> {
    let fields = schema.fields(kind).ok_or(ReplicateError::UnknownKind)?;
    check_values(fields, new)?;
    if old.len() != fields.len() {
        return Err(ReplicateError::BadSchema);
    }
    if old == new {
        return Ok(false);
    }
    let count = fields.len();
    let mut mask = 0u64;
    let mut total = count;
    for (index, ((field, &before), &after)) in fields.iter().zip(old).zip(new).enumerate() {
        if before == after {
            continue;
        }
        mask |= 1u64 << (count - 1 - index);
        total += changed_bits(field, before, after);
    }
    if w.remaining_bits() < total {
        return Err(ReplicateError::Bits(BitError::Overflow));
    }
    w.write_bits(mask, count as u32)?;
    for ((field, &before), &after) in fields.iter().zip(old).zip(new) {
        if before == after {
            continue;
        }
        if field.diff {
            w.write_varint_n(zigzag_encode(i64::from(after) - i64::from(before)), DIFF_GROUP_BITS)?;
            continue;
        }
        w.write_bits(u64::from(after), u32::from(field.bits))?;
    }
    Ok(true)
}

/// Reads a full record into `out` and returns its kind and the number of values written.
pub fn decode_full(
    schema: &Schema,
    r: &mut BitReader<'_>,
    out: &mut [u32; MAX_FIELDS],
) -> Result<(KindId, usize), ReplicateError> {
    let index = u8::try_from(r.read_bits(KIND_BITS)?).map_err(|_| ReplicateError::UnknownKind)?;
    let kind = KindId(index);
    let fields = schema.fields(kind).ok_or(ReplicateError::UnknownKind)?;
    for (field, slot) in fields.iter().zip(out.iter_mut()) {
        let value = r.read_bits(u32::from(field.bits))?;
        *slot = u32::try_from(value).map_err(|_| ReplicateError::BadValue)?;
    }
    Ok((kind, fields.len()))
}

/// Reads a delta of a `kind` entity whose baseline values are `old`, into `out`. Returns the number of values, which
/// is the kind's field count.
///
/// A decoded value outside its field's range is [`ReplicateError::BadValue`].
pub fn decode_delta(
    schema: &Schema,
    kind: KindId,
    old: &[u32],
    r: &mut BitReader<'_>,
    out: &mut [u32; MAX_FIELDS],
) -> Result<usize, ReplicateError> {
    let fields = schema.fields(kind).ok_or(ReplicateError::UnknownKind)?;
    if old.len() != fields.len() {
        return Err(ReplicateError::BadSchema);
    }
    let count = fields.len();
    let mask = r.read_bits(count as u32)?;
    for (index, ((field, &base), slot)) in fields.iter().zip(old).zip(out.iter_mut()).enumerate() {
        let changed = ((mask >> (count - 1 - index)) & 1) == 1;
        if !changed {
            *slot = base;
            continue;
        }
        *slot = read_changed(field, base, r)?;
    }
    Ok(count)
}

/// Reads one changed field whose baseline value is `base`.
fn read_changed(field: &FieldDesc, base: u32, r: &mut BitReader<'_>) -> Result<u32, ReplicateError> {
    if !field.diff {
        let value = r.read_bits(u32::from(field.bits))?;
        return u32::try_from(value).map_err(|_| ReplicateError::BadValue);
    }
    let zigzag = r.read_varint_n(DIFF_GROUP_BITS)?;
    let value = i64::from(base).checked_add(zigzag_decode(zigzag)).ok_or(ReplicateError::BadValue)?;
    let limit = 1i64 << field.bits;
    if value < 0 || value >= limit {
        return Err(ReplicateError::BadValue);
    }
    u32::try_from(value).map_err(|_| ReplicateError::BadValue)
}

/// Checks the value count and that every value fits its field.
fn check_values(fields: &[FieldDesc], values: &[u32]) -> Result<(), ReplicateError> {
    if fields.len() != values.len() {
        return Err(ReplicateError::BadSchema);
    }
    if !fields.iter().zip(values).all(|(field, &value)| fits(value, field.bits)) {
        return Err(ReplicateError::BadValue);
    }
    Ok(())
}

/// The bits one changed field takes in a delta.
fn changed_bits(field: &FieldDesc, before: u32, after: u32) -> usize {
    if field.diff {
        return varint_bits(zigzag_encode(i64::from(after) - i64::from(before)));
    }
    usize::from(field.bits)
}

/// The bits a varint with [`DIFF_GROUP_BITS`] data bits per group takes for `value`.
fn varint_bits(value: u64) -> usize {
    let needed = (64 - value.leading_zeros()).max(1) as usize;
    let group = DIFF_GROUP_BITS as usize;
    needed.div_ceil(group) * (group + 1)
}

/// Zigzag encoding of a difference: small magnitudes of either sign stay small.
fn zigzag_encode(delta: i64) -> u64 {
    ((delta << 1) ^ (delta >> 63)) as u64
}

/// Inverse of [`zigzag_encode`].
fn zigzag_decode(zigzag: u64) -> i64 {
    ((zigzag >> 1) as i64) ^ -((zigzag & 1) as i64)
}
