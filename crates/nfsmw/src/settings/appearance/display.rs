//! HDR calibration persists independently of visual style and quality.
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DisplayMode {
    Off,
    #[default]
    Auto,
    On,
}
names!(DisplayMode, "off, auto or on", [(Self::Off, "off"), (Self::Auto, "auto"), (Self::On, "on")]);
impl DisplayMode {
    pub const ALL: [Self; 3] = [Self::Off, Self::Auto, Self::On];
}
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct DisplayTuning {
    pub mode: DisplayMode,
    pub peak_nits: f32,
    pub scene_white_nits: f32,
    pub ui_white_nits: f32,
}
impl Default for DisplayTuning {
    fn default() -> Self {
        Self { mode: DisplayMode::Auto, peak_nits: 0.0, scene_white_nits: 203.0, ui_white_nits: 203.0 }
    }
}
impl DisplayTuning {
    pub fn sanitized(self) -> Self {
        let r = self.request();
        Self {
            mode: self.mode,
            peak_nits: r.peak_nits,
            scene_white_nits: r.scene_white_nits,
            ui_white_nits: r.ui_white_nits,
        }
    }
    pub fn request(self) -> blackbox_gfx::DisplaySettings {
        blackbox_gfx::DisplaySettings {
            hdr: match self.mode {
                DisplayMode::Off => blackbox_gfx::HdrMode::Off,
                DisplayMode::Auto => blackbox_gfx::HdrMode::Auto,
                DisplayMode::On => blackbox_gfx::HdrMode::On,
            },
            peak_nits: self.peak_nits,
            scene_white_nits: self.scene_white_nits,
            ui_white_nits: self.ui_white_nits,
        }
        .sanitized()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::{Partial, Settings, file, write};
    #[test]
    fn hdr_calibration_survives_style_quality_save_and_reload() {
        let mut settings = Settings::from(Partial::default());
        let display =
            DisplayTuning { mode: DisplayMode::On, peak_nits: 800.0, scene_white_nits: 150.0, ui_white_nits: 110.0 };
        settings.appearance.display = display;
        settings.select_visual_style(super::super::VisualStyle::OriginalPc);
        settings.select_graphics_quality(super::super::Quality::Low);
        let text =
            write::merge("game_dir='keep'\n[appearance.future]\noption=true\n", &settings.appearance_layer()).unwrap();
        let restored = Settings::from(file::parse(&text, "HDR round trip"));
        assert_eq!(restored.appearance.display, display);
        assert!(text.contains("keep") && text.contains("option = true"));
        let layer = file::parse(
            "[appearance.display]\nmode='auto'\npeak_nits=nan\nscene_white_nits=inf\nui_white_nits=-1\n",
            "invalid calibration",
        );
        let valid = Settings::from(layer).appearance.display;
        assert_eq!(valid.peak_nits, 0.0);
        assert_eq!(valid.scene_white_nits, 203.0);
        assert_eq!(valid.ui_white_nits, 16.0);
    }
}
