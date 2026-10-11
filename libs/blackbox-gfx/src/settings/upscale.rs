//! Upscaling settings: how the scene drawn at a reduced render scale is brought back to the output size.

use super::render_scale::{DEFAULT_RENDER_SCALE, clamp_render_scale};

/// The filter that stretches the scene to the output when it is drawn below the output size
/// (a render scale under 1.0). At a render scale of 1.0 or more no spatial upscaler runs and the scene
/// is copied (or, when supersampled, filtered down) by the final resolve pass.
///
/// Which upscalers exist depends on the renderer (its `Capabilities::upscalers`): the native renderer
/// has [`Off`](Self::Off), [`Bilinear`](Self::Bilinear) and [`Fsr1`](Self::Fsr1).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum Upscaler {
    /// No upscaling: the scene is drawn at the output size, whatever the render scale says.
    Off,
    /// Plain bilinear filtering in the resolve pass: soft, and the cheapest.
    #[default]
    Bilinear,
    /// AMD FidelityFX Super Resolution 1: an edge-adaptive spatial upscale (EASU) followed by a
    /// contrast-adaptive sharpening pass (RCAS). It expects an anti-aliased, display-referred input.
    Fsr1,
    /// AMD FSR 3 upscaler: temporal, any GPU. The [`UpscaleQuality`] mode sets the render size.
    Fsr3,
    /// AMD FSR 4: a machine-learning temporal upscaler on recent Radeon GPUs under Direct3D 12.
    Fsr4,
    /// NVIDIA DLSS Super Resolution: temporal, RTX GPUs under Vulkan.
    Dlss,
}

impl Upscaler {
    pub const ALL: [Upscaler; 6] =
        [Upscaler::Off, Upscaler::Bilinear, Upscaler::Fsr1, Upscaler::Fsr3, Upscaler::Fsr4, Upscaler::Dlss];

    pub fn name(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Bilinear => "bilinear",
            Self::Fsr1 => "fsr1",
            Self::Fsr3 => "fsr3",
            Self::Fsr4 => "fsr4",
            Self::Dlss => "dlss",
        }
    }

    /// Whether it accumulates frames: it needs jitter, motion vectors and a history, it does its own
    /// anti-aliasing, and its [`UpscaleQuality`] mode (not the render scale) sets the render size.
    pub fn is_temporal(self) -> bool {
        matches!(self, Self::Fsr3 | Self::Fsr4 | Self::Dlss)
    }
}

/// How far a temporal upscaler renders below the output: the quality modes of FSR 3/4 and DLSS.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum UpscaleQuality {
    /// Let the renderer pick; today that is [`Quality`](Self::Quality).
    Auto,
    /// Render at the output size (temporal anti-aliasing only: FSR native AA, DLAA).
    Native,
    #[default]
    Quality,
    Balanced,
    Performance,
    UltraPerformance,
}

impl UpscaleQuality {
    pub const ALL: [UpscaleQuality; 6] = [
        UpscaleQuality::Auto,
        UpscaleQuality::Native,
        UpscaleQuality::Quality,
        UpscaleQuality::Balanced,
        UpscaleQuality::Performance,
        UpscaleQuality::UltraPerformance,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Native => "native",
            Self::Quality => "quality",
            Self::Balanced => "balanced",
            Self::Performance => "performance",
            Self::UltraPerformance => "ultra_performance",
        }
    }

    /// The per-axis ratio output size / render size, as in FSR 3: 1.0, 1.5, 1.7, 2.0 and 3.0.
    pub fn ratio(self) -> f32 {
        match self {
            Self::Native => 1.0,
            Self::Auto | Self::Quality => 1.5,
            Self::Balanced => 1.7,
            Self::Performance => 2.0,
            Self::UltraPerformance => 3.0,
        }
    }

    /// The render scale (render size / output size per axis) this mode draws at.
    pub fn render_scale(self) -> f32 {
        clamp_render_scale(1.0 / self.ratio())
    }
}

/// The sharpening strength used when none is chosen: 0.0 is off, 1.0 is the strongest RCAS setting.
pub const DEFAULT_UPSCALE_SHARPNESS: f32 = 0.8;

