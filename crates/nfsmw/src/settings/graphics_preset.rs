//! The `graphics_preset` setting: one switch that stands for a set of the individual cost settings
//! (docs/low-end.md). A preset is the layer *below* every explicit setting and above the built-in defaults, so
//! the order is command line > environment > config file > preset > defaults, per key.

use std::fmt;
use std::str::FromStr;

use super::{CarShading, Partial, PostAa, PostBloom, PostTonemap, RenderScale, Settings, SmokeQuality, UpscaleMode};

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
}

impl GraphicsPreset {
    /// The settings this preset stands for. `Custom` sets none. Every preset sets the same keys, so
    /// switching between presets in a menu never leaves a value of the previous one behind.
    pub fn layer(self) -> Partial {
        let (shading, bloom, aa, scale, upscaler, smoke, sparks) = match self {
            Self::Custom => return Partial::default(),
            Self::Low => (
                CarShading::Simple,
                PostBloom::Off,
                PostAa::Off,
                75,
                UpscaleMode::Bilinear,
                SmokeQuality::Standard,
                false,
            ),
            Self::Medium => (
                CarShading::Glossy,
                PostBloom::Off,
                PostAa::Fxaa,
                100,
                UpscaleMode::Fsr1,
                SmokeQuality::Standard,
                false,
            ),
            Self::High => {
                (CarShading::Glossy, PostBloom::Low, PostAa::Fxaa, 100, UpscaleMode::Fsr1, SmokeQuality::High, true)
            }
        };
        Partial {
            car_shading: Some(shading),
            post_tonemap: Some(PostTonemap::Off),
            post_bloom: Some(bloom),
            post_aa: Some(aa),
            render_scale: RenderScale::new(scale),
            upscaler: Some(upscaler),
            tire_smoke: Some(true),
            skid_marks: Some(true),
            smoke_quality: Some(smoke),
            collision_sparks: Some(sparks),
            speed_trails: Some(false),
            ..Partial::default()
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

impl FromStr for GraphicsPreset {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, String> {
        match s {
            "custom" => Ok(Self::Custom),
            "low" => Ok(Self::Low),
            "medium" => Ok(Self::Medium),
            "high" => Ok(Self::High),
            _ => Err(format!("expected custom, low, medium or high, got {s:?}")),
        }
    }
}

impl fmt::Display for GraphicsPreset {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Custom => "custom",
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
        })
    }
}
