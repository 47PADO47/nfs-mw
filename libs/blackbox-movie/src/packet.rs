//! Packets handed out by the demuxer.

/// One compressed video frame (`MV0K` or `MV0F`).
#[derive(Debug, Clone, PartialEq)]
pub struct VideoPacket {
    /// Presentation index: the ordinal of this frame among the video frames, from 0.
    pub index: u32,
    /// Whether this is a key frame (`MV0K`). Decoding can only start at a key frame.
    pub key: bool,
    /// Presentation time in seconds, `index * scale / rate`.
    pub time: f64,
    /// The raw VP6 frame.
    pub data: Vec<u8>,
}

/// One audio block (`SCDl`).
#[derive(Debug, Clone, PartialEq)]
pub struct AudioPacket {
    /// Ordinal of this block among the audio blocks, from 0.
    pub index: u32,
    /// Position of the block's first sample in the stream: the sum of the sample counts before it.
    /// Divide by the audio sample rate for a time.
    pub first_sample: u64,
    /// Samples per channel in this block (the low 24 bits of the leading big-endian u32).
    pub samples: u32,
    /// The whole `SCDl` payload, leading sample count included, as the audio decoder expects it.
    pub data: Vec<u8>,
}

/// A block of the movie, in file order.
#[derive(Debug, Clone, PartialEq)]
pub enum Packet {
    /// A video frame.
    Video(VideoPacket),
    /// A block of audio.
    Audio(AudioPacket),
    /// `SCCl`: the number of audio blocks in the movie.
    AudioCount(u32),
    /// `SCEl`: the audio stream ends here. Video frames can still follow.
    AudioEnd,
    /// A block this crate does not know, passed through untouched (including a second `SCHl`).
    Unknown {
        /// The block tag.
        tag: [u8; 4],
        /// Its payload.
        data: Vec<u8>,
    },
}
