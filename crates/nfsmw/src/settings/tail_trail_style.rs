//! Independent NOS lamp trails, retained across cars and graphics presets.
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TrailShape {
    Ribbon,
    #[default]
    FlareRibbon,
}
impl std::fmt::Display for TrailShape {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Ribbon => "Thin ribbon",
            Self::FlareRibbon => "Flare + ribbon",
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct TailTrailStyle {
    pub enabled: bool,
    pub enhanced: bool,
    pub shape: TrailShape,
    pub length: u16,
    pub exposure: u16,
    pub width: u16,
    pub brightness: u16,
    pub glow: u16,
    pub lamp_glow: u16,
    pub lamp_bloom: u16,
    pub engagement: u16,
    pub follow_lamp: bool,
    pub head_size: u16,
    pub speed_response: u16,
    pub softness: u16,
    pub turbulence: u16,
    pub tint: [u8; 3],
}

impl Default for TailTrailStyle {
    fn default() -> Self {
        Self {
            enabled: true,
            enhanced: false,
            shape: TrailShape::default(),
            length: 100,
            exposure: 100,
            width: 100,
            brightness: 100,
            glow: 100,
            lamp_glow: 100,
            lamp_bloom: 100,
            engagement: 100,
            follow_lamp: true,
            head_size: 100,
            speed_response: 100,
            softness: 100,
            turbulence: 100,
            tint: [255, 45, 20],
        }
    }
}

impl TailTrailStyle {
    pub fn validate(&self) -> anyhow::Result<()> {
        for (name, value) in [
            ("length", self.length),
            ("exposure", self.exposure),
            ("width", self.width),
            ("brightness", self.brightness),
            ("glow", self.glow),
            ("lamp glow", self.lamp_glow),
            ("lamp bloom", self.lamp_bloom),
            ("engagement", self.engagement),
            ("head size", self.head_size),
            ("speed response", self.speed_response),
            ("softness", self.softness),
            ("turbulence", self.turbulence),
        ] {
            anyhow::ensure!(value <= 200, "NOS trail {name} is outside 0..=200%");
        }
        Ok(())
    }

    /// Smooth, bounded response: 0% disables speed influence, not the effect itself.
    pub fn speed_gain(self, speed: f32) -> f32 {
        let t = ((speed.abs() - 15.0) / 75.0).clamp(0.0, 1.0);
        let target = 0.7 + 0.5 * t * t * (3.0 - 2.0 * t);
        (1.0 + (target - 1.0) * f32::from(self.speed_response.min(200)) / 100.0).clamp(0.4, 1.4)
    }
    pub fn lamp_level(self, speed: f32) -> f32 {
        if !self.enabled || !self.enhanced {
            return 0.0;
        }
        (f32::from(self.lamp_glow.min(200)) / 100.0 * self.speed_gain(speed)).clamp(0.0, 2.0)
    }
}

pub(super) fn parse(value: &toml::Value) -> Option<TailTrailStyle> {
    let result = value.clone().try_into::<TailTrailStyle>().map_err(anyhow::Error::from).and_then(|style| {
        style.validate()?;
        Ok(style)
    });
    match result {
        Ok(style) => Some(style),
        Err(error) => {
            log::warn!("ignoring tail_trail_style: {error}");
            None
        }
    }
}

/// Session-only controls. Never serialized with appearance settings.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TailPreview {
    pub speed_kmh: u16,
    pub bend: i16,
    pub cycle: bool,
}
impl Default for TailPreview {
    fn default() -> Self {
        Self { speed_kmh: 240, bend: 0, cycle: false }
    }
}

mod presets;
pub use presets::TrailPreset;
