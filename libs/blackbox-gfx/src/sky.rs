//! Authored multilayer sky materials, independent of any game's texture names.

use crate::TextureHandle;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum SkyLayerKind {
    Base,
    Cloud2,
    Cloud3,
}

#[derive(Debug, Clone, Copy)]
pub struct SkyMaterialDesc {
    pub kind: SkyLayerKind,
    pub diffuse: TextureHandle,
    pub miscellaneous: TextureHandle,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SkyState {
    pub cloud_intensity: [f32; 4],
    pub scroll: f32,
    /// Independent procedural sky; authored layer weights remain untouched.
    pub enhanced: bool,
    /// Continuous weather coverage, including a genuinely clear zero.
    pub cloud_coverage: f32,
    pub sky_wind_speed: f32,
    pub sky_sun_size: f32,
    pub sky_sun_brightness: f32,
    pub sky_brightness: f32,
    pub sky_sunset_warmth: f32,
}

impl Default for SkyState {
    fn default() -> Self {
        Self {
            cloud_intensity: [1.0; 4],
            scroll: 0.0,
            enhanced: false,
            cloud_coverage: 0.0,
            sky_wind_speed: 2.0,
            sky_sun_size: 2.0,
            sky_sun_brightness: 1.0,
            sky_brightness: 1.0,
            sky_sunset_warmth: 1.0,
        }
    }
}

impl SkyState {
    pub fn sanitized(self) -> Self {
        fn finite(value: f32, default: f32) -> f32 {
            if value.is_finite() {
                return value;
            }
            default
        }
        Self {
            sky_wind_speed: finite(self.sky_wind_speed, 2.0).clamp(0.0, 12.0),
            sky_sun_size: finite(self.sky_sun_size, 2.0).clamp(0.5, 3.0),
            sky_sun_brightness: finite(self.sky_sun_brightness, 1.0).clamp(0.0, 3.0),
            sky_brightness: finite(self.sky_brightness, 1.0).clamp(0.25, 2.0),
            sky_sunset_warmth: finite(self.sky_sunset_warmth, 1.0).clamp(0.0, 2.0),
            ..self
        }
    }
}
