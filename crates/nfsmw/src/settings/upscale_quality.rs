//! The `upscale_quality` setting: how far a temporal upscaler (fsr3, fsr4, dlss) renders below the output
//! (docs/upscaling.md). The spatial upscalers use `render_scale` instead.

use blackbox_gfx::UpscaleQuality as Mode;

/// The quality mode of a temporal upscaler; it sets the render size.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum UpscaleQuality {
    /// Let the renderer pick (quality today).
    Auto,
    /// The output size: temporal anti-aliasing only, no upscaling.
    Native,
    /// 1.5 times smaller per axis (67 %). The default.
    #[default]
    Quality,
    /// 1.7 times smaller (59 %).
    Balanced,
    /// 2 times smaller (50 %).
    Performance,
    /// 3 times smaller (33 %).
    UltraPerformance,
}

names!(
    UpscaleQuality,
    "auto, native, quality, balanced, performance or ultra_performance",
    [
        (Self::Auto, "auto"),
        (Self::Native, "native"),
        (Self::Quality, "quality"),
        (Self::Balanced, "balanced"),
        (Self::Performance, "performance"),
        (Self::UltraPerformance, "ultra_performance")
    ]
);

impl UpscaleQuality {
    /// The renderer's quality mode.
    pub fn mode(self) -> Mode {
        match self {
            Self::Auto => Mode::Auto,
            Self::Native => Mode::Native,
            Self::Quality => Mode::Quality,
            Self::Balanced => Mode::Balanced,
            Self::Performance => Mode::Performance,
            Self::UltraPerformance => Mode::UltraPerformance,
        }
    }
}
