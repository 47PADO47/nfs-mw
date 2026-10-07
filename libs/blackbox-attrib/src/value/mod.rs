//! Decoded attribute values.

mod decode;
mod types;

use std::borrow::Cow;

pub(crate) use decode::decode_field;
pub use types::TypeKind;

/// `Attrib::StringKey`: a string with its hashes precomputed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StringKey {
    /// The 64-bit hash, as stored (absent in layouts without one).
    pub hash64: Option<u64>,
    /// `vlt_hash` of the string.
    pub hash32: u32,
    /// `None` for a null string pointer.
    pub string: Option<String>,
}

/// `Attrib::RefSpec`: a reference to collection `collection` of class `class`. Resolve it with
/// [`crate::Database::resolve`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RefSpec {
    pub class: u32,
    pub collection: u32,
}

/// `Attrib::Blob`: an opaque byte block. In NFS games it is usually compressed (`HUFF`, `JDLZ`
/// or `RAWW` header); [`Blob::decompress`] unwraps it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Blob {
    /// The stored bytes (`size` bytes at the blob's pointer; empty for a null pointer).
    pub data: Vec<u8>,
}

impl Blob {
    /// Which compression wrapper the data starts with, if any.
    pub fn wrapper(&self) -> ea_compress::Wrapper {
        ea_compress::Wrapper::detect(&self.data)
    }

    /// The data with any `HUFF`/`JDLZ`/`RAWW` wrapper removed (borrowed when there is none).
    pub fn decompress(&self) -> crate::Result<Cow<'_, [u8]>> {
        Ok(ea_compress::unwrap(&self.data)?)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Bool(bool),
    Int8(i8),
    Int16(i16),
    Int32(i32),
    Int64(i64),
    UInt8(u8),
    UInt16(u16),
    UInt32(u32),
    UInt64(u64),
    Float(f32),
    Double(f64),
    /// `Attrib::Key`: a name hash.
    Key(u32),
    /// `EA::Reflection::Text`; `None` for a null pointer.
    Text(Option<String>),
    StringKey(StringKey),
    RefSpec(RefSpec),
    Vector2([f32; 2]),
    Vector3([f32; 3]),
    Vector4([f32; 4]),
    Blob(Blob),
    /// An array field: its items, each decoded like a scalar of the field's type.
    Array(Vec<Value>),
    /// A type this crate doesn't decode (game-specific records and enums, matrices, ...), or a
    /// known type with an unexpected size. Pointers inside are not resolved.
    Raw {
        type_key: u32,
        bytes: Vec<u8>,
    },
}

impl Value {
    /// Element `index` of an array; a scalar is its own element 0 (as AttribSys accessors treat it).
    pub fn item(&self, index: usize) -> Option<&Value> {
        match self {
            Self::Array(items) => items.get(index),
            scalar => (index == 0).then_some(scalar),
        }
    }

    pub fn as_array(&self) -> Option<&[Value]> {
        match self {
            Self::Array(items) => Some(items),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match *self {
            Self::Bool(v) => Some(v),
            _ => None,
        }
    }

    /// Any signed or unsigned integer that fits in an `i64`.
    pub fn as_i64(&self) -> Option<i64> {
        match *self {
            Self::Int8(v) => Some(v.into()),
            Self::Int16(v) => Some(v.into()),
            Self::Int32(v) => Some(v.into()),
            Self::Int64(v) => Some(v),
            Self::UInt8(v) => Some(v.into()),
            Self::UInt16(v) => Some(v.into()),
            Self::UInt32(v) => Some(v.into()),
            Self::UInt64(v) => v.try_into().ok(),
            _ => None,
        }
    }

    pub fn as_i32(&self) -> Option<i32> {
        self.as_i64().and_then(|v| v.try_into().ok())
    }

    /// Unsigned integers, keys, and 4-byte raw values (enums are stored as raw `u32`s).
    pub fn as_u32(&self) -> Option<u32> {
        match self {
            Self::Key(v) => Some(*v),
            Self::Raw { bytes, .. } => bytes.as_slice().try_into().ok().map(u32::from_le_bytes),
            other => other.as_i64().and_then(|v| v.try_into().ok()),
        }
    }

    pub fn as_f32(&self) -> Option<f32> {
        match *self {
            Self::Float(v) => Some(v),
            Self::Double(v) => Some(v as f32),
            _ => None,
        }
    }

    /// The text of a `Text` or the string of a `StringKey`.
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::Text(s) => s.as_deref(),
            Self::StringKey(k) => k.string.as_deref(),
            _ => None,
        }
    }

    pub fn as_string_key(&self) -> Option<&StringKey> {
        match self {
            Self::StringKey(k) => Some(k),
            _ => None,
        }
    }

    pub fn as_ref_spec(&self) -> Option<RefSpec> {
        match *self {
            Self::RefSpec(r) => Some(r),
            _ => None,
        }
    }

    pub fn as_vector2(&self) -> Option<[f32; 2]> {
        match *self {
            Self::Vector2(v) => Some(v),
            _ => None,
        }
    }

    pub fn as_vector3(&self) -> Option<[f32; 3]> {
        match *self {
            Self::Vector3(v) => Some(v),
            _ => None,
        }
    }

    pub fn as_vector4(&self) -> Option<[f32; 4]> {
        match *self {
            Self::Vector4(v) => Some(v),
            _ => None,
        }
    }

    pub fn as_blob(&self) -> Option<&Blob> {
        match self {
            Self::Blob(b) => Some(b),
            _ => None,
        }
    }

    pub fn as_raw(&self) -> Option<&[u8]> {
        match self {
            Self::Raw { bytes, .. } => Some(bytes),
            _ => None,
        }
    }
}
