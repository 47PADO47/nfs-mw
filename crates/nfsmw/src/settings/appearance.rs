//! Canonical style/quality requests shared by the menu, F8 and render bridge.
pub mod display;
mod persistence;
pub mod registry;
pub(crate) mod shadows;
#[cfg(test)]
mod tests;

use super::{
    CarShading, GraphicsPreset, Partial, Percent, PostAa, PostBloom, PostTonemap, Settings, ShadowQuality,
    VisualTreatment,
};
use blackbox_gfx::{AoQuality, ReflectionQuality, RenderExperiments};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum VisualStyle {
    OriginalPc,
    Enhanced,
    #[default]
    Custom,
}
names!(
    VisualStyle,
    "original-pc, enhanced or custom",
    [(Self::OriginalPc, "original-pc"), (Self::Enhanced, "enhanced"), (Self::Custom, "custom")]
);
impl VisualStyle {
    pub const ALL: [Self; 3] = [Self::OriginalPc, Self::Enhanced, Self::Custom];
    pub fn label(self) -> &'static str {
        match self {
            Self::OriginalPc => "Original PC (incomplete)",
            Self::Enhanced => "Enhanced",
            Self::Custom => "Custom",
        }
    }
}
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Quality {
    Low,
    Balanced,
    #[default]
    High,
    VeryHigh,
}
names!(
    Quality,
    "low, balanced, high or very-high",
    [(Self::Low, "low"), (Self::Balanced, "balanced"), (Self::High, "high"), (Self::VeryHigh, "very-high")]
);
impl Quality {
    pub const ALL: [Self; 4] = [Self::Low, Self::Balanced, Self::High, Self::VeryHigh];
    pub fn offered(renderer: super::RendererKind) -> &'static [Self] {
        match renderer {
            super::RendererKind::Blackbox => &Self::ALL,
            super::RendererKind::Bevy => &[Self::Low, Self::Balanced, Self::High],
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Low => "Low",
            Self::Balanced => "Balanced",
            Self::High => "High",
            Self::VeryHigh => "Very High",
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Appearance {
    pub display: display::DisplayTuning,
    pub style: VisualStyle,
    pub quality: Quality,
    pub lighting: RenderExperiments,
    pub treatment_strength: Option<f32>,
}
impl Default for Appearance {
    fn default() -> Self {
        Self {
            display: Default::default(),
            style: Default::default(),
            quality: Default::default(),
            treatment_strength: None,
            lighting: RenderExperiments { original_world_materials: true, ..Default::default() },
        }
    }
}
impl Appearance {
    pub(crate) fn read(value: &toml::Value, origin: &str) -> Option<Self> {
        persistence::read(value, origin)
    }
    pub(crate) fn value(self) -> anyhow::Result<toml::Value> {
        persistence::value(self)
    }
    pub(crate) fn merge(self, existing: Option<&toml::Value>) -> anyhow::Result<toml::Value> {
        persistence::merge(existing, self.value()?)
    }
}
impl Settings {
    pub fn select_visual_style(&mut self, style: VisualStyle) {
        self.appearance.style = style;
        if style == VisualStyle::Custom {
            return;
        }
        let old = self.appearance.lighting;
        self.appearance.lighting = RenderExperiments {
            original_world_materials: true,
            freeze_sun: old.freeze_sun,
            lighting_debug: old.lighting_debug,
            ..Default::default()
        };
        self.appearance.treatment_strength = None;
        self.graphics_preset = GraphicsPreset::Custom;
        self.car_shading = CarShading::Glossy;
        self.post_bloom = PostBloom::Off;
        self.ao_compare = false;
        self.ao_intensity = Percent(100);
        match style {
            VisualStyle::OriginalPc => {
                self.car_shading = CarShading::Original;
                self.appearance.lighting.original_world_materials = true;
                self.appearance.lighting.sun_shadows = true;
                self.appearance.lighting.shadow_projection = Some(shadows::PROJECTION);
                self.visual_treatment = VisualTreatment::High;
                self.post_tonemap = PostTonemap::Off;
                self.post_aa = PostAa::Off;
                self.ao = AoQuality::Off;
            }
            VisualStyle::Enhanced => {
                self.visual_treatment = VisualTreatment::Low;
                self.post_tonemap = PostTonemap::Aces;
                self.post_aa = PostAa::Smaa;
                self.ao = AoQuality::High;
                self.ao_method = super::AoMethod::XeGtao;
                self.ao_intensity = Percent(80);
                self.ao_depth_mips = true;
                self.road_reflections = super::RoadReflections::Blurred;
                self.appearance.lighting = RenderExperiments {
                    sun_shadows: true,
                    car_skylight: true,
                    clearcoat: true,
                    original_shadows: false,
                    sky_strength: 0.76,
                    ground_strength: 1.12,
                    shadow_filter: 1.0,
                    sun_angle: 0.55,
                    ..self.appearance.lighting
                };
            }
            VisualStyle::Custom => unreachable!(),
        }
        self.select_graphics_quality(self.appearance.quality);
    }
    pub fn select_graphics_quality(&mut self, quality: Quality) {
        self.appearance.quality = quality;
        self.detail.world = match quality {
            Quality::Low => super::WorldDetail::Low,
            Quality::Balanced => super::WorldDetail::High,
            Quality::High | Quality::VeryHigh => super::WorldDetail::Maximum,
        };
        self.detail.car = match (quality, self.appearance.style) {
            (Quality::Low, _) => super::CarGeometry::Low,
            (Quality::High | Quality::VeryHigh, VisualStyle::Enhanced | VisualStyle::Custom) => {
                super::CarGeometry::Highest
            }
            _ => super::CarGeometry::High,
        };
        self.graphics_preset = GraphicsPreset::Custom;
        self.texture_filtering = super::TextureFiltering::Anisotropic16;
        let (size, range, reach, reflection, ao) = match quality {
            Quality::Low => (1024, 120.0, 0.25, ReflectionQuality::Low, AoQuality::Low),
            Quality::Balanced => (2048, 240.0, 0.5, ReflectionQuality::Medium, AoQuality::Medium),
            Quality::High | Quality::VeryHigh => (4096, 320.0, 0.8, ReflectionQuality::High, AoQuality::High),
        };
        self.reflections = reflection;
        if self.ao != AoQuality::Off {
            self.ao = ao;
        }
        if self.renderer == super::RendererKind::Bevy {
            self.shadows = match quality {
                Quality::Low => ShadowQuality::Low,
                Quality::Balanced => ShadowQuality::Medium,
                Quality::High | Quality::VeryHigh => ShadowQuality::High,
            };
            return;
        }
        if self.ao != AoQuality::Off && matches!(quality, Quality::High | Quality::VeryHigh) {
            self.ao_method = super::AoMethod::XeGtao;
            self.ao_intensity = Percent(80);
            self.ao_depth_mips = true;
        }
        let light = &mut self.appearance.lighting;
        if light.shadow_projection.is_none() {
            self.road_reflection_resolution = match quality {
                Quality::Low => super::RoadReflectionResolution::Original,
                Quality::Balanced | Quality::High => super::RoadReflectionResolution::Double,
                Quality::VeryHigh => super::RoadReflectionResolution::Quadruple,
            };
        }
        light.shadow_resolution = size;
        light.shadow_distance = range;
        light.contact_length = reach;
        light.soft_shadows = quality != Quality::Low;
        light.contact_shadows = light.sun_shadows && light.shadow_projection.is_none() && quality != Quality::Low;
        light.near_shadow_focus = false;
        light.wide_shadow_detail = light.shadow_projection.is_none() && quality != Quality::Low;
        light.shadow_detail_resolution = match quality {
            Quality::Low => 1024,
            Quality::Balanced | Quality::High => 4096,
            Quality::VeryHigh => 8192,
        };
        light.far_shadows = false;
        light.dense_shadow_filter = matches!(quality, Quality::High | Quality::VeryHigh);
        self.shadows = match (light.sun_shadows, quality) {
            (false, _) => ShadowQuality::Off,
            (true, Quality::Low) => ShadowQuality::Low,
            (true, Quality::Balanced) => ShadowQuality::Medium,
            (true, Quality::High | Quality::VeryHigh) => ShadowQuality::High,
        };
    }
    /// Baseline metadata survives overrides, while the visible selection reads Custom.
    pub fn visual_style(&self) -> VisualStyle {
        let style = self.appearance.style;
        if style == VisualStyle::Custom {
            return style;
        }
        let mut expected = *self;
        expected.select_visual_style(style);
        let mut lighting = self.appearance.lighting;
        lighting.freeze_sun = expected.appearance.lighting.freeze_sun;
        lighting.lighting_debug = expected.appearance.lighting.lighting_debug;
        if self.appearance.treatment_strength.is_some()
            || lighting != expected.appearance.lighting
            || self.car_shading != expected.car_shading
            || self.visual_treatment != expected.visual_treatment
            || self.post_tonemap != expected.post_tonemap
            || self.post_bloom != expected.post_bloom
            || self.post_aa != expected.post_aa
            || self.reflections != expected.reflections
            || self.ao != expected.ao
            || self.ao_intensity != expected.ao_intensity
        {
            return VisualStyle::Custom;
        }
        style
    }
    /// Fields owned by graphics profiles, deliberately excluding world and gameplay preferences.
    pub fn appearance_layer(&self) -> Partial {
        Partial {
            detail: Some(self.detail),
            appearance: Some(self.appearance),
            graphics_preset: Some(GraphicsPreset::Custom),
            car_shading: Some(self.car_shading),
            visual_treatment: Some(self.visual_treatment),
            post_tonemap: Some(self.post_tonemap),
            post_bloom: Some(self.post_bloom),
            post_aa: Some(self.post_aa),
            reflections: Some(self.reflections),
            texture_filtering: Some(self.texture_filtering),
            road_reflections: Some(self.road_reflections),
            road_reflection_resolution: Some(self.road_reflection_resolution),
            shadows: Some(self.shadows),
            ao: Some(self.ao),
            ao_intensity: Some(self.ao_intensity),
            ao_compare: Some(self.ao_compare),
            ao_depth_mips: Some(self.ao_depth_mips),
            ao_method: Some(self.ao_method),
            ..Default::default()
        }
    }

    /// Complete F8 snapshot, including renderer requests that are currently downgraded.
    /// Includes F8 video and traffic controls; renderer selection and audio remain outside the preset.
    pub fn graphics_layer(&self) -> Partial {
        Partial {
            backend: Some(self.backend),
            window_mode: Some(self.window_mode),
            monitor: Some(self.monitor),
            resolution: Some(self.resolution),
            reflex: Some(self.reflex),
            frame_generation: Some(self.frame_generation),
            vsync: Some(self.vsync),
            max_fps: Some(self.max_fps),
            show_metrics: Some(self.show_metrics),
            hd_content: self.hd_content.into(),
            custom_fonts: self.custom_fonts.into(),
            traffic_policy: Some(self.traffic_policy),
            traffic: Some(self.traffic),
            cop_share: Some(self.cop_share),
            traffic_lights: Some(self.traffic_lights),
            traffic_avoidance: Some(self.traffic_avoidance),
            reflection_cube_size: Some(self.reflection_cube_size),
            reflection_mirror_size: Some(self.reflection_mirror_size),
            reflection_pow2: Some(self.reflection_pow2),
            mirror_tint: Some(self.mirror_tint),
            reflection_distance: Some(self.reflection_extra.distance),
            reflection_lod: Some(self.reflection_extra.lod),
            reflection_content: Some(self.reflection_extra.content),
            vehicle_reflection_brightness: Some(self.reflection_extra.brightness),
            fe_shadows: self.fe_shadows.into(),
            motion_blur: Some(self.motion_blur),
            motion_blur_strength: Some(self.motion_blur_strength),
            render_scale: Some(self.render_scale),
            upscaler: Some(self.upscaler),
            upscale_sharpness: Some(self.upscale_sharpness),
            upscale_quality: Some(self.upscale_quality),
            ray_tracing: Some(self.ray_tracing),
            tire_smoke: Some(self.tire_smoke),
            skid_marks: Some(self.skid_marks),
            smoke_quality: Some(self.smoke_quality),
            collision_sparks: Some(self.collision_sparks),
            spark_style: Some(self.spark_style),
            speed_trails: Some(self.speed_trails),
            speed_trail_style: Some(self.speed_trail_style),
            exhaust_flames: Some(self.exhaust_flames),
            ..self.appearance_layer()
        }
    }
}
