//! The attribute types this crate decodes, keyed by the hash of their type name.
//!
//! Type names come from the database's own type table (`Attrib::DatabaseLoadData`, see
//! [`crate::Database::type_info`]); game-specific record and enum types (`EffectLinkageRecord`,
//! `GRace::Type`, ...) are left as raw bytes.

use crate::hash::vlt_hash;
use crate::layout::Layout;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TypeKind {
    Bool,
    Int8,
    Int16,
    Int32,
    Int64,
    UInt8,
    UInt16,
    UInt32,
    UInt64,
    Float,
    Double,
    /// `EA::Reflection::Text`: a pointer to a NUL-terminated string.
    Text,
    /// `Attrib::Key`: a name hash.
    Key,
    StringKey,
    RefSpec,
    Blob,
    Vector2,
    Vector3,
    Vector4,
}

const BOOL: u32 = vlt_hash("EA::Reflection::Bool");
const INT8: u32 = vlt_hash("EA::Reflection::Int8");
const INT16: u32 = vlt_hash("EA::Reflection::Int16");
const INT32: u32 = vlt_hash("EA::Reflection::Int32");
const INT64: u32 = vlt_hash("EA::Reflection::Int64");
const UINT8: u32 = vlt_hash("EA::Reflection::UInt8");
const UINT16: u32 = vlt_hash("EA::Reflection::UInt16");
const UINT32: u32 = vlt_hash("EA::Reflection::UInt32");
const UINT64: u32 = vlt_hash("EA::Reflection::UInt64");
const FLOAT: u32 = vlt_hash("EA::Reflection::Float");
const DOUBLE: u32 = vlt_hash("EA::Reflection::Double");
const TEXT: u32 = vlt_hash("EA::Reflection::Text");
const KEY: u32 = vlt_hash("Attrib::Key");
const STRING_KEY: u32 = vlt_hash("Attrib::StringKey");
const REF_SPEC: u32 = vlt_hash("Attrib::RefSpec");
const BLOB: u32 = vlt_hash("Attrib::Blob");
const VECTOR2: u32 = vlt_hash("Attrib::Types::Vector2");
const VECTOR3: u32 = vlt_hash("Attrib::Types::Vector3");
const VECTOR4: u32 = vlt_hash("Attrib::Types::Vector4");

impl TypeKind {
    pub fn from_key(type_key: u32) -> Option<Self> {
        Some(match type_key {
            BOOL => Self::Bool,
            INT8 => Self::Int8,
            INT16 => Self::Int16,
            INT32 => Self::Int32,
            INT64 => Self::Int64,
            UINT8 => Self::UInt8,
            UINT16 => Self::UInt16,
            UINT32 => Self::UInt32,
            UINT64 => Self::UInt64,
            FLOAT => Self::Float,
            DOUBLE => Self::Double,
            TEXT => Self::Text,
            KEY => Self::Key,
            STRING_KEY => Self::StringKey,
            REF_SPEC => Self::RefSpec,
            BLOB => Self::Blob,
            VECTOR2 => Self::Vector2,
            VECTOR3 => Self::Vector3,
            VECTOR4 => Self::Vector4,
            _ => return None,
        })
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Bool => "EA::Reflection::Bool",
            Self::Int8 => "EA::Reflection::Int8",
            Self::Int16 => "EA::Reflection::Int16",
            Self::Int32 => "EA::Reflection::Int32",
            Self::Int64 => "EA::Reflection::Int64",
            Self::UInt8 => "EA::Reflection::UInt8",
            Self::UInt16 => "EA::Reflection::UInt16",
            Self::UInt32 => "EA::Reflection::UInt32",
            Self::UInt64 => "EA::Reflection::UInt64",
            Self::Float => "EA::Reflection::Float",
            Self::Double => "EA::Reflection::Double",
            Self::Text => "EA::Reflection::Text",
            Self::Key => "Attrib::Key",
            Self::StringKey => "Attrib::StringKey",
            Self::RefSpec => "Attrib::RefSpec",
            Self::Blob => "Attrib::Blob",
            Self::Vector2 => "Attrib::Types::Vector2",
            Self::Vector3 => "Attrib::Types::Vector3",
            Self::Vector4 => "Attrib::Types::Vector4",
        }
    }

    pub fn key(self) -> u32 {
        vlt_hash(self.name())
    }

    /// Size of one value in this layout. A field whose definition gives another size is read as
    /// raw bytes.
    pub fn size(self, layout: &Layout) -> usize {
        match self {
            Self::Bool | Self::Int8 | Self::UInt8 => 1,
            Self::Int16 | Self::UInt16 => 2,
            Self::Int32 | Self::UInt32 | Self::Float | Self::Key => 4,
            Self::Int64 | Self::UInt64 | Self::Double | Self::Vector2 => 8,
            Self::Vector3 => 12,
            Self::Vector4 => 16,
            Self::Text => layout.pointer_len,
            Self::StringKey => layout.string_key.len,
            Self::RefSpec => layout.ref_spec.len,
            Self::Blob => layout.blob.len,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_round_trip() {
        assert_eq!(TypeKind::from_key(0x3C16_EC5E), Some(TypeKind::Float));
        assert_eq!(TypeKind::from_key(TypeKind::Vector4.key()), Some(TypeKind::Vector4));
        assert_eq!(TypeKind::from_key(vlt_hash("Attrib::Types::Matrix")), None);
        assert_eq!(TypeKind::StringKey.size(&crate::layout::LEGACY), 16);
    }
}
