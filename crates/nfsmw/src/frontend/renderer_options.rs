//! The renderer rows of the Video options (docs/renderers.md): which renderer, the graphics API it uses, the
//! temporal upscalers' quality mode, and ray tracing. The last two only exist on a renderer that offers them (a
//! menu row has no greyed-out state, so the row is left out instead).

use std::sync::OnceLock;

use blackbox_gfx::{Capabilities, GraphicsApi, Upscaler};

use super::options::{Data, Title};
use super::post_options::pick;
use crate::settings::{Partial, RayTracingLevel, RendererKind, Settings, UpscaleQuality};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RendererSetting {
    Renderer,
    GraphicsApi,
    UpscaleQuality,
    RayTracing,
}

impl RendererSetting {
    pub fn title(self) -> Title {
        Title::Text(match self {
            Self::Renderer => "Renderer (After Restart)",
            Self::GraphicsApi => "Graphics API (After Restart)",
            Self::UpscaleQuality => "Upscale Quality",
            Self::RayTracing => "Ray Tracing",
        })
    }

    /// Whether the renderer has this row at all. The renderer and graphics API choices are always there: they
    /// take effect after a restart, so the renderer that is running cannot show them as unavailable.
    pub fn shown(self, caps: &Capabilities) -> bool {
        match self {
            Self::Renderer | Self::GraphicsApi => true,
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
                Self::GraphicsApi => match s.backend {
                    GraphicsApi::Auto => "Auto",
                    GraphicsApi::Vulkan => "Vulkan",
                    GraphicsApi::Dx12 => "DirectX 12",
                    GraphicsApi::Gl => "OpenGL",
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
            Self::GraphicsApi => {
                let offered = offered_apis(s.renderer, available_apis());
                s.backend = pick(&offered, s.backend, forward);
                changed.backend = Some(s.backend);
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

/// The APIs this PC has a GPU for, probed once: enumerating adapters is too slow to repeat each time the row moves.
fn available_apis() -> &'static [GraphicsApi] {
    static APIS: OnceLock<Vec<GraphicsApi>> = OnceLock::new();
    APIS.get_or_init(blackbox_render::available_apis)
}

/// The APIs the row cycles through: the available ones the renderer can run. The Bevy renderer has no OpenGL
/// path (docs/bevy-backend.md), so it is left out for that renderer.
fn offered_apis(renderer: RendererKind, available: &[GraphicsApi]) -> Vec<GraphicsApi> {
    available.iter().copied().filter(|api| !(renderer == RendererKind::Bevy && *api == GraphicsApi::Gl)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_row_offers_only_the_apis_this_pc_has_and_auto_is_always_first() {
        let available = [GraphicsApi::Auto, GraphicsApi::Dx12];
        assert_eq!(offered_apis(RendererKind::Blackbox, &available), available);
    }

    #[test]
    fn bevy_is_offered_no_opengl() {
        let all = [GraphicsApi::Auto, GraphicsApi::Vulkan, GraphicsApi::Dx12, GraphicsApi::Gl];
        assert_eq!(offered_apis(RendererKind::Bevy, &all), [GraphicsApi::Auto, GraphicsApi::Vulkan, GraphicsApi::Dx12]);
        assert_eq!(offered_apis(RendererKind::Blackbox, &all), all);
    }

    #[test]
    fn the_row_names_each_api() {
        let mut s = Settings::from(Partial::default());
        let names: Vec<_> = GraphicsApi::ALL
            .into_iter()
            .map(|api| {
                s.backend = api;
                RendererSetting::GraphicsApi.data(&s)
            })
            .collect();
        let text = |t: &str| Data::Text(t.into());
        assert_eq!(names, [text("Auto"), text("Vulkan"), text("DirectX 12"), text("OpenGL")]);
    }
}
