//! Glossy shading: the lighting rig, per-material constants and the environment, for models
//! (vehicle bodies, rims, glass) that shine.
//!
//! A glossy draw lights its texture with three directional lights, adds a sun-style highlight
//! and mixes in a reflection of the environment cube map, with every ramp driven by how much
//! the surface faces the viewer. See `docs/specs/car-assembly.md` §8 for the maths.

use glam::Vec3;

/// One directional light.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DirectionalLight {
    /// Unit vector from the surface towards the light (world space).
    pub to_light: Vec3,
    /// Linear RGB intensity.
    pub color: Vec3,
}

impl DirectionalLight {
    /// A light shining from `to_light` (any length; a zero vector gives a light from above).
    pub fn new(to_light: Vec3, color: Vec3) -> Self {
        Self { to_light: to_light.try_normalize().unwrap_or(Vec3::Z), color }
    }
}

/// The three lights every glossy surface is lit by. The first is the sun: it also casts the
/// highlight.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LightingRig {
    pub lights: [DirectionalLight; 3],
    /// Added to the summed light colour (0 in the original shader; a way to lift shadows).
    pub ambient: Vec3,
}

impl Default for LightingRig {
    /// A neutral studio rig: a warm key from above and the front, a cool fill from the left and a
    /// back light from the right.
    fn default() -> Self {
        Self {
            lights: [
                DirectionalLight::new(Vec3::new(0.35, -0.3, 1.0), Vec3::new(0.95, 0.92, 0.86)),
                DirectionalLight::new(Vec3::new(-0.2, 1.0, 0.35), Vec3::new(0.35, 0.4, 0.5)),
                DirectionalLight::new(Vec3::new(-1.0, -0.5, 0.2), Vec3::new(0.3, 0.3, 0.34)),
            ],
            ambient: Vec3::ZERO,
        }
    }
}

/// Shading constants of one material. Each ramp runs from its `min` where the surface is seen
/// edge-on to `min + range` where it faces the viewer head-on.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GlossyMaterial {
    /// RGBA multiplier of the lit texture; alpha scales the highlight and the reflection, and
    /// the blended draw's opacity.
    pub diffuse_min: [f32; 4],
    pub diffuse_range: [f32; 4],
    pub specular_min: [f32; 3],
    pub specular_range: [f32; 3],
    /// Exponent of the facing term in the specular ramp.
    pub specular_power: f32,
    /// Strength of the environment reflection.
    pub envmap_min: f32,
    pub envmap_range: f32,
    /// Exponent of the facing term in the reflection ramp.
    pub envmap_power: f32,
}

impl Default for GlossyMaterial {
    /// A plain, slightly shiny surface.
    fn default() -> Self {
        Self {
            diffuse_min: [1.0; 4],
            diffuse_range: [0.0; 4],
            specular_min: [0.2; 3],
            specular_range: [0.0; 3],
            specular_power: 2.0,
            envmap_min: 0.1,
            envmap_range: 0.0,
            envmap_power: 1.0,
        }
    }
}

/// A sky for the environment reflection: three colours blended by the direction's height
/// (+z is up).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SkyGradient {
    pub zenith: [f32; 3],
    pub horizon: [f32; 3],
    pub ground: [f32; 3],
}

impl Default for SkyGradient {
    /// A daylight sky over a grey road.
    fn default() -> Self {
        Self { zenith: [0.30, 0.48, 0.78], horizon: [0.78, 0.84, 0.90], ground: [0.22, 0.23, 0.25] }
    }
}

/// What the glossy environment reflection shows.
#[derive(Debug, Clone, Copy)]
pub enum Environment<'a> {
    /// A procedural sky.
    Sky(SkyGradient),
    /// A cube map: six square RGBA8 faces of `size` by `size` pixels, in the order +X, -X, +Y, -Y, +Z, -Z.
    Faces { size: u32, faces: [&'a [u8]; 6] },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lights_are_normalised() {
        let light = DirectionalLight::new(Vec3::new(0.0, 0.0, 5.0), Vec3::ONE);
        assert!((light.to_light - Vec3::Z).length() < 1e-6);
        assert_eq!(DirectionalLight::new(Vec3::ZERO, Vec3::ONE).to_light, Vec3::Z);
        for light in LightingRig::default().lights {
            assert!((light.to_light.length() - 1.0).abs() < 1e-6);
        }
    }

    #[test]
    fn default_rig_key_light_is_above() {
        assert!(LightingRig::default().lights[0].to_light.z > 0.5);
    }
}
