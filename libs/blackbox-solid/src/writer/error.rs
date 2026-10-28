//! Why a pack cannot be written.

/// What is wrong with one shading group.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GroupProblem {
    NoVertices,
    NoIndices,
    /// The index count is not a multiple of 3.
    PartialTriangle(usize),
    /// An index points past the group's own vertices.
    IndexOutOfRange {
        index: u16,
        vertices: usize,
    },
    /// The effect needs vertices this writer does not produce (skinned, or the 44-byte second-UV layout).
    WideEffect(u32),
    /// A tangent effect needs one tangent per vertex, and any other effect takes none.
    Tangents {
        effect_id: u32,
        tangents: usize,
        vertices: usize,
    },
    /// The light material is neither 0xFF (none) nor an entry of the solid's light-material list.
    LightMaterialOutOfRange {
        light_material: u8,
        listed: usize,
    },
    /// The consecutive groups with this effect id share one vertex buffer, and indices are 16-bit.
    BufferTooLarge {
        vertices: usize,
    },
}

#[derive(Debug, thiserror::Error)]
pub enum WriteError {
    #[error(transparent)]
    Chunk(#[from] blackbox_chunk::Error),
    #[error("no solids to write")]
    NoSolids,
    #[error("solid name {name:?} is empty or contains a NUL")]
    BadName { name: String },
    #[error("solids {first:?} and {second:?} have the same name hash 0x{hash:08X}")]
    DuplicateHash { first: String, second: String, hash: u32 },
    #[error("solid {solid:?} has no shading groups")]
    NoGroups { solid: String },
    #[error("solid {solid:?}, group {group}: {problem:?}")]
    Group { solid: String, group: usize, problem: GroupProblem },
    #[error("solid {solid:?} has {count} {what}, at most {max} fit")]
    TooMany { solid: String, what: &'static str, count: usize, max: usize },
    #[error("the pack is larger than 4 GiB")]
    PackTooLarge,
    #[error("solid {solid:?} cannot be rewritten: {reason}")]
    Unsupported { solid: String, reason: &'static str },
}
