//! The post-processing settings: tone mapping, bloom and anti-aliasing (docs/post-processing.md).
//! Each is independent and every default is off, which keeps the image exactly as it was.

use blackbox_gfx::{Antialiasing, PostSettings, Tonemap};

use super::Settings;

/// Bloom strength at each step of the setting (the renderer's `bloom_intensity`).
const LOW_BLOOM: f32 = 0.5;
const MEDIUM_BLOOM: f32 = 1.0;
const HIGH_BLOOM: f32 = 1.8;

/// The `post_tonemap` setting: the curve that maps the scene to the display range.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum PostTonemap {
    /// No curve: values are clamped, as before the setting existed.
    #[default]
    Off,
    /// A filmic ACES curve. It adds contrast and darkens the image noticeably.
    Aces,
}

/// The `post_bloom` setting: glow around bright areas.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum PostBloom {
    #[default]
    Off,
    Low,
    Medium,
    High,
}

/// The `post_aa` setting: anti-aliasing of the 3D scene (the HUD and menus are never touched).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum PostAa {
    #[default]
    Off,
    Fxaa,
}

impl PostBloom {
    /// The renderer's bloom intensity; 0.0 means off.
    pub fn intensity(self) -> f32 {
        match self {
            Self::Off => 0.0,
            Self::Low => LOW_BLOOM,
            Self::Medium => MEDIUM_BLOOM,
            Self::High => HIGH_BLOOM,
        }
    }
}

/// What the renderer is asked to run for `settings`.
pub fn post_effects(settings: &Settings) -> PostSettings {
    PostSettings {
        tonemap: match settings.post_tonemap {
            PostTonemap::Off => Tonemap::Off,
            PostTonemap::Aces => Tonemap::Aces,
        },
        bloom_intensity: settings.post_bloom.intensity(),
        antialiasing: match settings.post_aa {
            PostAa::Off => Antialiasing::Off,
            PostAa::Fxaa => Antialiasing::Fxaa,
        },
        ..PostSettings::default()
    }
}

names!(PostTonemap, "off or aces", [(Self::Off, "off"), (Self::Aces, "aces")]);
names!(
    PostBloom,
    "off, low, medium or high",
    [(Self::Off, "off"), (Self::Low, "low"), (Self::Medium, "medium"), (Self::High, "high")]
);
names!(PostAa, "off or fxaa", [(Self::Off, "off"), (Self::Fxaa, "fxaa")]);
