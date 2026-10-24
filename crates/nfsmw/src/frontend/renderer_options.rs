//! The renderer rows of the Video options (docs/renderers.md): which renderer, the graphics API it uses, the
//! temporal upscalers' quality mode, and ray tracing. The quality mode only exists on a renderer that offers a
//! temporal upscaler, and is left out otherwise. Ray tracing is always listed: a renderer without it dims the row.

use std::sync::OnceLock;

use blackbox_gfx::{Capabilities, GraphicsApi, Upscaler};

use super::ids::LABEL_OFF;
use super::options::{Data, LABEL_AUTO, Title, message};
use super::post_options::pick;
use crate::settings::{Partial, RayTracingLevel, RendererKind, Settings, UpscaleMode, UpscaleQuality};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RendererSetting {
    Renderer,
    GraphicsApi,
    UpscaleQuality,
    /// Frame generation for temporal upscalers; unavailable until the active renderer supports it.
    FrameGeneration,
    RayTracing,
}

impl RendererSetting {
    pub fn title(self) -> Title {
        Title::Key(match self {
            Self::Renderer => "options-renderer",
            Self::GraphicsApi => "options-graphics-api",
            Self::UpscaleQuality => "options-upscale-quality",
            Self::FrameGeneration => "options-frame-generation",
            Self::RayTracing => "options-ray-tracing",
        })
    }

    /// Whether the renderer has this row at all. The renderer and graphics API choices are always there: they
    /// take effect after a restart, so the renderer that is running cannot show them as unavailable. Ray tracing
    /// is always there too, dimmed when the renderer cannot run it (`Setting::enabled_in`).
    pub fn shown(self, caps: &Capabilities) -> bool {
        match self {
            Self::Renderer | Self::GraphicsApi | Self::RayTracing => true,
            Self::UpscaleQuality | Self::FrameGeneration => caps.upscalers.iter().any(Upscaler::is_temporal),
        }
    }

    /// What the row shows. The renderers', APIs' and upscalers' names are not language; Auto and Off are the original's.
    pub fn data(self, s: &Settings) -> Data {
        match self {
            Self::Renderer => match s.renderer {
                RendererKind::Blackbox => Data::Text("Black Box".into()),
                RendererKind::Bevy => Data::Text("Bevy".into()),
            },
            Self::GraphicsApi => match s.backend {
                GraphicsApi::Auto => Data::Label(LABEL_AUTO),
                GraphicsApi::Vulkan => Data::Text("Vulkan".into()),
                GraphicsApi::Dx12 => Data::Text("DirectX 12".into()),
                GraphicsApi::Gl => Data::Text("OpenGL".into()),
            },
            Self::UpscaleQuality => match s.upscale_quality {
                UpscaleQuality::Auto => Data::Label(LABEL_AUTO),
                // DLSS at the output size is DLAA: it anti-aliases and does not upscale.
                UpscaleQuality::Native if s.upscaler == UpscaleMode::Dlss => Data::Text("DLAA".into()),
                UpscaleQuality::Native => message("options-upscale-native"),
                UpscaleQuality::Quality => message("options-upscale-quality-mode"),
                UpscaleQuality::Balanced => message("options-upscale-balanced"),
                UpscaleQuality::Performance => message("options-upscale-performance"),
                UpscaleQuality::UltraPerformance => message("options-upscale-ultra-performance"),
            },
            Self::FrameGeneration => match s.frame_generation.0 {
                0 => Data::Label(LABEL_OFF),
                // The multiplier (2x, 3x) is not language.
                _ => Data::Text(s.frame_generation.to_string()),
            },
            Self::RayTracing => match s.ray_tracing {
                RayTracingLevel::Off => Data::Label(LABEL_OFF),
                RayTracingLevel::Low => message("options-low"),
                RayTracingLevel::Medium => message("options-medium"),
                RayTracingLevel::High => message("options-high"),
            },
        }
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
            Self::FrameGeneration => {
                s.frame_generation = pick(
                    &[crate::settings::FrameGeneration(0), crate::settings::FrameGeneration(1)],
                    s.frame_generation,
                    forward,
                );
                changed.frame_generation = Some(s.frame_generation);
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
        let text = crate::frontend::options::shown;
        assert_eq!(names, [text("Auto"), text("Vulkan"), text("DirectX 12"), text("OpenGL")]);
    }
}
