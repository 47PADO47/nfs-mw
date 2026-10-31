//! Reading the values: constants, quantised animated data, range expansion and interpolation.
//!
//! Animated data is a bit stream per stored pose, most significant bit first: every animated rotation (three components
//! of its own bit count, or 32 bits each when the rate is [`RAW_BITS`]), then every animated translation, then every
//! scale. A quantised component `q` of `n` bits is `q / (2^n - 1)` in `0..=1` and is expanded by the segment range
//! (one byte per component, only with several segments) and then by the clip range (floats).

use crate::bits::{f32_at, read_raw_f32, read_unorm, u8_at, u16_at};
use crate::error::AclError;
use crate::header::{RotationFormat, VectorFormat};
use crate::layout::{RAW_BITS, Segment, Sub};
use crate::qvv::{self, Qvv};
use crate::tracks::{Seek, Tracks};

/// Which vector kind a function reads.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Translation = 1,
    Scale = 2,
}

fn read_f32s<const N: usize>(bytes: &[u8], at: usize) -> Result<[f32; N], AclError> {
    let mut out = [0.0; N];
    for (i, v) in out.iter_mut().enumerate() {
        *v = f32_at(bytes, at + 4 * i)?;
    }
    Ok(out)
}

/// Three consecutive `bits`-bit components as floats in `0..=1`.
fn unorm3(bytes: &[u8], bit: usize, bits: u32) -> Result<[f32; 3], AclError> {
    let step = bits as usize;
    Ok([read_unorm(bytes, bit, bits)?, read_unorm(bytes, bit + step, bits)?, read_unorm(bytes, bit + 2 * step, bits)?])
}

fn mul_add(v: [f32; 3], extent: [f32; 3], min: [f32; 3]) -> [f32; 3] {
    [v[0] * extent[0] + min[0], v[1] * extent[1] + min[1], v[2] * extent[2] + min[2]]
}

