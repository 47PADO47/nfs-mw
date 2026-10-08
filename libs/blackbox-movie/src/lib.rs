//! Reader for the movie container of EA Black Box games: a flat run of tagged blocks that interleaves
//! On2 VP6 video (`MV0K` key frames, `MV0F` inter frames) with an EA `SCHl` audio stream.
//!
//! The crate takes bytes (any [`std::io::Read`], a `&[u8]` included) and never opens files.
//! [`Demuxer`] splits the container into [`Packet`]s, [`Vp6Decoder`] (feature `vp6`, on by default)
//! turns video packets into [`YuvFrame`]s, and [`Timeline`] says which frame belongs on screen at a
//! given time. Audio packets are passed through undecoded.
//! Format notes: `docs/formats/video.md`.
//!
//! The audio (EA-XA stereo, `GSTR` header) is decoded by `libs/ea-audio`: hand it [`Demuxer::audio_header`]
//! and each [`AudioPacket`].

mod demux;
mod error;
mod header;
mod packet;
mod timing;
mod video;

pub use demux::Demuxer;
pub use error::MovieError;
pub use header::{MVHD_PAYLOAD_LEN, MovieHeader, VP6_FOURCC};
pub use packet::{AudioPacket, Packet, VideoPacket};
pub use timing::{Timeline, Update};
#[cfg(feature = "vp6")]
pub use video::Vp6Decoder;
pub use video::YuvFrame;

#[cfg(test)]
mod tests;
