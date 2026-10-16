//! The graphics-cost rows of the Video options (docs/low-end.md).

use blackbox_gfx::Capabilities;

use super::options::{Data, Title};
use crate::settings::reflection_res::{size_label, step_size, step_tint, tint_label};
use crate::settings::{
    AoQuality, CarShading, GraphicsPreset, Partial, Percent, ReflectionQuality, Settings, VisualTreatment,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GraphicsSetting {
    Preset,
    CarShading,
    VisualTreatment,
    Reflections,
    CubeSize,
    Pow2,
    MirrorSize,
    MirrorTint,
    Shadows,
    AmbientOcclusion,
    AoIntensity,
    AoComparison,
}

impl GraphicsSetting {
    pub const ALL: [Self; 12] = [
        Self::Preset,
        Self::CarShading,
        Self::VisualTreatment,
        Self::Reflections,
        Self::CubeSize,
        Self::Pow2,
        Self::MirrorSize,
        Self::MirrorTint,
        Self::Shadows,
        Self::AmbientOcclusion,
        Self::AoIntensity,
        Self::AoComparison,
    ];

    pub fn shown(self, caps: &Capabilities) -> bool {
        match self {
            Self::Shadows => caps.soft_shadows,
            Self::VisualTreatment => caps.scene_treatment,
            Self::Reflections | Self::CubeSize | Self::Pow2 | Self::MirrorSize | Self::MirrorTint => {
                caps.local_reflections
            }
            Self::AmbientOcclusion | Self::AoIntensity | Self::AoComparison => caps.ambient_occlusion,
            Self::Preset | Self::CarShading => true,
        }
    }

    pub fn enabled(self, s: &Settings) -> bool {
        match self {
            Self::Reflections => s.car_shading == CarShading::Glossy,
            Self::CubeSize | Self::Pow2 => {
                s.car_shading == CarShading::Glossy && s.reflections != ReflectionQuality::Off
            }
            Self::MirrorSize | Self::MirrorTint => true,
            Self::AoIntensity | Self::AoComparison => s.ao != AoQuality::Off,
            Self::Preset | Self::CarShading | Self::VisualTreatment | Self::AmbientOcclusion | Self::Shadows => true,
        }
    }

    pub fn title(self) -> Title {
        Title::Text(match self {
            Self::Preset => "Graphics Preset",
            Self::CarShading => "Car Shading",
            Self::VisualTreatment => "Visual Treatment (Experimental)",
            Self::Reflections => "Car Reflections",
            Self::CubeSize => "Car Reflection Size",
            Self::Pow2 => "Power-of-Two Cube Size",
            Self::MirrorSize => "Mirror Reflection Size",
            Self::MirrorTint => "Mirror Tint",
            Self::Shadows => "Dynamic Soft Shadows",
            Self::AmbientOcclusion => "Ambient Occlusion (Experimental)",
            Self::AoIntensity => "AO Intensity",
            Self::AoComparison => "AO Comparison",
        })
    }

    pub fn data(self, s: &Settings) -> Data {
        if self == Self::AoIntensity {
            return Data::Text(format!("{}%", s.ao_intensity.0));
        }
        if self == Self::CubeSize {
            return Data::Text(size_label(s.reflection_cube_size));
        }
        if self == Self::MirrorSize {
            return Data::Text(size_label(s.reflection_mirror_size));
        }
        Data::Text(
            match self {
                Self::Preset => match s.graphics_preset {
                    GraphicsPreset::Custom => "Custom",
                    GraphicsPreset::Low => "Low",
                    GraphicsPreset::Medium => "Medium",
                    GraphicsPreset::High => "High",
                    GraphicsPreset::Ultra => "Ultra",
                },
                Self::VisualTreatment => match s.visual_treatment {
                    VisualTreatment::Off => "Off",
                    VisualTreatment::Low => "Subtle",
                    VisualTreatment::High => "Original",
                },
                Self::Reflections => match s.reflections {
                    ReflectionQuality::Off => "Static Sky",
                    ReflectionQuality::Low => "Low",
                    ReflectionQuality::Medium => "Medium",
                    ReflectionQuality::High => "High",
                },
                Self::CubeSize | Self::MirrorSize => unreachable!("labelled above"),
                Self::Pow2 => match s.reflection_pow2 {
                    true => "On",
                    false => "Off",
                },
                Self::MirrorTint => tint_label(s.mirror_tint),
                Self::CarShading => match s.car_shading {
                    CarShading::Simple => "Simple",
                    CarShading::Glossy => "Glossy",
                },
                Self::Shadows => match s.shadows {
                    crate::settings::ShadowQuality::Off => "Off",
                    crate::settings::ShadowQuality::Low => "Low",
                    crate::settings::ShadowQuality::Medium => "Medium",
                    crate::settings::ShadowQuality::High => "High",
                },
                Self::AmbientOcclusion => match s.ao {
                    AoQuality::Off => "Off",
                    AoQuality::Low => "Low",
                    AoQuality::Medium => "Medium",
                    AoQuality::High => "High",
                },
                Self::AoComparison => match s.ao_compare {
                    true => "Split Screen",
                    false => "Off",
                },
                Self::AoIntensity => unreachable!("slider handled above"),
            }
            .to_owned(),
        )
    }

    /// Moves to the next (or previous) value, wrapping, and records the change for the config file.
    pub fn step(self, s: &mut Settings, changed: &mut Partial, forward: bool) -> bool {
        if !self.enabled(s) {
            return false;
        }
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
            Self::VisualTreatment => {
                s.visual_treatment = super::post_options::pick(&VisualTreatment::ALL, s.visual_treatment, forward);
                changed.visual_treatment = Some(s.visual_treatment);
            }
            Self::Reflections => {
                let all = ReflectionQuality::ALL;
                let at = all.iter().position(|q| *q == s.reflections).unwrap_or(0);
                let offset = match forward {
                    true => 1,
                    false => all.len() - 1,
                };
                s.reflections = all[(at + offset) % all.len()];
                changed.reflections = Some(s.reflections);
            }
            Self::CubeSize => {
                s.reflection_cube_size = step_size(s.reflection_cube_size, forward);
                changed.reflection_cube_size = Some(s.reflection_cube_size);
            }
            Self::MirrorSize => {
                s.reflection_mirror_size = step_size(s.reflection_mirror_size, forward);
                changed.reflection_mirror_size = Some(s.reflection_mirror_size);
            }
            Self::Pow2 => {
                s.reflection_pow2 = !s.reflection_pow2;
                changed.reflection_pow2 = Some(s.reflection_pow2);
            }
            Self::MirrorTint => {
                s.mirror_tint = step_tint(s.mirror_tint);
                changed.mirror_tint = Some(s.mirror_tint);
            }
            Self::CarShading => {
                s.car_shading = match s.car_shading {
                    CarShading::Simple => CarShading::Glossy,
                    CarShading::Glossy => CarShading::Simple,
                };
                changed.car_shading = Some(s.car_shading);
            }
            Self::Shadows => {
                s.shadows = super::post_options::pick(&crate::settings::ShadowQuality::ALL, s.shadows, forward);
                changed.shadows = Some(s.shadows);
            }
            Self::AmbientOcclusion => {
                let at = AoQuality::ALL.iter().position(|q| *q == s.ao).unwrap_or(0);
                let offset = match forward {
                    true => 1,
                    false => AoQuality::ALL.len() - 1,
                };
                s.ao = AoQuality::ALL[(at + offset) % AoQuality::ALL.len()];
                changed.ao = Some(s.ao);
            }
            Self::AoIntensity => {
                s.ao_intensity = Percent(match forward {
                    true => s.ao_intensity.0.saturating_add(10).min(100),
                    false => s.ao_intensity.0.saturating_sub(10),
                });
                changed.ao_intensity = Some(s.ao_intensity);
            }
            Self::AoComparison => {
                s.ao_compare = !s.ao_compare;
                changed.ao_compare = Some(s.ao_compare);
            }
        }
        before != *s
    }
}