impl Tracks<'_> {
    /// All tracks at the two poses `seek` names, blended by its weight.
    pub(crate) fn decode_pose(&self, seek: &Seek, defaults: Option<&[Qvv]>) -> Result<Vec<Qvv>, AclError> {
        let count = self.num_tracks();
        if defaults.is_some_and(|d| d.len() < count) {
            return Err(AclError::MissingDefaults(count));
        }
        (0..count).map(|track| self.decode_track(track, seek, defaults.map(|d| d[track]))).collect()
    }

    fn decode_track(&self, track: usize, seek: &Seek, default: Option<Qvv>) -> Result<Qvv, AclError> {
        let layout = &self.layout;
        let need_default = || -> Result<Qvv, AclError> {
            if default.is_none() && !self.has_trivial_defaults() {
                return Err(AclError::MissingDefaults(self.num_tracks()));
            }
            Ok(default.unwrap_or(Qvv::IDENTITY))
        };
        let rotation = match layout.rotation[track] {
            Sub::Default => need_default()?.rotation,
            Sub::Constant(i) => self.constant_rotation(i)?,
            Sub::Animated(j) => self.animated_rotation(seek, j)?,
        };
        let translation = match layout.translation[track] {
            Sub::Default => need_default()?.translation,
            Sub::Constant(i) => read_f32s(self.bytes, layout.constant_translation + 12 * i as usize)?,
            Sub::Animated(j) => self.animated_vector(Kind::Translation, seek, j)?,
        };
        let scale = match layout.scale.get(track).copied().unwrap_or(Sub::Default) {
            Sub::Default if !self.has_scale() => [self.default_scale(); 3],
            Sub::Default => match default {
                Some(d) => d.scale,
                None if self.has_trivial_defaults() => [self.default_scale(); 3],
                None => return Err(AclError::MissingDefaults(self.num_tracks())),
            },
            Sub::Constant(i) => read_f32s(self.bytes, layout.constant_scale + 12 * i as usize)?,
            Sub::Animated(j) => self.animated_vector(Kind::Scale, seek, j)?,
        };
        Ok(Qvv { rotation, translation, scale })
    }

    /// A rotation constant over the clip. Constants are stored four to a group, component by component.
    fn constant_rotation(&self, index: u32) -> Result<[f32; 4], AclError> {
        let layout = &self.layout;
        if layout.rotation_format == RotationFormat::Full {
            return read_f32s(self.bytes, layout.constant_rotation + 16 * index as usize);
        }
        let group = (index / 4) as usize;
        let within = (index % 4) as usize;
        let size = (layout.constant_rotation_count as usize - group * 4).min(4);
        let base = layout.constant_rotation + group * 48;
        let component = |k: usize| f32_at(self.bytes, base + 4 * (k * size + within));
        let q = qvv::from_positive_w([component(0)?, component(1)?, component(2)?]);
        Ok(qvv::normalize(q))
    }

    fn animated_rotation(&self, seek: &Seek, index: u32) -> Result<[f32; 4], AclError> {
        let first = self.rotation_at_pose(&self.layout.segments[seek.segment[0]], index, seek.pose[0])?;
        let same = seek.segment[0] == seek.segment[1] && seek.pose[0] == seek.pose[1];
        if same || seek.alpha <= 0.0 {
            return Ok(qvv::normalize(first));
        }
        let second = self.rotation_at_pose(&self.layout.segments[seek.segment[1]], index, seek.pose[1])?;
        Ok(qvv::lerp(first, second, seek.alpha))
    }

    fn animated_vector(&self, kind: Kind, seek: &Seek, index: u32) -> Result<[f32; 3], AclError> {
        let first = self.vector_at_pose(kind, &self.layout.segments[seek.segment[0]], index, seek.pose[0])?;
        let same = seek.segment[0] == seek.segment[1] && seek.pose[0] == seek.pose[1];
        if same || seek.alpha <= 0.0 {
            return Ok(first);
        }
        let second = self.vector_at_pose(kind, &self.layout.segments[seek.segment[1]], index, seek.pose[1])?;
        Ok(qvv::lerp3(first, second, seek.alpha))
    }

    /// One animated rotation at one stored pose, not yet normalised for the formats that rebuild `w`.
    fn rotation_at_pose(&self, seg: &Segment, index: u32, pose: u32) -> Result<[f32; 4], AclError> {
        let pose_bit = seg.poses * 8 + pose as usize * seg.pose_bits as usize;
        let bit = pose_bit + seg.rotation_at[index as usize] as usize;
        match self.layout.rotation_format {
            RotationFormat::Full => {
                let mut q = [0.0; 4];
                for (k, v) in q.iter_mut().enumerate() {
                    *v = read_raw_f32(self.bytes, bit + 32 * k)?;
                }
                Ok(q)
            }
            RotationFormat::DropWFull => {
                let v = [
                    read_raw_f32(self.bytes, bit)?,
                    read_raw_f32(self.bytes, bit + 32)?,
                    read_raw_f32(self.bytes, bit + 64)?,
                ];
                Ok(qvv::normalize(qvv::from_positive_w(v)))
            }
            RotationFormat::DropWVariable => {
                let xyz = self.variable_rotation_xyz(seg, index, bit)?;
                Ok(qvv::normalize(qvv::from_positive_w(xyz)))
            }
        }
    }

    /// The `x, y, z` of a quantised rotation after both range expansions.
    fn variable_rotation_xyz(&self, seg: &Segment, index: u32, bit: usize) -> Result<[f32; 3], AclError> {
        let layout = &self.layout;
        let bytes = self.bytes;
        let bits = u8_at(bytes, seg.format[0] + index as usize)?;
        let multiple = layout.segments.len() > 1;
        let group = (index / 4) as usize;
        let within = (index % 4) as usize;
        let size = (layout.animated_rotation as usize - group * 4).min(4);
        let range = seg.range[0] + group * 24 + within;
        let (mut v, segment_range) = match bits {
            RAW_BITS => {
                return Ok([read_raw_f32(bytes, bit)?, read_raw_f32(bytes, bit + 32)?, read_raw_f32(bytes, bit + 64)?]);
            }
            0 => {
                if !multiple {
                    return Err(AclError::Invalid("a constant bit rate without segment ranges"));
                }
                let pair = |at: usize| -> Result<f32, AclError> {
                    Ok(((u32::from(u8_at(bytes, at)?) << 8) | u32::from(u8_at(bytes, at + 4)?)) as f32)
                };
                let v = [pair(range)?, pair(range + 8)?, pair(range + 16)?];
                (v.map(|c| c * (1.0 / 65535.0)), false)
            }
            n => (unorm3(bytes, bit, u32::from(n))?, multiple),
        };
        if segment_range {
            let byte =
                |k: usize| -> Result<f32, AclError> { Ok(f32::from(u8_at(bytes, range + 4 * k)?) * (1.0 / 255.0)) };
            let min = [byte(0)?, byte(1)?, byte(2)?];
            let extent = [byte(3)?, byte(4)?, byte(5)?];
            v = mul_add(v, extent, min);
        }
        let base = layout.clip_rotation + group * 96;
        let float = |k: usize| f32_at(bytes, base + 4 * (k * size + within));
        let min = [float(0)?, float(1)?, float(2)?];
        let extent = [float(3)?, float(4)?, float(5)?];
        Ok(mul_add(v, extent, min))
    }

    /// One animated translation or scale at one stored pose.
    fn vector_at_pose(&self, kind: Kind, seg: &Segment, index: u32, pose: u32) -> Result<[f32; 3], AclError> {
        let layout = &self.layout;
        let bytes = self.bytes;
        let (format, table, clip, start) = match kind {
            Kind::Translation => {
                (layout.translation_format, &seg.translation_at, layout.clip_translation, seg.rotation_bits)
            }
            Kind::Scale => {
                (layout.scale_format, &seg.scale_at, layout.clip_scale, seg.rotation_bits + seg.translation_bits)
            }
        };
        let bit =
            seg.poses * 8 + pose as usize * seg.pose_bits as usize + start as usize + table[index as usize] as usize;
        let raw = |at: usize| -> Result<[f32; 3], AclError> {
            Ok([read_raw_f32(bytes, at)?, read_raw_f32(bytes, at + 32)?, read_raw_f32(bytes, at + 64)?])
        };
        if format == VectorFormat::Full {
            return raw(bit);
        }
        let bits = u8_at(bytes, seg.format[kind as usize] + index as usize)?;
        let multiple = layout.segments.len() > 1;
        let range = seg.range[kind as usize] + 6 * index as usize;
        let (mut v, segment_range) = match bits {
            RAW_BITS => return raw(bit),
            0 => {
                if !multiple {
                    return Err(AclError::Invalid("a constant bit rate without segment ranges"));
                }
                let v = [u16_at(bytes, range)?, u16_at(bytes, range + 2)?, u16_at(bytes, range + 4)?];
                (v.map(|c| f32::from(c) * (1.0 / 65535.0)), false)
            }
            n => (unorm3(bytes, bit, u32::from(n))?, multiple),
        };
        if segment_range {
            let byte = |k: usize| -> Result<f32, AclError> { Ok(f32::from(u8_at(bytes, range + k)?) * (1.0 / 255.0)) };
            let min = [byte(0)?, byte(1)?, byte(2)?];
            let extent = [byte(3)?, byte(4)?, byte(5)?];
            v = mul_add(v, extent, min);
        }
        let base = clip + 24 * index as usize;
        Ok(mul_add(v, read_f32s(bytes, base + 12)?, read_f32s(bytes, base)?))
    }
}
