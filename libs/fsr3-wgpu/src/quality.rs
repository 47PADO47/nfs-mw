//! The quality modes of FSR 3.1 and the numbers that follow from them.
//!
//! The per-axis upscale ratios are AMD's published presets
//! (`ffxFsr3UpscalerGetUpscaleRatioFromQualityMode`). They are the only ratios an application should
//! offer by name, but any ratio between 1.0 and 3.0 works with the upscaler.

use glam::UVec2;

/// One of the preset render-to-display ratios.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum QualityMode {
    /// 1.0x per axis: temporal anti-aliasing at native resolution.
    NativeAa,
    /// 1.5x per axis.
    Quality,
    /// 1.7x per axis.
    Balanced,
    /// 2.0x per axis.
    Performance,
    /// 3.0x per axis.
    UltraPerformance,
}

impl QualityMode {
    /// Every mode, from the sharpest to the cheapest.
    pub const ALL: [Self; 5] =
        [Self::NativeAa, Self::Quality, Self::Balanced, Self::Performance, Self::UltraPerformance];

    /// The display size divided by the render size, per axis.
    pub fn ratio(self) -> f32 {
        match self {
            Self::NativeAa => 1.0,
            Self::Quality => 1.5,
            Self::Balanced => 1.7,
            Self::Performance => 2.0,
            Self::UltraPerformance => 3.0,
        }
    }

    /// The size to render at for a display of `display` pixels. Truncates like AMD's helper does and
    /// never returns an empty size.
    pub fn render_size(self, display: UVec2) -> UVec2 {
        render_size_for_ratio(display, self.ratio())
    }

    /// The texture LOD bias to sample world textures with at this mode: see [`mip_bias`].
    pub fn mip_bias(self) -> f32 {
        mip_bias(self.ratio())
    }
}

/// The render size for a display of `display` pixels at an arbitrary per-axis `ratio` (display over render).
pub fn render_size_for_ratio(display: UVec2, ratio: f32) -> UVec2 {
    let ratio = if ratio.is_finite() && ratio >= 1.0 { ratio } else { 1.0 };
    let scale = |v: u32| ((v as f32 / ratio) as u32).max(1);
    UVec2::new(scale(display.x), scale(display.y))
}

/// The texture LOD bias for a per-axis upscale `ratio`: `log2(1 / ratio) - 1`.
///
/// Sampling textures with this bias keeps their detail at the display resolution, and the extra
/// -1 compensates for the jitter accumulation's softening, as AMD recommends. Add it to the usual LOD
/// computation of every world material.
pub fn mip_bias(ratio: f32) -> f32 {
    (1.0 / ratio).log2() - 1.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ratios_are_the_amd_presets() {
        let ratios = QualityMode::ALL.map(QualityMode::ratio);
        assert_eq!(ratios, [1.0, 1.5, 1.7, 2.0, 3.0]);
    }

    #[test]
    fn render_sizes_truncate_like_the_sdk() {
        let display = UVec2::new(3840, 2160);
        assert_eq!(QualityMode::NativeAa.render_size(display), UVec2::new(3840, 2160));
        assert_eq!(QualityMode::Quality.render_size(display), UVec2::new(2560, 1440));
        assert_eq!(QualityMode::Balanced.render_size(display), UVec2::new(2258, 1270));
        assert_eq!(QualityMode::Performance.render_size(display), UVec2::new(1920, 1080));
        assert_eq!(QualityMode::UltraPerformance.render_size(display), UVec2::new(1280, 720));
    }

    #[test]
    fn render_size_never_collapses() {
        assert_eq!(QualityMode::UltraPerformance.render_size(UVec2::new(2, 1)), UVec2::new(1, 1));
        assert_eq!(render_size_for_ratio(UVec2::new(100, 100), f32::NAN), UVec2::new(100, 100));
        assert_eq!(render_size_for_ratio(UVec2::new(100, 100), 0.5), UVec2::new(100, 100));
    }

    #[test]
    fn mip_bias_is_log2_of_the_inverse_ratio_minus_one() {
        assert_eq!(mip_bias(1.0), -1.0);
        assert!((mip_bias(2.0) - -2.0).abs() < 1e-6);
        assert!((mip_bias(1.5) - (-0.584_962_5 - 1.0)).abs() < 1e-5);
        assert!((QualityMode::UltraPerformance.mip_bias() - (-1.584_962_5 - 1.0)).abs() < 1e-5);
    }

    #[test]
    fn mip_bias_matches_the_render_to_display_size_ratio() {
        // AMD's own formula is log2(renderWidth / displayWidth) - 1.
        let display = UVec2::new(1920, 1080);
        let render = QualityMode::Performance.render_size(display);
        let amd = (render.x as f32 / display.x as f32).log2() - 1.0;
        assert!((amd - QualityMode::Performance.mip_bias()).abs() < 1e-6);
    }
}