/// Largest RCAS attenuation, in stops, at sharpness 0.0 (a sharpness of 0.0 skips RCAS entirely).
const RCAS_MAX_STOPS: f32 = 2.0;

/// Clamp a sharpness into 0.0..=1.0. Non-finite values give [`DEFAULT_UPSCALE_SHARPNESS`].
pub fn clamp_upscale_sharpness(sharpness: f32) -> f32 {
    if !sharpness.is_finite() {
        return DEFAULT_UPSCALE_SHARPNESS;
    }
    sharpness.clamp(0.0, 1.0)
}

/// The RCAS attenuation in stops (halvings of the sharpening) for a sharpness, or `None` when the
/// sharpness is zero and the pass is skipped. AMD's reference value is 0.2 stops (a sharpness of 0.9).
pub fn rcas_stops(sharpness: f32) -> Option<f32> {
    let sharpness = clamp_upscale_sharpness(sharpness);
    if sharpness <= 0.0 {
        return None;
    }
    Some((1.0 - sharpness) * RCAS_MAX_STOPS)
}

/// Whether the FSR 1 passes run: the upscaler is chosen and the scene is drawn below the output size.
pub fn fsr1_active(upscaler: Upscaler, render_scale: f32) -> bool {
    upscaler == Upscaler::Fsr1 && clamp_render_scale(render_scale) < DEFAULT_RENDER_SCALE
}

/// The texture LOD bias that keeps texture detail matched to the output size when the scene is drawn at
/// `render_scale`: `log2(scale)` below 1.0 (so 0.5 gives -1.0, one mip sharper), 0.0 at or above it.
/// Pass it to the renderer's texture LOD bias while upscaling.
///
/// A spatial upscaler has no history to accumulate samples over, so this is the plain match of texel
/// density to the output and does not add the extra -1 that temporal upscalers use.
pub fn suggested_texture_lod_bias(render_scale: f32) -> f32 {
    let scale = clamp_render_scale(render_scale);
    if scale >= DEFAULT_RENDER_SCALE {
        return 0.0;
    }
    scale.log2()
}

/// The texture LOD bias for a temporal upscaler drawing at `render_scale`: the spatial bias minus one
/// more mip, `log2(scale) - 1`, because the accumulated history supplies the extra samples (also
/// below native, so -1.0 at 1.0). Clamped to the accepted range.
pub fn suggested_temporal_texture_lod_bias(render_scale: f32) -> f32 {
    let scale = clamp_render_scale(render_scale);
    clamp_texture_lod_bias(scale.log2() - 1.0)
}

/// Smallest and largest texture LOD bias a renderer accepts.
pub const MIN_TEXTURE_LOD_BIAS: f32 = -4.0;
pub const MAX_TEXTURE_LOD_BIAS: f32 = 4.0;

