//! The graphics-cost rows of the Video options (docs/low-end.md).

use super::options::{Data, Title};
use crate::settings::{CarShading, GraphicsPreset, Partial, Settings};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GraphicsSetting {
    Preset,
    CarShading,
}

impl GraphicsSetting {
    pub const ALL: [Self; 2] = [Self::Preset, Self::CarShading];

    pub fn title(self) -> Title {
        Title::Text(match self {
            Self::Preset => "Graphics Preset",
            Self::CarShading => "Car Shading",
        })
    }

    pub fn data(self, s: &Settings) -> Data {
        Data::Text(
            match self {
                Self::Preset => match s.graphics_preset {
                    GraphicsPreset::Custom => "Custom",
                    GraphicsPreset::Low => "Low",
                    GraphicsPreset::Medium => "Medium",
                    GraphicsPreset::High => "High",
                    GraphicsPreset::Ultra => "Ultra",
                },
                Self::CarShading => match s.car_shading {
                    CarShading::Simple => "Simple",
                    CarShading::Glossy => "Glossy",
                },
            }
            .to_owned(),
        )
    }

    /// Moves to the next (or previous) value, wrapping, and records the change for the config file.
    pub fn step(self, s: &mut Settings, changed: &mut Partial, forward: bool) -> bool {
        let before = *s;
        match self {
            Self::Preset => {
                let all = GraphicsPreset::ALL;
                let at = all.iter().position(|p| *p == s.graphics_preset).unwrap_or(0);
                let next = if forward { (at + 1) % all.len() } else { (at + all.len() - 1) % all.len() };
                s.apply_preset(all[next]);
                // Every key the preset sets is written, so the file says what the player sees.
                *changed = Partial { graphics_preset: Some(s.graphics_preset), ..all[next].layer() }.or(*changed);
            }
            Self::CarShading => {
                s.car_shading = match s.car_shading {
                    CarShading::Simple => CarShading::Glossy,
                    CarShading::Glossy => CarShading::Simple,
                };
                changed.car_shading = Some(s.car_shading);
            }
        }
        before != *s
    }
}

#[cfg(test)]
mod tests {
    use super::super::options::Setting;
    use super::*;

    #[test]
    fn the_preset_row_applies_a_preset_and_records_all_its_keys() {
        let mut s = Settings::from(Partial::default());
        let mut changed = Partial::default();
        assert_eq!(GraphicsSetting::Preset.data(&s), Data::Text("Custom".into()));
        assert!(GraphicsSetting::Preset.step(&mut s, &mut changed, true));
        assert_eq!(GraphicsSetting::Preset.data(&s), Data::Text("Low".into()));
        assert_eq!((s.car_shading, s.render_scale.percent()), (CarShading::Simple, 75));
        assert_eq!(
            (changed.graphics_preset, changed.car_shading),
            (Some(GraphicsPreset::Low), Some(CarShading::Simple))
        );
        assert!(changed.render_scale.is_some() && changed.post_aa.is_some());
        assert!(GraphicsSetting::Preset.step(&mut s, &mut changed, false));
        assert_eq!(s.graphics_preset, GraphicsPreset::Custom);
        assert_eq!(s.car_shading, CarShading::Simple, "custom keeps what the last preset set");
    }

    #[test]
    fn car_shading_toggles_and_records_only_its_change() {
        let mut s = Settings::from(Partial::default());
        let mut changed = Partial::default();
        assert_eq!(GraphicsSetting::CarShading.data(&s), Data::Text("Glossy".into()));
        assert!(GraphicsSetting::CarShading.step(&mut s, &mut changed, true));
        assert_eq!(GraphicsSetting::CarShading.data(&s), Data::Text("Simple".into()));
        assert_eq!(changed, Partial { car_shading: Some(CarShading::Simple), ..Partial::default() });
        assert!(GraphicsSetting::CarShading.step(&mut s, &mut changed, false));
        assert_eq!(s.car_shading, CarShading::Glossy);
    }

    #[test]
    fn changing_a_covered_setting_after_a_preset_makes_the_row_custom() {
        let (mut s, mut changes) = (Settings::from(Partial::default()), Partial::default());
        s.apply_preset(GraphicsPreset::High);
        Setting::TireSmoke.step(&mut s, &mut changes, true);
        assert_eq!(s.graphics_preset, GraphicsPreset::Custom);
        assert_eq!(changes.graphics_preset, Some(GraphicsPreset::Custom));
        let mut changes = Partial::default();
        s.apply_preset(GraphicsPreset::Low);
        Setting::Vsync.step(&mut s, &mut changes, true);
        assert_eq!((s.graphics_preset, changes.graphics_preset), (GraphicsPreset::Low, None), "unrelated rows keep it");
    }
}
