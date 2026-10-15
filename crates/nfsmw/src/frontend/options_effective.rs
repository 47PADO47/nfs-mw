//! Capability-aware values shown by menu rows. Stored requests stay intact.
use super::*;

impl Setting {
    /// What the data string shows on a renderer with these capabilities: the value it actually runs, so a stored
    /// `taa` reads FXAA on a renderer that has no TAA.
    pub fn data_in(self, s: &Settings, caps: &Capabilities) -> Data {
        if matches!(self, Setting::Graphics(GraphicsSetting::Style | GraphicsSetting::Quality)) {
            return self.data(s);
        }
        let mut shown = s.effective(caps);
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
