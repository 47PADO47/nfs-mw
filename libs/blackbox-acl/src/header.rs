//! The two fixed headers of a compressed transform track list.
//!
//! ```text
//!  0  u32 size of the whole buffer          8  u32 tag 0xac11ac11
//!  4  u32 hash                              12 u16 version, u8 algorithm, u8 track type
//! 16  u32 track count, u32 sample count, f32 sample rate, u32 flags       (the transform header follows at 32)
//! ```

use crate::bits::{f32_at, u8_at, u16_at, u32_at};
use crate::error::AclError;

/// Tag of a compressed track list.
const TAG: u32 = 0xac11_ac11;
/// The track type of transform (rotation, translation, scale) tracks.
const TRACK_TYPE_TRANSFORM: u8 = 12;
/// Where the transform header starts.
pub(crate) const TRANSFORM_HEADER: usize = 32;
/// Size of the transform header; the segment start indices of a multi-segment clip follow it.
pub(crate) const TRANSFORM_HEADER_SIZE: usize = 52;
/// An offset field that points nowhere.
const NO_OFFSET: u32 = u32::MAX;

/// How a rotation is stored.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RotationFormat {
    /// Four floats.
    Full,
    /// Three floats, `w` rebuilt.
    DropWFull,
    /// Three quantised components per sample, the bit count chosen per track, `w` rebuilt.
    DropWVariable,
}

/// How a translation or a scale is stored.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VectorFormat {
    /// Three floats.
    Full,
    /// Three quantised components per sample, the bit count chosen per track.
    Variable,
}

/// The first header.
#[derive(Clone, Debug)]
pub(crate) struct Header {
    pub version: u16,
    pub num_tracks: u32,
    pub num_samples: u32,
    pub sample_rate: f32,
    misc: u32,
}

impl Header {
    pub fn parse(bytes: &[u8]) -> Result<Self, AclError> {
        let size = u32_at(bytes, 0)?;
        let tag = u32_at(bytes, 8)?;
        if tag != TAG {
            return Err(AclError::BadTag(tag));
        }
        let version = u16_at(bytes, 12)?;
        if !(9..=10).contains(&version) {
            return Err(AclError::BadVersion(version));
        }
        let algorithm = u8_at(bytes, 14)?;
        let track_type = u8_at(bytes, 15)?;
        if algorithm != 0 || track_type != TRACK_TYPE_TRANSFORM {
            return Err(AclError::BadKind { algorithm, track_type });
        }
        if size as usize > bytes.len() {
            return Err(AclError::Truncated(bytes.len()));
        }
        let header = Self {
            version,
            num_tracks: u32_at(bytes, 16)?,
            num_samples: u32_at(bytes, 20)?,
            sample_rate: f32_at(bytes, 24)?,
            misc: u32_at(bytes, 28)?,
        };
        if header.has_database() {
            return Err(AclError::Unsupported("clips split into a database"));
        }
        Ok(header)
    }

    pub fn has_scale(&self) -> bool {
        self.misc & 1 != 0
    }

    /// The scale a track has without data: 0 for additive clips, 1 otherwise.
    pub fn default_scale(&self) -> f32 {
        ((self.misc >> 1) & 1) as f32
    }

    pub fn scale_format(&self) -> VectorFormat {
        vector_format((self.misc >> 2) & 1)
    }

    pub fn translation_format(&self) -> VectorFormat {
        vector_format((self.misc >> 3) & 1)
    }

    pub fn rotation_format(&self) -> Result<RotationFormat, AclError> {
        match (self.misc >> 4) & 15 {
            0 => Ok(RotationFormat::Full),
            2 => Ok(RotationFormat::DropWFull),
            3 => Ok(RotationFormat::DropWVariable),
            _ => Err(AclError::Invalid("unknown rotation format")),
        }
    }

    fn has_database(&self) -> bool {
        self.misc & (1 << 8) != 0
    }

    /// Default sub-tracks are the identity (or zero) and need no skeleton.
    pub fn has_trivial_defaults(&self) -> bool {
        self.misc & (1 << 9) != 0
    }

    pub fn has_stripped_keyframes(&self) -> bool {
        self.misc & (1 << 10) != 0
    }

    /// The last sample is not stored because it repeats the first.
    pub fn is_wrap_optimized(&self) -> bool {
        self.misc & (1 << 30) != 0
    }

    /// Overrides the default scale bit (the Unreal plugin treats additive clips as having 0).
    pub fn set_default_scale(&mut self, one: bool) {
        self.misc = (self.misc & !(1 << 1)) | (u32::from(one) << 1);
    }
}

fn vector_format(bit: u32) -> VectorFormat {
    if bit == 0 { VectorFormat::Full } else { VectorFormat::Variable }
}

/// The second header: counts and the offsets of the blocks, relative to its own start.
#[derive(Clone, Debug)]
pub(crate) struct TransformHeader {
    pub num_segments: u32,
    pub num_animated_variable: u32,
    pub num_animated_rotation: u32,
    pub num_animated_translation: u32,
    pub num_animated_scale: u32,
    pub num_constant_rotation: u32,
    pub num_constant_translation: u32,
    pub num_constant_scale: u32,
    pub segment_headers: u32,
    pub sub_track_types: u32,
    pub constant_data: u32,
    pub clip_range: u32,
}

impl TransformHeader {
    pub fn parse(bytes: &[u8]) -> Result<Self, AclError> {
        let word = |i: usize| u32_at(bytes, TRANSFORM_HEADER + i * 4);
        let header = Self {
            num_segments: word(0)?,
            num_animated_variable: word(1)?,
            num_animated_rotation: word(2)?,
            num_animated_translation: word(3)?,
            num_animated_scale: word(4)?,
            num_constant_rotation: word(5)?,
            num_constant_translation: word(6)?,
            num_constant_scale: word(7)?,
            segment_headers: word(9)?,
            sub_track_types: word(10)?,
            constant_data: word(11)?,
            clip_range: word(12)?,
        };
        if word(8)? != NO_OFFSET {
            return Err(AclError::Unsupported("clips split into a database"));
        }
        if header.num_segments == 0 || header.num_segments > 4096 {
            return Err(AclError::Invalid("segment count"));
        }
        Ok(header)
    }
}
