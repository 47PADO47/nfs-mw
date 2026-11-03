//! Where everything is: the sub-track tables, the constant and animated blocks, and the segments.
//!
//! The buffer, after the two headers (offsets relative to the start of the transform header, byte 32):
//!
//! ```text
//! [segment start indices u32 x n]         only when there is more than one segment
//! [segment headers]                       16 bytes each, 20 with stripped keyframes
//! [sub-track types]                       2 bits per track and kind, 16 tracks per u32, MSB first:
//!                                         all rotation words, then all translation words, then all scale words
//!                                         (0 default, 1 constant, 2 animated)
//! [constant data]                         rotations, then translations, then scales
//! [clip range data]                       min and extent of every animated variable rotation, translation, scale
//! per segment: [bit rate byte per animated variable track] [range data, only with several segments] [poses]
//! ```

use crate::bits::{u8_at, u32_at};
use crate::error::AclError;
use crate::header::{Header, RotationFormat, TRANSFORM_HEADER, TRANSFORM_HEADER_SIZE, TransformHeader, VectorFormat};

/// What one sub-track (the rotation, translation or scale of one track) is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Sub {
    /// No data: the skeleton's reference value.
    Default,
    /// One value for the whole clip; the number is its place among the constants of that kind.
    Constant(u32),
    /// A value per sample; the number is its place among the animated sub-tracks of that kind.
    Animated(u32),
}

/// A bit rate byte that stands for uncompressed floats (revision 9 and later).
pub(crate) const RAW_BITS: u8 = 31;
/// The widest quantised component.
const MAX_BITS: u8 = 23;

/// One segment: a run of samples that share their quantisation ranges.
#[derive(Clone, Debug)]
pub(crate) struct Segment {
    /// First sample of the clip in this segment.
    pub start: u32,
    /// Bits of one stored pose, and of its rotation part and translation part.
    pub pose_bits: u32,
    pub rotation_bits: u32,
    pub translation_bits: u32,
    /// With stripped keyframes: bit 31 - n set means the segment's sample n is stored. Otherwise all ones.
    pub sample_indices: u32,
    /// Absolute byte offsets of the bit rate bytes of the rotations, translations and scales.
    pub format: [usize; 3],
    /// Absolute byte offsets of the segment range data (six bytes per animated variable track) of the three kinds.
    pub range: [usize; 3],
    /// Absolute byte offset of the first stored pose.
    pub poses: usize,
    /// Bit offset inside a pose of each animated rotation, translation and scale sub-track.
    pub rotation_at: Vec<u32>,
    pub translation_at: Vec<u32>,
    pub scale_at: Vec<u32>,
}

/// The parsed shape of a track list.
#[derive(Clone, Debug)]
pub(crate) struct Layout {
    pub rotation: Vec<Sub>,
    pub translation: Vec<Sub>,
    pub scale: Vec<Sub>,
    pub rotation_format: RotationFormat,
    pub translation_format: VectorFormat,
    pub scale_format: VectorFormat,
    pub animated_rotation: u32,
    pub animated_translation: u32,
    pub animated_scale: u32,
    pub constant_rotation: usize,
    pub constant_rotation_count: u32,
    pub constant_translation: usize,
    pub constant_scale: usize,
    pub clip_rotation: usize,
    pub clip_translation: usize,
    pub clip_scale: usize,
    pub segments: Vec<Segment>,
    pub stripped: bool,
}

fn sub_tracks(bytes: &[u8], first_word: usize, count: usize) -> Result<Vec<Sub>, AclError> {
    let (mut constants, mut animated) = (0u32, 0u32);
    let mut out = Vec::with_capacity(count);
    for track in 0..count {
        let word = u32_at(bytes, first_word + (track / 16) * 4)?;
        let kind = (word >> ((15 - track % 16) * 2)) & 3;
        out.push(match kind {
            0 => Sub::Default,
            1 => {
                constants += 1;
                Sub::Constant(constants - 1)
            }
            2 => {
                animated += 1;
                Sub::Animated(animated - 1)
            }
            _ => return Err(AclError::Invalid("sub-track type")),
        });
    }
    Ok(out)
}

fn counts(subs: &[Sub]) -> (u32, u32) {
    let constants = subs.iter().filter(|s| matches!(s, Sub::Constant(_))).count();
    let animated = subs.iter().filter(|s| matches!(s, Sub::Animated(_))).count();
    (constants as u32, animated as u32)
}

fn align(offset: usize, to: usize) -> usize {
    offset.div_ceil(to) * to
}

/// The bit offset of each sub-track inside a pose, given the bit-rate bytes (`None` for fixed-size floats).
fn bit_offsets(bytes: &[u8], format: Option<usize>, count: u32, float_bits: u32) -> Result<Vec<u32>, AclError> {
    let mut out = Vec::with_capacity(count as usize);
    let mut at = 0u32;
    for j in 0..count {
        out.push(at);
        let Some(format) = format else {
            at += float_bits;
            continue;
        };
        let bits = u8_at(bytes, format + j as usize)?;
        at += 3 * match bits {
            RAW_BITS => 32,
            b if b <= MAX_BITS => u32::from(b),
            _ => return Err(AclError::Invalid("bit rate")),
        };
    }
    Ok(out)
}

