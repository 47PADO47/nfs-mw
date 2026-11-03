//! The public type: a parsed track list and the ways to read samples out of it.

use crate::error::AclError;
use crate::header::{Header, RotationFormat, TransformHeader, VectorFormat};
use crate::layout::Layout;
use crate::qvv::Qvv;

/// A compressed list of transform tracks (one per bone), borrowed from the bytes it was parsed from.
///
/// ACL stores `num_samples` poses at `sample_rate` per second. A clip made to loop (`is_wrap_optimized`) leaves the
/// last pose out because it equals the first: [`Tracks::num_frames`] counts it back in, as the engines that import
/// ACL clips do.
#[derive(Clone, Debug)]
pub struct Tracks<'a> {
    pub(crate) bytes: &'a [u8],
    pub(crate) header: Header,
    pub(crate) layout: Layout,
}

/// Which two stored poses a time falls between.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Seek {
    pub segment: [usize; 2],
    /// Index among the poses stored in the segment.
    pub pose: [u32; 2],
    pub alpha: f32,
}

impl<'a> Tracks<'a> {
    /// Parses and checks the headers and the layout. The sample data is read when asked for.
    ///
    /// # Errors
    /// Fails when the buffer is not an ACL 2.1 transform track list or is damaged.
    pub fn parse(bytes: &'a [u8]) -> Result<Self, AclError> {
        let header = Header::parse(bytes)?;
        let transform = TransformHeader::parse(bytes)?;
        let layout = Layout::new(bytes, &header, &transform)?;
        Ok(Self { bytes, header, layout })
    }

    /// The revision of the buffer format (9 or 10: ACL 2.1).
    #[must_use]
    pub fn version(&self) -> u16 {
        self.header.version
    }

    /// How the clip stores its animated rotations.
    #[must_use]
    pub fn rotation_format(&self) -> RotationFormat {
        self.layout.rotation_format
    }

    /// How the clip stores its animated translations.
    #[must_use]
    pub fn translation_format(&self) -> VectorFormat {
        self.layout.translation_format
    }

    /// How the clip stores its animated scales.
    #[must_use]
    pub fn scale_format(&self) -> VectorFormat {
        self.layout.scale_format
    }

    /// The number of tracks (bones).
    #[must_use]
    pub fn num_tracks(&self) -> usize {
        self.header.num_tracks as usize
    }

    /// The poses the clip stores, not counting the repeated first pose of a looping clip.
    #[must_use]
    pub fn num_samples(&self) -> usize {
        self.header.num_samples as usize
    }

    /// The poses the clip plays: [`Self::num_samples`], plus the repeated first pose of a looping clip.
    #[must_use]
    pub fn num_frames(&self) -> usize {
        self.num_samples() + usize::from(self.is_wrap_optimized() && self.num_samples() > 0)
    }

    /// Poses per second.
    #[must_use]
    pub fn sample_rate(&self) -> f32 {
        self.header.sample_rate
    }

    /// Seconds from the first frame to the last.
    #[must_use]
    pub fn duration(&self) -> f32 {
        if self.num_frames() < 2 || self.sample_rate() <= 0.0 {
            return 0.0;
        }
        (self.num_frames() - 1) as f32 / self.sample_rate()
    }

    /// The clip loops: its last pose is its first and is not stored.
    #[must_use]
    pub fn is_wrap_optimized(&self) -> bool {
        self.header.is_wrap_optimized()
    }

    /// Some tracks carry a scale; otherwise every scale is [`Self::default_scale`].
    #[must_use]
    pub fn has_scale(&self) -> bool {
        self.header.has_scale()
    }

    /// The scale of a track without scale data: 0 for additive clips, 1 otherwise.
    #[must_use]
    pub fn default_scale(&self) -> f32 {
        self.header.default_scale()
    }

    /// Sets what [`Self::default_scale`] answers (the Unreal plugin forces 0 for additive animations).
    pub fn set_default_scale(&mut self, one: bool) {
        self.header.set_default_scale(one);
    }

    /// The clip's default sub-tracks stand for the identity. When false the caller must pass the skeleton's reference
    /// pose to the `decode_*` methods.
    #[must_use]
    pub fn has_trivial_defaults(&self) -> bool {
        self.header.has_trivial_defaults()
    }

    /// The number of segments the samples are split into.
    #[must_use]
    pub fn num_segments(&self) -> usize {
        self.layout.segments.len()
    }

    /// Whether some poses were left out and are rebuilt by interpolation.
    #[must_use]
    pub fn has_stripped_keyframes(&self) -> bool {
        self.layout.stripped
    }

