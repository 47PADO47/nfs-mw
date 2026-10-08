//! Reader for the movie container of EA Black Box games: a flat run of tagged blocks that interleaves
//! On2 VP6 video (`MV0K` key frames, `MV0F` inter frames) with an EA `SCHl` audio stream.
//!
//! The crate takes bytes (any [`std::io::Read`], a `&[u8]` included) and never opens files.
//! Format notes: `docs/formats/video.md`.

mod demux;
mod error;
mod header;
mod packet;

pub use demux::Demuxer;
pub use error::MovieError;
pub use header::{MVHD_PAYLOAD_LEN, MovieHeader, VP6_FOURCC};
pub use packet::{AudioPacket, Packet, VideoPacket};

#[cfg(test)]
mod tests;
