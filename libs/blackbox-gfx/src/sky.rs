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
    /// Optional celestial direction distinct from the scene light.
    pub celestial_sun: Option<[f32; 3]>,
    pub moon_direction: [f32; 3],
    pub night_amount: f32,
    /// What the panoramic sky shows now, when [`SkyPanorama`] pictures are set and the procedural sky is on.
    pub panorama: Option<SkyPanoramaState>,
}

/// The pictures of a panoramic sky: two cloud domes the sky blends between, and the moon. A dome is an equirectangular picture
/// of the upper hemisphere: the full width is 360 degrees of azimuth, the top row the zenith and the bottom row the horizon;
/// the colour is the light of the clouds and the alpha their cover. The moon picture is a square with the disc in the middle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SkyPanorama {
    pub a: TextureHandle,
    pub b: TextureHandle,
    pub moon: Option<TextureHandle>,
}

/// How the panorama is drawn this frame (`SkyPanorama`): the blend from `a` to `b` and what each is lit by.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SkyPanoramaState {
    /// 0 is the first dome, 1 the second.
    pub blend: f32,
    /// How far each dome is turned about the vertical, in turns of the full circle.
    pub rotation: [f32; 2],
    /// What each dome's linear colour is multiplied by, the exposure included.
    pub tint: [[f32; 3]; 2],
    /// A multiplier of the clouds' cover.
    pub gain: f32,
    /// Half the moon picture's width as an angle in radians, and its brightness.
    pub moon_size: f32,
    pub moon_brightness: f32,
}

impl SkyPanoramaState {
    fn sanitized(self) -> Self {
        let finite = |value: f32, default: f32| match value.is_finite() {
            true => value,
            false => default,
        };
        Self {
            blend: finite(self.blend, 0.0).clamp(0.0, 1.0),
            rotation: self.rotation.map(|turns| finite(turns, 0.0).rem_euclid(1.0)),
            tint: self.tint.map(|tint| tint.map(|value| finite(value, 0.0).clamp(0.0, 16.0))),
            gain: finite(self.gain, 1.0).clamp(0.0, 8.0),
            moon_size: finite(self.moon_size, 0.0173).clamp(0.002, 0.1),
            moon_brightness: finite(self.moon_brightness, 1.0).clamp(0.0, 8.0),
        }
    }
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
            celestial_sun: None,
            moon_direction: [0.0, 0.0, 1.0],
            night_amount: 0.0,
            panorama: None,
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
            sky_brightness: finite(self.sky_brightness, 1.0).clamp(0.05, 2.0),
            sky_sunset_warmth: finite(self.sky_sunset_warmth, 1.0).clamp(0.0, 2.0),
            celestial_sun: self
                .celestial_sun
                .filter(|v| v.iter().all(|x| x.is_finite()) && v.iter().map(|x| x * x).sum::<f32>() > 1e-6),
            moon_direction: match self.moon_direction.iter().all(|x| x.is_finite())
                && self.moon_direction.iter().map(|x| x * x).sum::<f32>() > 1e-6
            {
                true => self.moon_direction,
                false => [0.0, 0.0, 1.0],
            },
            night_amount: finite(self.night_amount, 0.0).clamp(0.0, 1.0),
            panorama: self.panorama.map(SkyPanoramaState::sanitized),
            ..self
        }
    }
}

/// Caller-owned optical artwork. The native pass is main-view only, before display mapping.
#[derive(Debug, Clone, PartialEq)]
pub struct SunGlare {
    /// Direction towards the drawn sun, when it differs from the scene light (`None`: opposite `light_dir`).
    pub to_sun: Option<[f32; 3]>,
    pub layers: Vec<SunGlareLayer>,
    pub colour: [f32; 3],
    pub strength: f32,
    /// Used when no procedural-cloud transmittance map is available.
    pub weather_visibility: f32,
    pub angular_radius: f32,
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SunGlareLayer {
    pub texture: TextureHandle,
    /// Half-width as a fraction of view height.
    pub radius: f32,
    pub brightness: f32,
    pub rotation: f32,
}