#[cfg(test)]
mod tests {
    use super::super::options::Setting;
    use super::*;
    use crate::settings::ReflectionSize;

    #[test]
    fn shadows_are_shown_only_on_supported_renderers_and_record_changes() {
        let mut s = Settings::from(Partial::default());
        let mut changed = Partial::default();
        assert!(!GraphicsSetting::Shadows.shown(&crate::settings::test_caps::native()));
        assert!(GraphicsSetting::Shadows.shown(&crate::settings::test_caps::full()));
        assert!(GraphicsSetting::Shadows.step(&mut s, &mut changed, true));
        assert_eq!(s.shadows, crate::settings::ShadowQuality::Low);
        assert_eq!(changed.shadows, Some(crate::settings::ShadowQuality::Low));
    }

    #[test]
    fn ao_rows_cycle_clamp_and_persist_independently() {
        let (mut s, mut changed) = (Settings::from(Partial::default()), Partial::default());
        for quality in [AoQuality::Low, AoQuality::Medium, AoQuality::High, AoQuality::Off] {
            GraphicsSetting::AmbientOcclusion.step(&mut s, &mut changed, true);
            assert_eq!((s.ao, changed.ao), (quality, Some(quality)));
        }
        GraphicsSetting::AmbientOcclusion.step(&mut s, &mut changed, false);
        assert_eq!(s.ao, AoQuality::High);
        for _ in 0..12 {
            GraphicsSetting::AoIntensity.step(&mut s, &mut changed, true);
        }
        assert_eq!(
            Setting::Graphics(GraphicsSetting::AoIntensity).control(&s),
            super::super::options::Control::Slider(100)
        );
        assert!(!GraphicsSetting::AoIntensity.step(&mut s, &mut changed, true));
        for _ in 0..12 {
            GraphicsSetting::AoIntensity.step(&mut s, &mut changed, false);
        }
        assert_eq!(changed.ao_intensity, Some(Percent(0)));
        GraphicsSetting::AoComparison.step(&mut s, &mut changed, true);
        assert_eq!((s.ao_compare, changed.ao_compare), (true, Some(true)));
    }

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
    fn the_reflection_rows_step_and_follow_car_reflections() {
        let mut s = Settings::from(Partial::default());
        let mut changed = Partial::default();
        assert_eq!(GraphicsSetting::CubeSize.data(&s), Data::Text("100%".into()));
        assert!(GraphicsSetting::CubeSize.step(&mut s, &mut changed, true));
        assert_eq!(s.reflection_cube_size, ReflectionSize::Scale(1.5));
        assert_eq!(changed.reflection_cube_size, Some(ReflectionSize::Scale(1.5)));
        assert!(GraphicsSetting::MirrorTint.step(&mut s, &mut changed, true));
        assert_eq!(GraphicsSetting::MirrorTint.data(&s), Data::Text("Stock".into()));
        s.reflections = ReflectionQuality::Off;
        assert!(!GraphicsSetting::Pow2.enabled(&s), "no cube capture to round");
        assert!(GraphicsSetting::MirrorSize.enabled(&s), "the mirror does not need car reflections");
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