/// Clamp a texture LOD bias into [`MIN_TEXTURE_LOD_BIAS`]..=[`MAX_TEXTURE_LOD_BIAS`]. Non-finite values give 0.0.
pub fn clamp_texture_lod_bias(bias: f32) -> f32 {
    if !bias.is_finite() {
        return 0.0;
    }
    bias.clamp(MIN_TEXTURE_LOD_BIAS, MAX_TEXTURE_LOD_BIAS)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sharpness_clamps_and_maps_to_stops() {
        assert_eq!(clamp_upscale_sharpness(-1.0), 0.0);
        assert_eq!(clamp_upscale_sharpness(3.0), 1.0);
        assert_eq!(clamp_upscale_sharpness(f32::NAN), DEFAULT_UPSCALE_SHARPNESS);
        assert_eq!(rcas_stops(0.0), None, "zero skips the pass");
        assert_eq!(rcas_stops(1.0), Some(0.0), "full sharpness is zero stops of attenuation");
        let stops = rcas_stops(0.9).unwrap();
        assert!((stops - 0.2).abs() < 1e-6, "0.9 is AMD's reference 0.2 stops, got {stops}");
        assert!(rcas_stops(0.2).unwrap() > rcas_stops(0.8).unwrap(), "less sharpness, more attenuation");
    }

    #[test]
    fn fsr1_runs_only_below_native() {
        assert!(fsr1_active(Upscaler::Fsr1, 0.5));
        assert!(fsr1_active(Upscaler::Fsr1, 0.99));
        assert!(!fsr1_active(Upscaler::Fsr1, 1.0), "native needs no upscale");
        assert!(!fsr1_active(Upscaler::Fsr1, 1.5), "supersampling is filtered down, not upscaled");
        assert!(!fsr1_active(Upscaler::Bilinear, 0.5));
        assert!(fsr1_active(Upscaler::Fsr1, 0.0), "scales clamp to the minimum first");
    }

    #[test]
    fn suggested_bias_follows_the_scale() {
        assert_eq!(suggested_texture_lod_bias(1.0), 0.0);
        assert_eq!(suggested_texture_lod_bias(2.0), 0.0);
        assert_eq!(suggested_texture_lod_bias(0.5), -1.0);
        assert_eq!(suggested_texture_lod_bias(0.25), -2.0);
        assert_eq!(suggested_texture_lod_bias(0.0), -2.0, "clamped to the smallest scale");
        assert_eq!(suggested_texture_lod_bias(f32::NAN), 0.0);
        let bias = suggested_texture_lod_bias(0.67);
        assert!((-0.6..-0.5).contains(&bias), "{bias}");
    }

    #[test]
    fn only_fsr3_fsr4_and_dlss_are_temporal() {
        let temporal: Vec<_> = Upscaler::ALL.into_iter().filter(|u| u.is_temporal()).collect();
        assert_eq!(temporal, [Upscaler::Fsr3, Upscaler::Fsr4, Upscaler::Dlss]);
        assert_eq!(Upscaler::default(), Upscaler::Bilinear);
        assert_eq!(Upscaler::ALL.map(Upscaler::name), ["off", "bilinear", "fsr1", "fsr3", "fsr4", "dlss"]);
    }

    #[test]
    fn quality_modes_map_to_the_fsr_ratios() {
        let ratios = UpscaleQuality::ALL.map(UpscaleQuality::ratio);
        assert_eq!(ratios, [1.5, 1.0, 1.5, 1.7, 2.0, 3.0]);
        assert_eq!(UpscaleQuality::Native.render_scale(), 1.0);
        assert!((UpscaleQuality::Quality.render_scale() - 2.0 / 3.0).abs() < 1e-6);
        assert_eq!(UpscaleQuality::Auto.render_scale(), UpscaleQuality::Quality.render_scale());
        assert_eq!(UpscaleQuality::Performance.render_scale(), 0.5);
        for q in UpscaleQuality::ALL {
            assert!((crate::MIN_RENDER_SCALE..=1.0).contains(&q.render_scale()), "{q:?}");
        }
        assert_eq!(UpscaleQuality::ALL.map(UpscaleQuality::name)[5], "ultra_performance");
        assert_eq!(UpscaleQuality::default(), UpscaleQuality::Quality);
    }

    #[test]
    fn temporal_bias_is_one_mip_sharper_than_spatial() {
        assert_eq!(suggested_temporal_texture_lod_bias(1.0), -1.0);
        assert_eq!(suggested_temporal_texture_lod_bias(0.5), -2.0);
        assert_eq!(suggested_temporal_texture_lod_bias(0.25), -3.0);
        assert_eq!(suggested_temporal_texture_lod_bias(f32::NAN), -1.0);
        for scale in [1.0, 0.67, 0.5, 0.33] {
            assert_eq!(suggested_temporal_texture_lod_bias(scale), suggested_texture_lod_bias(scale) - 1.0);
        }
    }

    #[test]
    fn bias_clamps() {
        assert_eq!(clamp_texture_lod_bias(-9.0), MIN_TEXTURE_LOD_BIAS);
        assert_eq!(clamp_texture_lod_bias(9.0), MAX_TEXTURE_LOD_BIAS);
        assert_eq!(clamp_texture_lod_bias(f32::NAN), 0.0);
        assert_eq!(clamp_texture_lod_bias(-1.0), -1.0);
    }
}
