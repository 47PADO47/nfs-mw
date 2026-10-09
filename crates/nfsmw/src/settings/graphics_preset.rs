//! The `graphics_preset` setting: one switch that stands for a set of the individual cost settings
//! (docs/low-end.md). A preset is the layer *below* every explicit setting and above the built-in defaults, so
//! the order is command line > environment > config file > preset > defaults, per key.
//!
//! A preset is a set of *requests*. What a renderer actually runs is decided afterwards by its capabilities
//! (`blackbox_gfx::resolve`), so `ultra` on the native renderer is `high`: the temporal anti-aliasing and the ray
//! tracing it asks for fall back with a logged downgrade (docs/renderers.md).

use super::{
    CarShading, Partial, PostAa, PostBloom, PostTonemap, RayTracingLevel, RenderScale, Settings, SmokeQuality,
    UpscaleMode,
};

/// A named set of graphics settings.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum GraphicsPreset {
    /// No preset: every setting is what the layers and the built-in defaults say. The default.
    #[default]
    Custom,
    /// For weak GPUs and integrated graphics: simple car shading, no post effects, 75 % render scale.
    Low,
    /// The default look plus FXAA.
    Medium,
    /// FXAA, a little bloom, high-quality smoke and collision sparks.
    High,
    /// `high` plus temporal anti-aliasing and medium ray tracing, for the Bevy renderer on a strong GPU. The
    /// native renderer has neither, so there it is `high`.
    Ultra,
}

names!(
    GraphicsPreset,
    "custom, low, medium, high or ultra",
    [
        (Self::Custom, "custom"),
        (Self::Low, "low"),
        (Self::Medium, "medium"),
        (Self::High, "high"),
        (Self::Ultra, "ultra")
    ]
);

impl GraphicsPreset {
    pub const ALL: [Self; 5] = [Self::Custom, Self::Low, Self::Medium, Self::High, Self::Ultra];

    /// The settings this preset stands for. `Custom` sets none. Every other preset sets the same keys, so
    /// switching between presets in a menu never leaves a value of the previous one behind.
    pub fn layer(self) -> Partial {
        let medium = Partial {
            car_shading: Some(CarShading::Glossy),
            post_tonemap: Some(PostTonemap::Off),
            post_bloom: Some(PostBloom::Off),
            post_aa: Some(PostAa::Fxaa),
            render_scale: RenderScale::new(100),
            upscaler: Some(UpscaleMode::Fsr1),
            ray_tracing: Some(RayTracingLevel::Off),
            tire_smoke: Some(true),
            skid_marks: Some(true),
            smoke_quality: Some(SmokeQuality::Standard),
            collision_sparks: Some(false),
            speed_trails: Some(false),
            ..Partial::default()
        };
        let high = Partial {
            post_bloom: Some(PostBloom::Low),
            smoke_quality: Some(SmokeQuality::High),
            collision_sparks: Some(true),
            ..medium
        };
        match self {
            Self::Custom => Partial::default(),
            Self::Low => Partial {
                car_shading: Some(CarShading::Simple),
                post_aa: Some(PostAa::Off),
                render_scale: RenderScale::new(75),
                upscaler: Some(UpscaleMode::Bilinear),
                ..medium
            },
            Self::Medium => medium,
            Self::High => high,
            Self::Ultra => Partial { post_aa: Some(PostAa::Taa), ray_tracing: Some(RayTracingLevel::Medium), ..high },
        }
    }
}

impl Settings {
    /// The settings a preset covers, as a layer, for comparing with [`GraphicsPreset::layer`].
    pub(super) fn preset_keys(&self) -> Partial {
        Partial {
            car_shading: Some(self.car_shading),
            post_tonemap: Some(self.post_tonemap),
            post_bloom: Some(self.post_bloom),
            post_aa: Some(self.post_aa),
            render_scale: Some(self.render_scale),
            upscaler: Some(self.upscaler),
            ray_tracing: Some(self.ray_tracing),
            tire_smoke: Some(self.tire_smoke),
            skid_marks: Some(self.skid_marks),
            smoke_quality: Some(self.smoke_quality),
            collision_sparks: Some(self.collision_sparks),
            speed_trails: Some(self.speed_trails),
            ..Partial::default()
        }
    }

    /// Choose `preset` at run time (the menu row, the console): its settings replace the current ones.
    /// `Custom` only names the state and changes nothing else.
    pub fn apply_preset(&mut self, preset: GraphicsPreset) {
        self.graphics_preset = preset;
        let l = preset.layer();
        self.car_shading = l.car_shading.unwrap_or(self.car_shading);
        self.post_tonemap = l.post_tonemap.unwrap_or(self.post_tonemap);
        self.post_bloom = l.post_bloom.unwrap_or(self.post_bloom);
        self.post_aa = l.post_aa.unwrap_or(self.post_aa);
        self.render_scale = l.render_scale.unwrap_or(self.render_scale);
        self.upscaler = l.upscaler.unwrap_or(self.upscaler);
        self.ray_tracing = l.ray_tracing.unwrap_or(self.ray_tracing);
        self.tire_smoke = l.tire_smoke.unwrap_or(self.tire_smoke);
        self.skid_marks = l.skid_marks.unwrap_or(self.skid_marks);
        self.smoke_quality = l.smoke_quality.unwrap_or(self.smoke_quality);
        self.collision_sparks = l.collision_sparks.unwrap_or(self.collision_sparks);
        self.speed_trails = l.speed_trails.unwrap_or(self.speed_trails);
    }

    /// A preset names the state only while the settings it covers still match it: once one of them is
    /// changed the preset becomes `Custom`. Returns whether it did.
    pub fn settle_preset(&mut self) -> bool {
        if self.graphics_preset == GraphicsPreset::Custom || self.preset_keys() == self.graphics_preset.layer() {
            return false;
        }
        self.graphics_preset = GraphicsPreset::Custom;
        true
    }
}
