/// What the top byte of a resource id says about the resource.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IdClass {
    /// 0: an ordinary resource, the low 32 bits are a hash of its name.
    Normal,
    /// 1: a resource with a numeric id (the low 32 bits are a number, e.g. the 1658001 of a `MAPS` texture).
    GameChanger,
    /// 0x80 (named by the community tools; no 2012 file has it).
    ResourceList,
    /// 0xC0 (named by the community tools; no 2012 file has it).
    DeltaBundle,
    Other(u8),
}

/// The 64-bit id of a resource, taken apart. Little endian bytes: a 32-bit key, a 16-bit sub-type, an 8-bit index and
/// the class byte.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ResourceId(pub u64);

impl ResourceId {
    /// The 32-bit key: a name hash, or a number for [`IdClass::GameChanger`].
    pub const fn key(self) -> u32 {
        self.0 as u32
    }

    /// Tells apart resources that share a key. The 2012 files use the resource type here for shaders (0x53) and shader
    /// program buffers (0x8), small numbers for others.
    pub const fn sub_type(self) -> u16 {
        (self.0 >> 32) as u16
    }

    pub const fn index(self) -> u8 {
        (self.0 >> 48) as u8
    }

    pub const fn class(self) -> IdClass {
        match (self.0 >> 56) as u8 {
            0 => IdClass::Normal,
            1 => IdClass::GameChanger,
            0x80 => IdClass::ResourceList,
            0xC0 => IdClass::DeltaBundle,
            other => IdClass::Other(other),
        }
    }

    pub const fn from_parts(key: u32, sub_type: u16, index: u8, class: u8) -> Self {
        Self(key as u64 | (sub_type as u64) << 32 | (index as u64) << 48 | (class as u64) << 56)
    }
}

impl From<u64> for ResourceId {
    fn from(id: u64) -> Self {
        Self(id)
    }
}

impl std::fmt::Display for ResourceId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:016X}", self.0)
    }
}
