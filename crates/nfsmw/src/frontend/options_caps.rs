//! The side of a row that depends on the running renderer: whether it is live, what it reads and how it steps over
//! the values the renderer cannot run.

use blackbox_gfx::Capabilities;

use super::{Data, MAX_VALUES, Setting};
use crate::frontend::graphics_options::GraphicsSetting;
use crate::frontend::post_options::PostSetting;
use crate::frontend::renderer_options::RendererSetting;
use crate::settings::availability;
use crate::settings::{Partial, Settings};

impl Setting {
    /// Dependency checks use the running renderer's values, keeping stored preferences intact.
    pub fn enabled_in(self, s: &Settings, caps: &Capabilities) -> bool {
        if self == Setting::RenderScale {
            // Even a temporal request falling back to FSR 1 takes its size from Upscale Quality.
            return self.enabled(s);
        }
        if self == Setting::Renderer(RendererSetting::RayTracing) && !caps.ray_tracing.is_available() {
            return false;
        }
        if self == Setting::Renderer(RendererSetting::FrameGeneration) {
            return caps.frame_generation && self.enabled(&s.effective(caps));
        }
        self.enabled(&s.effective(caps))
    }

    /// What the data string shows on a renderer with these capabilities: the value it actually runs, so a stored
    /// `taa` reads FXAA on a renderer that has no TAA.
    pub fn data_in(self, s: &Settings, caps: &Capabilities) -> Data {
        if self == Setting::Renderer(RendererSetting::RayTracing) && !caps.ray_tracing.is_available() {
            return Data::Text("Unavailable".into());
        }
        if self == Setting::Renderer(RendererSetting::FrameGeneration) && !caps.frame_generation {
            return Data::Text("Unavailable".into());
        }
        self.data(&s.effective(caps))
    }

    /// The config key of a setting a renderer may not be able to run.
    fn key(self) -> Option<&'static str> {
        match self {
            Setting::Graphics(GraphicsSetting::VisualTreatment) => Some("visual_treatment"),
            Setting::Graphics(GraphicsSetting::Reflections) => Some("reflections"),
            Setting::Graphics(GraphicsSetting::Shadows) => Some("shadows"),
            Setting::Graphics(GraphicsSetting::AmbientOcclusion) => Some("ao"),
            Setting::Upscaler => Some("upscaler"),
            Setting::Post(PostSetting::Aa) => Some("post_aa"),
            Setting::Post(PostSetting::Tonemap) => Some("post_tonemap"),
            Setting::Post(PostSetting::Bloom) => Some("post_bloom"),
            Setting::Renderer(RendererSetting::RayTracing) => Some("ray_tracing"),
            _ => None,
        }
    }

    /// Whether the renderer can run the setting's current value.
    pub fn offered(self, s: &Settings, caps: &Capabilities) -> bool {
        self.key().is_none_or(|key| availability::is_offered(key, s, caps))
    }

    /// [`step`](Self::step), skipping the values the renderer cannot run, so a toggle only cycles through what
    /// works. A stored value the renderer cannot run (set for another renderer) moves to the next one it can.
    pub fn advance(self, s: &mut Settings, changed: &mut Partial, forward: bool, caps: &Capabilities) -> bool {
        if !self.enabled_in(s, caps) {
            return false;
        }
        let before = *s;
        for _ in 0..MAX_VALUES {
            self.step_value(s, changed, forward);
            if s.settle_preset() {
                changed.graphics_preset = Some(crate::settings::GraphicsPreset::Custom);
            }
            if self.offered(s, caps) {
                break;
            }
        }
        before != *s
    }
}
