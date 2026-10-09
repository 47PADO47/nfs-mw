//! How the application's inputs are laid out: fixed for the life of a context.

use wgpu::TextureFormat;

use crate::error::Fsr3Error;

/// How the depth buffer encodes distance.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct DepthConvention {
    /// The depth buffer is reversed: 1 at the near plane and 0 at the far plane.
    pub inverted: bool,
    /// The far plane is at infinity (the camera's `far` value is then ignored). Reverse-Z with an
    /// infinite far plane stores `near / z`.
    pub infinite: bool,
}

impl DepthConvention {
    /// Reverse-Z with an infinite far plane: `depth = near / view_z`. Bevy's and this repository's
    /// renderers' convention.
    pub const REVERSE_INFINITE: Self = Self { inverted: true, infinite: true };
    /// Reverse-Z with a finite far plane.
    pub const REVERSE: Self = Self { inverted: true, infinite: false };
    /// The classic 0 (near) to 1 (far) depth buffer.
    pub const STANDARD: Self = Self { inverted: false, infinite: false };
}

/// How the motion vector texture is laid out.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct MotionVectorLayout {
    /// The motion vectors are at the output resolution instead of the render resolution.
    pub display_resolution: bool,
    /// The motion vectors include the jitter of the current and previous frames, which the upscaler
    /// then cancels.
    pub jittered: bool,
}

/// Tuning knobs with AMD's defaults (`ffxFsr3UpscalerSetConstant`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Tuning {
    /// Scales how much fast motion lowers the history weight; `[0, 1]`. 0 can calm flickering bright
    /// pixels. Default 1.
    pub velocity_factor: f32,
    /// Scales the reactive mask at read time; `>= 0`. Default 1.
    pub reactiveness_scale: f32,
    /// Scales the computed shading-change signal at read time; `>= 0`. Default 1.
    pub shading_change_scale: f32,
    /// How much accumulation a pixel gains per frame; `[0, 1]`. Default 1/3.
    pub accumulation_added_per_frame: f32,
    /// The accumulation a disoccluded pixel starts from; `[-1, 1]`. Default -1/3.
    pub min_disocclusion_accumulation: f32,
}

impl Default for Tuning {
    fn default() -> Self {
        Self {
            velocity_factor: 1.0,
            reactiveness_scale: 1.0,
            shading_change_scale: 1.0,
            accumulation_added_per_frame: 1.0 / 3.0,
            min_disocclusion_accumulation: -1.0 / 3.0,
        }
    }
}

impl Tuning {
    /// The values clamped to the ranges AMD documents.
    pub(crate) fn clamped(self) -> Self {
        Self {
            velocity_factor: self.velocity_factor.clamp(0.0, 1.0),
            reactiveness_scale: self.reactiveness_scale.max(0.0),
            shading_change_scale: self.shading_change_scale.max(0.0),
            accumulation_added_per_frame: self.accumulation_added_per_frame.clamp(0.0, 1.0),
            min_disocclusion_accumulation: self.min_disocclusion_accumulation.clamp(-1.0, 1.0),
        }
    }
}

/// What a context is created for.
#[derive(Clone, Debug, PartialEq)]
pub struct Fsr3Config {
    /// The colour input holds HDR, scene-referred linear values (no tone mapping applied). Turn this off
    /// for linear LDR input in `[0, 1]`: the HDR path then skips the tone-mapped accumulation.
    pub hdr: bool,
    /// How the depth input encodes distance.
    pub depth: DepthConvention,
    /// The upscaler computes the exposure itself (from the average log luminance, like AMD's auto
    /// exposure); [`crate::Fsr3Inputs::exposure`] is then ignored. When off, the exposure input (or 1)
    /// is used.
    pub auto_exposure: bool,
    /// The layout of the motion vector input.
    pub motion_vectors: MotionVectorLayout,
    /// The format of the output texture: a storage-capable one ([`Fsr3Config::OUTPUT_FORMATS`]).
    pub output_format: TextureFormat,
    /// Tuning knobs.
    pub tuning: Tuning,
}

impl Fsr3Config {
    /// The formats the output texture can have: ones every backend lets a compute shader write.
    pub const OUTPUT_FORMATS: [TextureFormat; 3] =
        [TextureFormat::Rgba16Float, TextureFormat::Rgba32Float, TextureFormat::Rgba8Unorm];

    /// A configuration for HDR input with reverse infinite depth, render-resolution motion vectors, a
    /// fixed exposure and an `Rgba16Float` output.
    pub fn new() -> Self {
        Self {
            hdr: true,
            depth: DepthConvention::REVERSE_INFINITE,
            auto_exposure: false,
            motion_vectors: MotionVectorLayout::default(),
            output_format: TextureFormat::Rgba16Float,
            tuning: Tuning::default(),
        }
    }

    pub(crate) fn validate(&self) -> Result<(), Fsr3Error> {
        if !Self::OUTPUT_FORMATS.contains(&self.output_format) {
            return Err(Fsr3Error::UnsupportedOutputFormat(self.output_format));
        }
        Ok(())
    }
}

impl Default for Fsr3Config {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_is_valid_and_reverse_infinite() {
        let config = Fsr3Config::default();
        assert!(config.validate().is_ok());
        assert_eq!(config.depth, DepthConvention::REVERSE_INFINITE);
    }

    #[test]
    fn unsupported_output_formats_are_refused() {
        let config = Fsr3Config { output_format: TextureFormat::Rgba8UnormSrgb, ..Fsr3Config::new() };
        assert!(matches!(config.validate(), Err(Fsr3Error::UnsupportedOutputFormat(_))));
    }

    #[test]
    fn tuning_clamps_to_the_documented_ranges() {
        let t = Tuning {
            velocity_factor: 2.0,
            reactiveness_scale: -1.0,
            shading_change_scale: -3.0,
            accumulation_added_per_frame: 4.0,
            min_disocclusion_accumulation: -9.0,
        }
        .clamped();
        assert_eq!((t.velocity_factor, t.reactiveness_scale, t.shading_change_scale), (1.0, 0.0, 0.0));
        assert_eq!((t.accumulation_added_per_frame, t.min_disocclusion_accumulation), (1.0, -1.0));
    }
}
