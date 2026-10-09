//! The renderer rows of the Video options (docs/renderers.md): which renderer, the temporal upscalers' quality
//! mode, and ray tracing. The last two only exist on a renderer that offers them (a menu row has no greyed-out
//! state, so the row is left out instead).

use blackbox_gfx::{Capabilities, Upscaler};

use super::options::{Data, Title};
use super::post_options::pick;
use crate::settings::{Partial, RayTracingLevel, RendererKind, Settings, UpscaleQuality};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RendererSetting {
    Renderer,
    UpscaleQuality,
    RayTracing,
}

impl RendererSetting {
    pub fn title(self) -> Title {
        Title::Text(match self {
            Self::Renderer => "Renderer (After Restart)",
            Self::UpscaleQuality => "Upscale Quality",
            Self::RayTracing => "Ray Tracing",
        })
    }

    /// Whether the renderer has this row at all. The renderer choice is always there: it is what switches to
    /// another renderer after a restart.
    pub fn shown(self, caps: &Capabilities) -> bool {
        match self {
            Self::Renderer => true,
            Self::UpscaleQuality => caps.upscalers.iter().any(Upscaler::is_temporal),
            Self::RayTracing => caps.ray_tracing.is_available(),
        }
    }

    pub fn data(self, s: &Settings) -> Data {
        Data::Text(
            match self {
                Self::Renderer => match s.renderer {
                    RendererKind::Blackbox => "Black Box",
                    RendererKind::Bevy => "Bevy",
                },
                Self::UpscaleQuality => match s.upscale_quality {
                    UpscaleQuality::Auto => "Auto",
                    UpscaleQuality::Native => "Native",
                    UpscaleQuality::Quality => "Quality",
                    UpscaleQuality::Balanced => "Balanced",
                    UpscaleQuality::Performance => "Performance",
                    UpscaleQuality::UltraPerformance => "Ultra Performance",
                },
                Self::RayTracing => match s.ray_tracing {
                    RayTracingLevel::Off => "Off",
                    RayTracingLevel::Low => "Low",
                    RayTracingLevel::Medium => "Medium",
                    RayTracingLevel::High => "High",
                },
            }
            .to_owned(),
        )
    }

    /// Moves to the next (or previous) value, wrapping, and records the change for the config file.
    pub fn step(self, s: &mut Settings, changed: &mut Partial, forward: bool) -> bool {
        let before = *s;
        match self {
            Self::Renderer => {
                s.renderer = pick(&[RendererKind::Blackbox, RendererKind::Bevy], s.renderer, forward);
                changed.renderer = Some(s.renderer);
            }
            Self::UpscaleQuality => {
                let all = [
                    UpscaleQuality::Auto,
                    UpscaleQuality::Native,
                    UpscaleQuality::Quality,
                    UpscaleQuality::Balanced,
                    UpscaleQuality::Performance,
                    UpscaleQuality::UltraPerformance,
                ];
                s.upscale_quality = pick(&all, s.upscale_quality, forward);
                changed.upscale_quality = Some(s.upscale_quality);
            }
            Self::RayTracing => {
                let all = [RayTracingLevel::Off, RayTracingLevel::Low, RayTracingLevel::Medium, RayTracingLevel::High];
                s.ray_tracing = pick(&all, s.ray_tracing, forward);
                changed.ray_tracing = Some(s.ray_tracing);
            }
        }
        before != *s
    }
}
