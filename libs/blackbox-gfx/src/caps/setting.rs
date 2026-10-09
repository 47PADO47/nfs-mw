//! Names of the settings a renderer can downgrade or ask a restart for.

use std::fmt;

use crate::GraphicsSettings;

use super::RestartSet;

/// One user-facing graphics setting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Setting {
    Antialiasing,
    Upscaler,
    UpscaleQuality,
    RenderScale,
    Tonemap,
    Bloom,
    RayTracing,
}

impl Setting {
    pub const ALL: [Setting; 7] = [
        Setting::Antialiasing,
        Setting::Upscaler,
        Setting::UpscaleQuality,
        Setting::RenderScale,
        Setting::Tonemap,
        Setting::Bloom,
        Setting::RayTracing,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Self::Antialiasing => "antialiasing",
            Self::Upscaler => "upscaler",
            Self::UpscaleQuality => "upscale quality",
            Self::RenderScale => "render scale",
            Self::Tonemap => "tone mapping",
            Self::Bloom => "bloom",
            Self::RayTracing => "ray tracing",
        }
    }
}

impl fmt::Display for Setting {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

impl GraphicsSettings {
    /// The settings that differ between `self` and `other`. Exposure, bloom threshold and sharpness
    /// are tuning values of a setting, not settings, so they never count.
    pub fn changed_settings(&self, other: &Self) -> RestartSet {
        let mut changed = RestartSet::empty();
        let mut note = |setting: Setting, differs: bool| {
            if differs {
                changed.insert(setting);
            }
        };
        note(Setting::Antialiasing, self.post.antialiasing != other.post.antialiasing);
        note(Setting::Upscaler, self.upscaler != other.upscaler);
        note(Setting::UpscaleQuality, self.upscale_quality != other.upscale_quality);
        note(Setting::RenderScale, self.render_scale != other.render_scale);
        note(Setting::Tonemap, self.post.tonemap != other.post.tonemap);
        note(Setting::Bloom, (self.post.bloom_intensity > 0.0) != (other.post.bloom_intensity > 0.0));
        note(Setting::RayTracing, self.ray_tracing != other.ray_tracing);
        changed
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Antialiasing, RayTracing, Upscaler};

    #[test]
    fn identical_settings_have_no_changes() {
        let s = GraphicsSettings::default();
        assert!(s.changed_settings(&s).is_empty());
    }

    #[test]
    fn each_setting_is_reported_once() {
        let base = GraphicsSettings::default();
        let mut other = base;
        other.post.antialiasing = Antialiasing::Taa;
        other.upscaler = Upscaler::Fsr3;
        other.ray_tracing = RayTracing::Low;
        other.post.bloom_intensity = 0.5;
        assert_eq!(
            base.changed_settings(&other),
            RestartSet::of(&[Setting::Antialiasing, Setting::Upscaler, Setting::Bloom, Setting::RayTracing])
        );
    }

    #[test]
    fn tuning_values_are_not_settings() {
        let base = GraphicsSettings::default();
        let mut other = base;
        other.post.exposure = 2.0;
        other.upscale_sharpness = 0.1;
        other.post.bloom_threshold = 1.0;
        assert!(base.changed_settings(&other).is_empty());
        other.post.bloom_intensity = 0.2;
        let mut louder = other;
        louder.post.bloom_intensity = 0.4;
        assert!(other.changed_settings(&louder).is_empty(), "still on");
    }

    #[test]
    fn names_are_lower_case() {
        assert!(Setting::ALL.iter().all(|s| s.name() == s.name().to_lowercase()));
        assert_eq!(Setting::RenderScale.to_string(), "render scale");
    }
}
