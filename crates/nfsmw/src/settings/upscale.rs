//! Render scale and upscaler settings (docs/upscaling.md).

use std::fmt;
use std::str::FromStr;

use blackbox_render::Upscaler;

/// Smallest and largest render scale in percent of the output size per axis.
pub const MIN_RENDER_SCALE: u16 = 50;
pub const MAX_RENDER_SCALE: u16 = 200;

/// The size the 3D scene is drawn at, in percent of the output size per axis (50 to 200, 100 is native).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RenderScale(u16);

impl Default for RenderScale {
    fn default() -> Self {
        Self(100)
    }
}

impl RenderScale {
    /// `None` outside 50 to 200.
    pub fn new(percent: u16) -> Option<Self> {
        (MIN_RENDER_SCALE..=MAX_RENDER_SCALE).contains(&percent).then_some(Self(percent))
    }

    pub fn percent(self) -> u16 {
        self.0
    }

    /// As the renderer's per-axis factor (1.0 is native).
    pub fn factor(self) -> f32 {
        f32::from(self.0) / 100.0
    }
}

impl FromStr for RenderScale {
    type Err = String;

    /// `67`, `67%` (percent) or `0.67`, `1.5` (a factor, recognised by its decimal point).
    fn from_str(s: &str) -> Result<Self, String> {
        let text = s.trim().trim_end_matches('%');
        let bad =
            || format!("expected a render scale from {MIN_RENDER_SCALE} to {MAX_RENDER_SCALE} percent, got {s:?}");
        let value: f32 = text.parse().map_err(|_| bad())?;
        let percent = if text.contains('.') { value * 100.0 } else { value };
        // The cast saturates, so negative, NaN and huge values land outside the range.
        Self::new(percent.round() as u16).ok_or_else(bad)
    }
}

impl fmt::Display for RenderScale {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// How the scene is brought back to the output size when the render scale is below 100.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum UpscaleMode {
    /// No scaling: the render scale is ignored and the scene is drawn at the output size.
    Off,
    /// Bilinear filtering.
    Bilinear,
    /// AMD FidelityFX Super Resolution 1.
    #[default]
    Fsr1,
}

impl UpscaleMode {
    /// The renderer's upscaler, or `None` when the render scale is ignored.
    pub fn upscaler(self) -> Option<Upscaler> {
        match self {
            Self::Off => None,
            Self::Bilinear => Some(Upscaler::Bilinear),
            Self::Fsr1 => Some(Upscaler::Fsr1),
        }
    }
}

impl FromStr for UpscaleMode {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, String> {
        match s.trim().to_ascii_lowercase().as_str() {
            "off" | "none" => Ok(Self::Off),
            "bilinear" => Ok(Self::Bilinear),
            "fsr1" | "fsr" | "fsr-1" => Ok(Self::Fsr1),
            _ => Err(format!("expected off, bilinear or fsr1, got {s:?}")),
        }
    }
}

impl fmt::Display for UpscaleMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Off => "off",
            Self::Bilinear => "bilinear",
            Self::Fsr1 => "fsr1",
        })
    }
}
