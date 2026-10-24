//! Capability-aware values shown by menu rows. Stored requests stay intact.
use super::*;
use crate::i18n::t;
use blackbox_gfx::Capabilities;

impl Setting {
    /// What the data string shows on a renderer with these capabilities: the value it actually runs, so a stored
    /// `taa` reads FXAA on a renderer that has no TAA.
    pub fn data_in(self, s: &Settings, caps: &Capabilities) -> Data {
        if self == Setting::Graphics(GraphicsSetting::WorldDetail) {
            return Data::Text(t!("options-world-detail-value", level = s.detail.world.menu_label()));
        }
        if self == Setting::Renderer(RendererSetting::RayTracing) && !caps.ray_tracing.is_available() {
            return message("options-unavailable");
        }
        if self == Setting::Renderer(RendererSetting::FrameGeneration) && !caps.frame_generation {
            return message("options-unavailable");
        }
        if matches!(
            self,
            Setting::Graphics(
                GraphicsSetting::Style
                    | GraphicsSetting::Quality
                    | GraphicsSetting::ShadowPreset
                    | GraphicsSetting::ReflectionPreset
            )
        ) {
            return self.data(s);
        }
        if self == Setting::Graphics(GraphicsSetting::CarShading)
            && s.car_shading.is_original()
            && caps.renderer != "blackbox"
        {
            return message("options-car-shading-fallback");
        }
        let mut shown = s.effective(caps);
        if self == Setting::Graphics(GraphicsSetting::TextureFiltering) {
            let filtering = super::super::graphics_options::filtering_name(shown.texture_filtering);
            return Data::Text(match caps.restart_required.contains(blackbox_gfx::Setting::TextureFiltering) {
                true => t!("options-filtering-restart", filtering = filtering),
                false => filtering,
            });
        }
        if self == Setting::Graphics(GraphicsSetting::Shadows) && caps.renderer != "blackbox" {
            return match shown.shadows {
                crate::settings::ShadowQuality::Off => Data::Label(LABEL_OFF),
                crate::settings::ShadowQuality::Low => message("options-low"),
                crate::settings::ShadowQuality::Medium => message("options-medium"),
                crate::settings::ShadowQuality::High => message("options-high"),
            };
        }
        if !caps.scene_treatment {
            shown.appearance.treatment_strength = None;
        }
        self.data(&shown)
    }
}