    /// The pose at frame `frame` (`0..num_frames`), one transform per track.
    ///
    /// `defaults` gives, per track, the value of every sub-track the clip stores nothing for (the skeleton's
    /// reference pose); it may be `None` when [`Self::has_trivial_defaults`].
    ///
    /// # Errors
    /// Fails on a frame past the end, missing defaults or damaged data.
    pub fn decode_frame(&self, frame: usize, defaults: Option<&[Qvv]>) -> Result<Vec<Qvv>, AclError> {
        let frames = self.num_frames();
        if frame >= frames {
            return Err(AclError::OutOfRange { index: frame, count: frames });
        }
        let samples = self.header.num_samples;
        let key = if frame >= self.num_samples() { 0 } else { frame as u32 };
        // As ACL seeks to a sample time with the "nearest" rule: the pose after the wanted one is its neighbour.
        let next = match key + 1 {
            n if n < samples => n,
            _ if self.is_wrap_optimized() => 0,
            _ => key,
        };
        let seek = self.resolve(key, next, 0.0)?;
        self.decode_pose(&seek, defaults)
    }

    /// The pose at `time` seconds (clamped to the clip), interpolated between the two surrounding frames.
    ///
    /// # Errors
    /// As [`Self::decode_frame`].
    pub fn sample(&self, time: f32, defaults: Option<&[Qvv]>) -> Result<Vec<Qvv>, AclError> {
        let frames = self.num_frames() as u32;
        if frames == 0 {
            return Err(AclError::OutOfRange { index: 0, count: 0 });
        }
        let position = (time * self.sample_rate()).clamp(0.0, (frames - 1) as f32);
        let key0 = position as u32;
        let alpha = position - key0 as f32;
        let wrap = self.is_wrap_optimized();
        let samples = self.header.num_samples;
        let next = key0 + 1;
        let (key0, key1, alpha) = if key0 >= samples {
            (0, 0, 0.0)
        } else if next < samples {
            (key0, next, alpha)
        } else if wrap {
            (key0, 0, alpha)
        } else {
            (key0, key0, 0.0)
        };
        let seek = self.resolve(key0, key1, alpha)?;
        self.decode_pose(&seek, defaults)
    }

    /// Every frame of the clip, in order.
    ///
    /// # Errors
    /// As [`Self::decode_frame`].
    pub fn decode_all(&self, defaults: Option<&[Qvv]>) -> Result<Vec<Vec<Qvv>>, AclError> {
        (0..self.num_frames()).map(|f| self.decode_frame(f, defaults)).collect()
    }

    /// The segment that holds sample `key`.
    fn segment_of(&self, key: u32) -> usize {
        self.layout.segments.partition_point(|s| s.start <= key).saturating_sub(1)
    }

    /// Turns two sample numbers and a weight into the two stored poses to read, skipping the samples the encoder
    /// stripped (they are rebuilt by interpolating the stored neighbours).
    pub(crate) fn resolve(&self, key0: u32, key1: u32, alpha: f32) -> Result<Seek, AclError> {
        let segments = &self.layout.segments;
        let (s0, s1) = (self.segment_of(key0), self.segment_of(key1));
        let (mut local0, mut local1) = (key0 - segments[s0].start, key1 - segments[s1].start);
        if !self.layout.stripped {
            return Ok(Seek { segment: [s0, s1], pose: [local0, local1], alpha });
        }
        if local0 > 31 || local1 > 31 {
            return Err(AclError::Invalid("a segment with stripped keyframes has more than 32 samples"));
        }
        let (stored0, stored1) = (segments[s0].sample_indices, segments[s1].sample_indices);
        let below = stored0 & (u32::MAX << (31 - local0));
        let above = stored1 & (u32::MAX >> local1);
        if below == 0 || above == 0 {
            return Err(AclError::Invalid("stripped keyframes leave no neighbour"));
        }
        let position = alpha + key0 as f32;
        local0 = 31 - below.trailing_zeros();
        local1 = above.leading_zeros();
        let (first, second) = (segments[s0].start + local0, segments[s1].start + local1);
        let alpha = if first == second {
            0.0
        } else if first < second {
            (position - first as f32) / (second - first) as f32
        } else {
            // The second pose is the repeated first one of a looping clip, one frame after the last stored.
            (position - first as f32) / (self.header.num_samples - first) as f32
        };
        let before = |stored: u32, local: u32| (stored & !(u32::MAX >> local)).count_ones();
        Ok(Seek { segment: [s0, s1], pose: [before(stored0, local0), before(stored1, local1)], alpha })
    }
}
