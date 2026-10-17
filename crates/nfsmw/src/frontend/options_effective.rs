//! Capability-aware values shown by menu rows. Stored requests stay intact.
use super::*;
use blackbox_gfx::Capabilities;

impl Setting {
    /// What the data string shows on a renderer with these capabilities: the value it actually runs, so a stored
    /// `taa` reads FXAA on a renderer that has no TAA.
    pub fn data_in(self, s: &Settings, caps: &Capabilities) -> Data {
        if self == Setting::Graphics(GraphicsSetting::WorldDetail) {
            return Data::Text(format!("{} (Textures: next world load)", s.detail.world.label()));
        }
        if self == Setting::Renderer(RendererSetting::RayTracing) && !caps.ray_tracing.is_available() {
            return Data::Text("Unavailable".into());
        }
        if self == Setting::Renderer(RendererSetting::FrameGeneration) && !caps.frame_generation {
            return Data::Text("Unavailable".into());
        }
        if matches!(self, Setting::Graphics(GraphicsSetting::Style | GraphicsSetting::Quality)) {
            return self.data(s);
        }
        if self == Setting::Graphics(GraphicsSetting::CarShading)
            && s.car_shading.is_original()
            && caps.renderer != "blackbox"
        {
            return Data::Text("Custom shader fallback".into());
        }
        let mut shown = s.effective(caps);
        if self == Setting::Graphics(GraphicsSetting::TextureFiltering) {
            let suffix = match caps.restart_required.contains(blackbox_gfx::Setting::TextureFiltering) {
                true => " (Restart)",
                false => "",
            };
            return Data::Text(format!("{}{suffix}", shown.texture_filtering.label()));
        }
        if self == Setting::Graphics(GraphicsSetting::Shadows) && caps.renderer != "blackbox" {
            return Data::Text(
                match shown.shadows {
                    crate::settings::ShadowQuality::Off => "Off",
                    crate::settings::ShadowQuality::Low => "Low",
                    crate::settings::ShadowQuality::Medium => "Medium",
                    crate::settings::ShadowQuality::High => "High",
                }
                .into(),
            );
        }
        if !caps.scene_treatment {
            shown.appearance.treatment_strength = None;
        }
        self.data(&shown)
    }
}
