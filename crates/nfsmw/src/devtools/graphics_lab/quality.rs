//! Complete session profiles shared by F8 and startup launch requests.
use super::*;
use crate::settings::{CarShading, GraphicsPreset, Percent, RenderScale, UpscaleMode};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Quality {
    Low,
    Balanced,
    High,
}

impl Quality {
    pub(super) fn parse(name: &str) -> Option<Self> {
        match name {
            "low" => Some(Self::Low),
            "balanced" => Some(Self::Balanced),
            "high" => Some(Self::High),
            _ => None,
        }
    }

    pub(super) fn apply(self, settings: &mut Settings) -> RenderExperiments {
        // Keep the chosen art treatment and native render scale across quality levels.
        settings.graphics_preset = GraphicsPreset::Custom;
        settings.car_shading = CarShading::Glossy;
        settings.render_scale = RenderScale::new(100).expect("valid native scale");
        settings.upscaler = UpscaleMode::Fsr1;
        settings.post_tonemap = PostTonemap::Aces;
        settings.post_bloom = PostBloom::Off;
        settings.post_aa = PostAa::Fxaa;
        settings.visual_treatment = VisualTreatment::Low;
        settings.ao_intensity = Percent(100);
        settings.ao_compare = false;
        let (resolution, range, soft, contact, reach, coat, reflections, ao) = match self {
            Self::Low => (1024, 120.0, false, false, 0.25, false, ReflectionQuality::Low, AoQuality::Off),
            Self::Balanced => (2048, 200.0, true, true, 0.3, true, ReflectionQuality::Medium, AoQuality::Medium),
            Self::High => (4096, 320.0, true, true, 0.425, true, ReflectionQuality::High, AoQuality::High),
        };
        settings.reflections = reflections;
        settings.ao = ao;
        RenderExperiments {
            sun_shadows: true,
            far_shadows: false,
            shadow_resolution: resolution,
            shadow_distance: range,
            shadow_filter: 0.7,
            soft_shadows: soft,
            sun_angle: 0.51,
            contact_shadows: contact,
            contact_length: reach,
            clearcoat: coat,
            car_skylight: true,
            sky_strength: 0.32,
            ground_strength: 0.55,
            world_ambient: false,
            original_shadows: false,
            freeze_sun: false,
            lighting_debug: LightingDebug::Off,
            ..Default::default()
        }
    }
}

/// Applied once before renderer creation. Normal launches leave saved settings alone.
pub(super) fn startup(mut lab: ResMut<GraphicsLab>, mut settings: ResMut<Settings>) {
    let Some(quality) = lab.startup_quality.take() else { return };
    lab.options = quality.apply(&mut settings);
    lab.remember_startup(&settings);
}

#[cfg(test)]
mod tests;
