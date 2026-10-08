//! Errors of the demuxer.

/// What can go wrong while reading a movie.
#[derive(Debug, thiserror::Error)]
pub enum MovieError {
    /// The reader failed.
    #[error("read error: {0}")]
    Io(#[from] std::io::Error),
    /// The data does not start with an `MVhd` block.
    #[error("not an EA movie: the first block is {0:?}, expected \"MVhd\"")]
    NotAMovie([u8; 4]),
    /// The stream ended inside a block.
    #[error("truncated {tag:?} block: wanted {wanted} bytes, got {got}")]
    Truncated {
        /// Tag of the cut-off block.
        tag: [u8; 4],
        /// Bytes the block declared.
        wanted: u64,
        /// Bytes that were there.
        got: u64,
    },
    /// A block declares a size smaller than its own 8-byte header.
    #[error("block {tag:?} declares an impossible size of {size} bytes")]
    BadBlockSize {
        /// Tag of the block.
        tag: [u8; 4],
        /// The declared size.
        size: u32,
    },
    /// A block payload is shorter than its fixed fields.
    #[error("block {tag:?} payload is {len} bytes, need at least {need}")]
    ShortPayload {
        /// Tag of the block.
        tag: [u8; 4],
        /// Payload length found.
        len: usize,
        /// Payload length needed.
        need: usize,
    },
    /// The header declares a frame rate of zero or an infinite one.
    #[error("the header has rate {rate} and scale {scale}")]
    BadFrameRate {
        /// Header rate field.
        rate: u32,
        /// Header scale field.
        scale: u32,
    },
    /// The video decoder rejected a frame.
    #[error("video decode error: {0}")]
    Decode(String),
}
