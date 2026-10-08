//! The `MVhd` video header.

use crate::error::MovieError;

/// FourCC stored in `MVhd` for VP6 video: bytes `30 36 50 56` (`'VP60'` as a little-endian u32).
pub const VP6_FOURCC: [u8; 4] = [0x30, 0x36, 0x50, 0x56];

/// Payload size of an `MVhd` block (block size 0x20 minus the 8-byte block header).
pub const MVHD_PAYLOAD_LEN: usize = 24;

/// Contents of the `MVhd` block.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MovieHeader {
    /// Codec FourCC exactly as stored.
    pub codec: [u8; 4],
    /// Picture width in pixels.
    pub width: u16,
    /// Picture height in pixels.
    pub height: u16,
    /// Number of video frames (`MV0K` plus `MV0F` blocks).
    pub frame_count: u32,
    /// Size of the largest frame payload; matches the real maximum to within block padding.
    pub max_frame_size: u32,
    /// Frame rate numerator.
    pub rate: u32,
    /// Frame rate denominator.
    pub scale: u32,
}

impl MovieHeader {
    /// Parses the payload of an `MVhd` block (the bytes after the block's 8-byte header).
    pub fn parse(payload: &[u8]) -> Result<Self, MovieError> {
        if payload.len() < MVHD_PAYLOAD_LEN {
            return Err(MovieError::ShortPayload { tag: *b"MVhd", len: payload.len(), need: MVHD_PAYLOAD_LEN });
        }
        let u16_at = |at: usize| u16::from_le_bytes([payload[at], payload[at + 1]]);
        let u32_at = |at: usize| u32::from_le_bytes([payload[at], payload[at + 1], payload[at + 2], payload[at + 3]]);
        let header = Self {
            codec: [payload[0], payload[1], payload[2], payload[3]],
            width: u16_at(4),
            height: u16_at(6),
            frame_count: u32_at(8),
            max_frame_size: u32_at(12),
            rate: u32_at(16),
            scale: u32_at(20),
        };
        if header.rate == 0 || header.scale == 0 {
            return Err(MovieError::BadFrameRate { rate: header.rate, scale: header.scale });
        }
        Ok(header)
    }

    /// Whether the codec FourCC is VP6.
    pub fn is_vp6(&self) -> bool {
        self.codec == VP6_FOURCC
    }

    /// Frames per second, `rate / scale` (29.97 in Most Wanted).
    pub fn fps(&self) -> f64 {
        f64::from(self.rate) / f64::from(self.scale)
    }

    /// Presentation time in seconds of the frame with the given index.
    pub fn frame_time(&self, index: u32) -> f64 {
        f64::from(index) * f64::from(self.scale) / f64::from(self.rate)
    }

    /// Length of the video in seconds.
    pub fn duration(&self) -> f64 {
        self.frame_time(self.frame_count)
    }
}
