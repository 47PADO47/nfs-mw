//! PC gameplay/frontend shaper rigs. Spec: `car-lighting.md`.

use blackbox_gfx::{DirectionalLight, LightingRig};
use glam::Vec3;

const GAMEPLAY: [(f32, f32, [f32; 3], f32); 3] = [
    (0.0, 350.0, [0.85, 0.9, 1.0], 0.83),
    (50.0, 220.0, [0.55, 0.6, 1.0], 0.4),
    (310.0, 220.0, [0.65, 0.7, 1.0], 0.45),
];
const FRONTEND: [(f32, f32, [f32; 3], f32); 3] = [
    (90.0, 48.0, [1.0, 1.0, 0.95], 0.98),
    (134.0, 185.0, [0.83, 0.83, 1.0], 0.5),
    (27.0, 175.0, [0.83, 0.83, 1.0], 0.35),
];

/// Current sun, not a sun-relative custom fill/back approximation.
pub fn gameplay(to_sun: Vec3, material_specular_scale: f32) -> LightingRig {
    let sun = to_sun.try_normalize().unwrap_or(Vec3::Z);
    let azimuth = angle(sun.y.atan2(sun.x).to_degrees());
    let polar = angle(sun.z.clamp(-1.0, 1.0).acos().to_degrees());
    let lights = GAMEPLAY.map(|(theta, phi, rgb, scale)| {
        DirectionalLight::new(spherical(azimuth + theta + 180.0, -polar - phi), Vec3::from_array(rgb) * scale)
    });
    LightingRig { lights, ambient: Vec3::ZERO, material_specular_scale }
}

/// Default quick-race frontend profile, also used by several stock garage defaults.
pub fn frontend(material_specular_scale: f32) -> LightingRig {
    frontend_theta(material_specular_scale, 1.0)
}

/// The original backroom uses the same entries with theta negated before angle quantization.
pub fn backroom(material_specular_scale: f32) -> LightingRig {
    frontend_theta(material_specular_scale, -1.0)
}

fn frontend_theta(material_specular_scale: f32, theta_sign: f32) -> LightingRig {
    let lights = FRONTEND.map(|(theta, phi, rgb, scale)| {
        DirectionalLight::new(spherical(theta * theta_sign + 180.0, 90.0 - phi), Vec3::from_array(rgb) * scale)
    });
    LightingRig { lights, ambient: Vec3::ZERO, material_specular_scale }
}

fn spherical(azimuth: f32, polar: f32) -> Vec3 {
    let (sa, ca) = angle(azimuth).to_radians().sin_cos();
    let (sp, cp) = angle(polar).to_radians().sin_cos();
    Vec3::new(sp * ca, sp * sa, cp)
}

/// Engine angle precision; original lookup-table interpolation remains separately unverified.
fn angle(degrees: f32) -> f32 {
    let units = ((degrees * 65536.0).trunc() as i64 / 360) as u16;
    f32::from(units) * (360.0 / 65536.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn original_gameplay_colors_and_sun_relative_orientation_are_authored() {
        let first = gameplay(Vec3::X, 0.7);
        assert_eq!(first.lights[0].color, Vec3::new(0.85, 0.9, 1.0) * 0.83);
        assert_eq!(first.material_specular_scale, 0.7);
        assert_eq!(first.ambient, Vec3::ZERO);
        assert!(first.lights[0].to_light.x > 0.98 && first.lights[0].to_light.z > 0.17);
        let second = gameplay(Vec3::Y, 0.7);
        assert!(second.lights[0].to_light.y > 0.98);
        assert!(first.lights.iter().all(|light| (light.to_light.length() - 1.0).abs() < 0.00001));
    }

    #[test]
    fn frontend_profile_is_world_fixed_and_not_the_gameplay_rig() {
        let rig = frontend(1.0);
        assert_eq!(rig.lights[0].color, Vec3::new(1.0, 1.0, 0.95) * 0.98);
        assert!(rig.lights[0].to_light.y < -0.66 && rig.lights[0].to_light.z > 0.74);
        assert_ne!(rig.lights, gameplay(Vec3::Z, 1.0).lights);
    }

    #[test]
    fn backroom_negates_theta_and_keeps_the_authored_colors_and_material_scale() {
        let normal = frontend(0.7);
        let mirrored = backroom(0.7);
        assert_eq!(mirrored.material_specular_scale, normal.material_specular_scale);
        assert_eq!(mirrored.ambient, normal.ambient);
        for (actual, source) in mirrored.lights.iter().zip(&normal.lights) {
            assert_eq!(actual.color, source.color);
            assert!((actual.to_light.x - source.to_light.x).abs() < 0.0002);
            assert!((actual.to_light.y + source.to_light.y).abs() < 0.0002);
            assert!((actual.to_light.z - source.to_light.z).abs() < 0.0002);
        }
    }
}
