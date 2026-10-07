#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Compress(#[from] ea_compress::Error),
    /// Neither a `VPAK` pack (after unwrapping RAWW/JDLZ/HUFF) nor anything else this crate reads.
    #[error("not an AttribSys pack (no VPAK header)")]
    NotAttrib,
    #[error("VPAK pack: {0}")]
    Pack(String),
    #[error("vault `{vault}`: {detail}")]
    Vault { vault: String, detail: String },
    /// The vault parses as chunks, but its export records don't match a known layout
    /// (for example the 2006+ "modern" AttribSys of Carbon and later).
    #[error("vault `{vault}`: unsupported AttribSys layout: {detail}")]
    UnsupportedLayout { vault: String, detail: String },
    /// A collection uses a class that no loaded vault defines. Load the vault with the class
    /// definitions (`attributes.bin` in NFS: Most Wanted) first.
    #[error("vault `{vault}`: collection 0x{collection:08X} uses class 0x{class:08X}, which is not loaded")]
    UnknownClass { vault: String, collection: u32, class: u32 },
    /// A class or collection is defined twice.
    #[error("vault `{vault}`: {what} 0x{key:08X} is already loaded")]
    Duplicate { vault: String, what: &'static str, key: u32 },
}

pub type Result<T> = std::result::Result<T, Error>;
