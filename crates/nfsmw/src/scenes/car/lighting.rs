//! The light rig cars are drawn with (docs/specs/car-assembly.md §8).
//!
//! The game's rig is four lights given as angles and colours, but no rig is stored in the
//! install files read so far, so this builds three directional lights around the sun: the
//! sun itself as the key, a cool fill on the far side and a dim back light. Angles and colours
//! are choices, not game values.

use blackbox_render::{DirectionalLight, LightingRig};
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
    }
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
