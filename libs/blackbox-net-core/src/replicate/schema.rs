//! The schema: the entity kinds a game registers, and the field layout of each.

use super::{MAX_FIELDS, MAX_KINDS, ReplicateError};

/// Bits the kind id takes in a full record.
const KIND_BITS: usize = 6;
/// FNV-1a 64-bit offset basis.
const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
/// FNV-1a 64-bit prime.
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

/// One field of an entity kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FieldDesc {
    /// Width in bits, 1..=32.
    pub bits: u8,
    /// True when a change is coded as a difference (for positions) instead of raw bits.
    pub diff: bool,
}

/// The index of an entity kind in its schema.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct KindId(pub u8);

#[derive(Debug, Clone)]
struct KindDef {
    fields: Vec<FieldDesc>,
    full_bits: usize,
}

/// The entity kinds of one game. Register them in the same order on the server and on every client.
#[derive(Debug, Clone, Default)]
pub struct Schema {
    kinds: Vec<KindDef>,
}

impl Schema {
    /// An empty schema.
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers a kind with `fields` and returns its id.
    ///
    /// A kind has 1..=[`MAX_FIELDS`] fields, each 1..=32 bits, else [`ReplicateError::BadSchema`]. A schema holds at
    /// most [`MAX_KINDS`] kinds, else [`ReplicateError::TooManyKinds`].
    pub fn add_kind(&mut self, fields: &[FieldDesc]) -> Result<KindId, ReplicateError> {
        let index = u8::try_from(self.kinds.len())
            .ok()
            .filter(|index| usize::from(*index) < MAX_KINDS)
            .ok_or(ReplicateError::TooManyKinds)?;
        if fields.is_empty() || fields.len() > MAX_FIELDS {
            return Err(ReplicateError::BadSchema);
        }
        if fields.iter().any(|field| !(1..=32).contains(&field.bits)) {
            return Err(ReplicateError::BadSchema);
        }
        let field_bits: usize = fields.iter().map(|field| usize::from(field.bits)).sum();
        self.kinds.push(KindDef { fields: fields.to_vec(), full_bits: KIND_BITS + field_bits });
        Ok(KindId(index))
    }

    /// The fields of `kind`, or `None` for an unknown kind.
    pub fn fields(&self, kind: KindId) -> Option<&[FieldDesc]> {
        self.kinds.get(usize::from(kind.0)).map(|def| def.fields.as_slice())
    }

    /// How many kinds are registered.
    pub fn kind_count(&self) -> usize {
        self.kinds.len()
    }

    /// The size of a full record of `kind` in bits: the 6-bit kind id plus every field's bits.
    pub fn full_bits(&self, kind: KindId) -> Option<usize> {
        self.kinds.get(usize::from(kind.0)).map(|def| def.full_bits)
    }

    /// A 64-bit fingerprint of the layout, for the handshake: FNV-1a over the bytes `b"BBNS"`, the kind count, then
    /// for each kind its field count and for each field `bits` and `diff as u8`. Every count is one byte.
    pub fn schema_id(&self) -> u64 {
        let mut hash = FNV_OFFSET;
        for byte in b"BBNS" {
            hash = fnv_byte(hash, *byte);
        }
        hash = fnv_byte(hash, self.kinds.len() as u8);
        for kind in &self.kinds {
            hash = fnv_byte(hash, kind.fields.len() as u8);
            for field in &kind.fields {
                hash = fnv_byte(hash, field.bits);
                hash = fnv_byte(hash, u8::from(field.diff));
            }
        }
        hash
    }
}

/// One FNV-1a step.
fn fnv_byte(hash: u64, byte: u8) -> u64 {
    (hash ^ u64::from(byte)).wrapping_mul(FNV_PRIME)
}

/// Whether `value` fits in `bits` bits.
pub(crate) fn fits(value: u32, bits: u8) -> bool {
    bits >= 32 || (u64::from(value) >> bits) == 0
}
