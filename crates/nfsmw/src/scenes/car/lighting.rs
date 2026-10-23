//! Custom light rig; selectable original rigs are in `original_lighting`.
//!
//! The game's rig is four lights given as angles and colours, but no rig is stored in the
//! install files read so far, so this builds three directional lights around the sun: the
//! sun itself as the key, a cool fill on the far side and a dim back light. Angles and colours
//! are choices, not game values.

use blackbox_gfx::{DirectionalLight, LightingRig};
use glam::Vec3;

/// Fill light: degrees around the sun's azimuth and elevation above the horizon.
const FILL: (f32, f32) = (150.0, 25.0);
/// Back light, as above.
const BACK: (f32, f32) = (-110.0, 35.0);

const KEY_COLOR: Vec3 = Vec3::new(0.95, 0.92, 0.84);
const FILL_COLOR: Vec3 = Vec3::new(0.38, 0.43, 0.52);
const BACK_COLOR: Vec3 = Vec3::new(0.30, 0.32, 0.38);

/// The rig for a sun in direction `to_sun` (from the car towards the sun; +z is up).
pub fn rig(to_sun: Vec3) -> LightingRig {
    LightingRig {
        lights: [
            DirectionalLight::new(to_sun, KEY_COLOR),
            DirectionalLight::new(around(to_sun, FILL), FILL_COLOR),
            DirectionalLight::new(around(to_sun, BACK), BACK_COLOR),
        ],
        ambient: Vec3::ZERO,
        material_specular_scale: 1.0,
    }
}

/// Custom world response follows the same authored colour/intensity as scenery.
/// The installed values are already display-space lighting multipliers, like the
/// legacy custom rig. Do not apply another transfer function here.
pub fn world_rig(to_sun: Vec3, light: Option<blackbox_gfx::SurfaceLight>) -> LightingRig {
    let Some(light) = light else { return rig(to_sun) };
    let mut result = rig(to_sun);
    result.lights[0].color = Vec3::from_array(light.diffuse).clamp(Vec3::ZERO, Vec3::ONE);
    let ambient = Vec3::from_array(light.ambient).clamp(Vec3::ZERO, Vec3::ONE) * light.ambient_alpha.clamp(0.0, 1.0);
    // Without probe irradiance the two directional fills follow authored ambient.
    // With probe irradiance enabled the shader replaces these fills, avoiding double light.
    result.lights[1].color = ambient * 0.65;
    result.lights[2].color = ambient * 0.45;
    result
}

/// The direction at `(azimuth offset, elevation)` degrees, the azimuth counted from `to_sun`'s.
fn around(to_sun: Vec3, (offset, elevation): (f32, f32)) -> Vec3 {
    let azimuth = to_sun.y.atan2(to_sun.x) + offset.to_radians();
    let elevation = elevation.to_radians();
    Vec3::new(elevation.cos() * azimuth.cos(), elevation.cos() * azimuth.sin(), elevation.sin())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_key_light_is_the_sun() {
        let sun = Vec3::new(0.35, 0.45, 1.0);
        assert!((rig(sun).lights[0].to_light - sun.normalize()).length() < 1e-6);
    }

    #[test]
    fn fill_and_back_sit_around_the_sun_at_their_elevations() {
        let rig = rig(Vec3::new(1.0, 0.0, 1.0));
        for (light, (offset, elevation)) in [(rig.lights[1], FILL), (rig.lights[2], BACK)] {
            let d = light.to_light;
            assert!((d.length() - 1.0).abs() < 1e-5);
            assert!((d.z.asin().to_degrees() - elevation).abs() < 1e-3);
            assert!((d.y.atan2(d.x).to_degrees() - offset).abs() < 1e-3);
        }
    }

    #[test]
    fn a_vertical_sun_still_gives_a_rig() {
        for light in rig(Vec3::Z).lights {
            assert!(light.to_light.is_finite());
        }
    }
}

#[cfg(test)]
mod weather_tests {
    use super::*;

    #[test]
    fn missing_world_inputs_keep_the_viewer_rig() {
        assert_eq!(world_rig(Vec3::Z, None), rig(Vec3::Z));
    }

    #[test]
    fn cloud_and_sun_inputs_control_key_and_fills_separately() {
        let mut light = blackbox_gfx::SurfaceLight {
            shadow_strength: 1.0,
            ambient: [0.0; 3],
            ambient_alpha: 1.0,
            diffuse: [0.0; 3],
            specular: [0.0; 3],
            specular_power: 16.0,
            reflection_scale: 0.0,
            reflective_specular_scale: 1.0,
        };
        light.diffuse = [0.8, 0.6, 0.4];
        light.ambient = [0.2, 0.3, 0.5];
        let clear = world_rig(Vec3::Z, Some(light));
        assert_eq!(clear.lights[0].color, Vec3::new(0.8, 0.6, 0.4));
        light.diffuse = [0.1; 3];
        let cloud = world_rig(Vec3::Z, Some(light));
        assert!(cloud.lights[0].color.length() < clear.lights[0].color.length());
        assert_eq!(cloud.lights[1], clear.lights[1]);
        light.ambient_alpha = 0.0;
        let sheltered = world_rig(Vec3::Z, Some(light));
        assert_eq!(sheltered.lights[0], cloud.lights[0]);
        assert_eq!(sheltered.lights[1].color, Vec3::ZERO);
        assert_eq!(sheltered.lights[2].color, Vec3::ZERO);
        light.diffuse = [0.0; 3];
        light.ambient = [0.0; 3];
        assert!(world_rig(Vec3::Z, Some(light)).lights.iter().all(|l| l.color == Vec3::ZERO));
    }
}