impl Layout {
    pub fn new(bytes: &[u8], header: &Header, th: &TransformHeader) -> Result<Self, AclError> {
        if header.num_tracks == 0 || header.num_tracks > 1 << 16 || header.num_samples > 1 << 24 {
            return Err(AclError::Invalid("track or sample count"));
        }
        let tracks = header.num_tracks as usize;
        let words = tracks.div_ceil(16);
        let types = TRANSFORM_HEADER + th.sub_track_types as usize;
        let rotation = sub_tracks(bytes, types, tracks)?;
        let translation = sub_tracks(bytes, types + words * 4, tracks)?;
        let scale = if header.has_scale() { sub_tracks(bytes, types + 2 * words * 4, tracks)? } else { Vec::new() };
        let (constant_rotation_count, animated_rotation) = counts(&rotation);
        let (constant_translation_count, animated_translation) = counts(&translation);
        let (constant_scale_count, animated_scale) = counts(&scale);
        let matches_header = constant_rotation_count == th.num_constant_rotation
            && constant_translation_count == th.num_constant_translation
            && constant_scale_count == th.num_constant_scale
            && animated_rotation == th.num_animated_rotation
            && animated_translation == th.num_animated_translation
            && animated_scale == th.num_animated_scale;
        if !matches_header {
            return Err(AclError::Invalid("sub-track counts disagree with the header"));
        }
        let rotation_format = header.rotation_format()?;
        let rotation_size = if rotation_format == RotationFormat::Full { 16 } else { 12 };
        let constant_rotation = TRANSFORM_HEADER + th.constant_data as usize;
        let constant_translation = constant_rotation + rotation_size * constant_rotation_count as usize;
        let constant_scale = constant_translation + 12 * constant_translation_count as usize;
        let clip = TRANSFORM_HEADER + th.clip_range as usize;
        let translation_format = header.translation_format();
        let rotation_variable = rotation_format == RotationFormat::DropWVariable;
        let clip_translation = clip + if rotation_variable { 24 * animated_rotation as usize } else { 0 };
        let clip_scale = clip_translation
            + if translation_format == VectorFormat::Variable { 24 * animated_translation as usize } else { 0 };
        let mut layout = Self {
            rotation,
            translation,
            scale,
            rotation_format,
            translation_format,
            scale_format: header.scale_format(),
            animated_rotation,
            animated_translation,
            animated_scale,
            constant_rotation,
            constant_rotation_count,
            constant_translation,
            constant_scale,
            clip_rotation: clip,
            clip_translation,
            clip_scale,
            segments: Vec::new(),
            stripped: header.has_stripped_keyframes(),
        };
        layout.segments = layout.read_segments(bytes, header, th)?;
        Ok(layout)
    }

    fn read_segments(&self, bytes: &[u8], header: &Header, th: &TransformHeader) -> Result<Vec<Segment>, AclError> {
        let count = th.num_segments as usize;
        let stride = if self.stripped { 20 } else { 16 };
        let headers = TRANSFORM_HEADER + th.segment_headers as usize;
        let multiple = count > 1;
        let rotation_variable = self.rotation_format == RotationFormat::DropWVariable;
        let translation_variable = self.translation_format == VectorFormat::Variable;
        let scale_variable = self.scale_format == VectorFormat::Variable;
        let rotation_padded = align(self.animated_rotation as usize, 4);
        let mut out: Vec<Segment> = Vec::with_capacity(count);
        for s in 0..count {
            let at = headers + s * stride;
            let start = if multiple { u32_at(bytes, TRANSFORM_HEADER + TRANSFORM_HEADER_SIZE + s * 4)? } else { 0 };
            if start >= header.num_samples.max(1) || out.last().is_some_and(|p| p.start >= start) {
                return Err(AclError::Invalid("segment start indices"));
            }
            let data = TRANSFORM_HEADER + u32_at(bytes, at + 12)? as usize;
            let sample_indices = if self.stripped { u32_at(bytes, at + 16)? } else { u32::MAX };
            let fmt_translation = data + if rotation_variable { rotation_padded } else { 0 };
            let fmt_scale = fmt_translation + if translation_variable { self.animated_translation as usize } else { 0 };
            let range = align(data + th.num_animated_variable as usize, 2);
            let range_translation = range + if rotation_variable { 6 * rotation_padded } else { 0 };
            let range_scale =
                range_translation + if translation_variable { 6 * self.animated_translation as usize } else { 0 };
            let range_size = if multiple { 6 * th.num_animated_variable as usize } else { 0 };
            let rotation_float_bits = if self.rotation_format == RotationFormat::Full { 128 } else { 96 };
            out.push(Segment {
                start,
                pose_bits: u32_at(bytes, at)?,
                rotation_bits: u32_at(bytes, at + 4)?,
                translation_bits: u32_at(bytes, at + 8)?,
                sample_indices,
                format: [data, fmt_translation, fmt_scale],
                range: [range, range_translation, range_scale],
                poses: align(range + range_size, 4),
                rotation_at: bit_offsets(
                    bytes,
                    rotation_variable.then_some(data),
                    self.animated_rotation,
                    rotation_float_bits,
                )?,
                translation_at: bit_offsets(
                    bytes,
                    translation_variable.then_some(fmt_translation),
                    self.animated_translation,
                    96,
                )?,
                scale_at: bit_offsets(bytes, scale_variable.then_some(fmt_scale), self.animated_scale, 96)?,
            });
        }
        Ok(out)
    }
}
