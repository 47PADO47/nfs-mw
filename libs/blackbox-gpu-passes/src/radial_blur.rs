//! Radial blur: a fullscreen filter that smears the image towards a centre point, for the speed and pursuit
//! effect. Engine-generic: the caller gives the centre, the strength, the radius where the blur starts and the
//! number of samples.
//!
//! A pixel is averaged over `taps` samples on the segment from the pixel to the centre. The first sample is the
//! pixel itself; the sample at position `t` (0 at the pixel, 1 at the far end) weighs `1 - t / 2`, so the pixel
//! counts most. The reach of the segment grows from 0 at `inner` (a distance from the centre, in UV) to `strength`
//! at the corners, so the middle of the frame stays sharp.

use crate::{Blend, Draw, Filter, Params, filter_source};

/// The most samples a pixel takes. The shader's loop stops here whatever the caller asks for.
pub const RADIAL_MAX_TAPS: u32 = 32;

/// The blur's settings. Start from [`Default`] and change what the caller needs.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RadialBlurSettings {
    /// How far the far samples reach towards the centre: 0 draws nothing, 1 reaches the centre.
    pub strength: f32,
    /// The centre of the blur in UV (0 to 1 on both axes).
    pub centre: [f32; 2],
    /// The distance from the centre, in UV, inside which the image is not blurred (0 to 0.99).
    pub inner: f32,
    /// Samples per pixel (1 to [`RADIAL_MAX_TAPS`]).
    pub taps: u32,
}

impl Default for RadialBlurSettings {
    fn default() -> Self {
        Self { strength: 0.0, centre: [0.5, 0.5], inner: 0.25, taps: 12 }
    }
}

impl RadialBlurSettings {
    /// The settings with every value in its range. A non-finite value takes the default's.
    pub fn sanitized(self) -> Self {
        let default = Self::default();
        let finite = |v: f32, fallback: f32| match v.is_finite() {
            true => v,
            false => fallback,
        };
        Self {
            strength: finite(self.strength, default.strength).clamp(0.0, 1.0),
            centre: [finite(self.centre[0], 0.5).clamp(0.0, 1.0), finite(self.centre[1], 0.5).clamp(0.0, 1.0)],
            inner: finite(self.inner, default.inner).clamp(0.0, 0.99),
            taps: self.taps.clamp(1, RADIAL_MAX_TAPS),
        }
    }

    /// Whether the blur changes nothing once sanitised, so it need not be drawn at all.
    pub fn is_identity(self) -> bool {
        self.sanitized().strength <= 0.0
    }
}

/// The position of sample `index` of `taps` along the segment: 0 at the pixel, 1 at the far end.
pub fn tap_position(index: u32, taps: u32) -> f32 {
    let span = taps.max(2) - 1;
    index as f32 / span as f32
}

/// The unnormalised weight of sample `index` of `taps`: `1 - t / 2` at position `t`.
pub fn tap_weight(index: u32, taps: u32) -> f32 {
    1.0 - 0.5 * tap_position(index, taps)
}

/// The parameter block the shader reads for `settings` (sanitised first). The normaliser is the reciprocal of the
/// sum of the tap weights, so the weights of the samples add up to one.
pub fn params(settings: RadialBlurSettings) -> Params {
    let s = settings.sanitized();
    let total: f32 = (0..s.taps).map(|i| tap_weight(i, s.taps)).sum();
    Params { a: [s.strength, s.inner, s.taps as f32, 1.0 / total], b: [s.centre[0], s.centre[1], 0.0, 0.0] }
}

/// How far the segment of a pixel at `uv` reaches towards the centre, from 0 to the strength. CPU mirror of the
/// shader, for the tests.
#[cfg(test)]
pub fn reach(settings: RadialBlurSettings, uv: [f32; 2]) -> f32 {
    let s = settings.sanitized();
    let distance = ((uv[0] - s.centre[0]).powi(2) + (uv[1] - s.centre[1]).powi(2)).sqrt();
    s.strength * ((distance - s.inner) / (1.0 - s.inner)).clamp(0.0, 1.0)
}

/// The images of one blur draw.
pub struct RadialBlurIo<'a> {
    pub input: &'a wgpu::TextureView,
    pub output: &'a wgpu::TextureView,
    pub output_format: wgpu::TextureFormat,
}

/// The part of the input, from its top-left corner, that holds the image, on each axis: 1 for an input that is the
/// image. A value that is not in `(0, 1]` counts as 1.
fn content_scale(scale: [f32; 2]) -> [f32; 2] {
    scale.map(|v| match v.is_finite() && v > 0.0 {
        true => v.min(1.0),
        false => 1.0,
    })
}

/// The radial blur pass: a pipeline per output format, built the first time it draws into one.
pub struct RadialBlur {
    filter: Filter,
    settings: RadialBlurSettings,
    content: [f32; 2],
}

impl RadialBlur {
    pub fn new(device: &wgpu::Device) -> Self {
        let source = filter_source(include_str!("shaders/radial_blur.wgsl"));
        Self {
            filter: Filter::new(device, "radial blur", &source),
            settings: RadialBlurSettings::default(),
            content: [1.0; 2],
        }
    }

    /// Say that the input holds the image only in the top-left part of it (a scene drawn at a render scale into a
    /// texture of the output's size): the centre, the reach and the samples are then those of the image, not of the
    /// whole texture. The default is the whole texture.
    pub fn set_content_scale(&mut self, scale: [f32; 2]) {
        self.content = content_scale(scale);
    }

    /// Replace the settings (sanitised). Takes effect on the next [`Self::encode`].
    pub fn set(&mut self, settings: RadialBlurSettings) {
        self.settings = settings.sanitized();
    }

    /// The settings in force, after sanitising.
    pub fn settings(&self) -> RadialBlurSettings {
        self.settings
    }

    /// Record the blur of `io.input` into `io.output`. A zero strength records nothing and leaves the output as it
    /// was, so a caller that wants no blur must take the pass out of its chain instead.
    pub fn encode(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        io: &RadialBlurIo<'_>,
    ) {
        if self.settings.is_identity() {
            return;
        }
        let mut block = params(self.settings);
        block.b[2..4].copy_from_slice(&self.content);
        self.filter.set_params(queue, &block);
        self.filter.draw(
            device,
            encoder,
            &Draw {
                entry: "fs_radial",
                src: io.input,
                src2: io.input,
                target: io.output,
                format: io.output_format,
                blend: Blend::Replace,
            },
        );
    }
}

#[cfg(test)]
mod content_tests {
    use super::content_scale;

    #[test]
    fn the_content_part_is_kept_in_range_and_defaults_to_the_whole_texture() {
        assert_eq!(content_scale([1.0, 1.0]), [1.0, 1.0]);
        assert_eq!(content_scale([0.5, 0.67]), [0.5, 0.67]);
        assert_eq!(content_scale([2.0, 0.0]), [1.0, 1.0]);
        assert_eq!(content_scale([f32::NAN, -1.0]), [1.0, 1.0]);
    }
}
