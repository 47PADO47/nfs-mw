/// Why a texture pack could not be written.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum WriteError {
    /// The pack name or file name does not fit its header field.
    #[error("texture pack header: {0}")]
    Header(String),
    /// A texture's fields are inconsistent (size, mips, data length, name, palette).
    #[error("texture {name:?}: {detail}")]
    Texture { name: String, detail: String },
    /// Two textures share a name hash (same name, or names that hash alike).
    #[error("textures {first:?} and {second:?} have the same name hash 0x{hash:08X}")]
    Duplicate { first: String, second: String, hash: u32 },
    /// The pack would not fit the format's 32-bit offsets.
    #[error("texture pack too large: {0}")]
    TooLarge(String),
}

pub type WriteResult<T> = std::result::Result<T, WriteError>;
