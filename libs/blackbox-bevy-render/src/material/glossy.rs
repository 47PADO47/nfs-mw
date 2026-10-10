//! The glossy material's extra uniform blocks: the lighting rig (shared by every glossy material)
//! and one material's shading constants (baked in at creation, like native's `MaterialUniform`;
//! see `libs/blackbox-render/src/gpu/glossy/uniforms.rs` and `docs/bevy-backend.md`).
//!
//! Three lights are flattened into separate fields rather than a WGSL array: the memory layout is
//! identical either way (three consecutive 16-byte vec4s), and it keeps both sides of the uniform
//! free of array-of-struct encoding questions.

use bevy_math::Vec4;
use bevy_render::render_resource::ShaderType;
use blackbox_gfx::{GlossyMaterial, LightingRig};

/// The lighting rig every glossy material reads: three lights and an ambient colour, in Bevy axes.
#[derive(Debug, Clone, Copy, PartialEq, ShaderType)]
pub struct RigUniform {
    pub light0_dir: Vec4,
    pub light1_dir: Vec4,
    pub light2_dir: Vec4,
    pub light0_color: Vec4,
    pub light1_color: Vec4,
    pub light2_color: Vec4,
    pub ambient: Vec4,
}

impl RigUniform {
    /// `rig`'s lights, converted to Bevy's axes.
    pub fn of(rig: &LightingRig) -> Self {
        let [a, b, c] = rig.lights;
        let dir = |l: blackbox_gfx::DirectionalLight| {
            let d = crate::axes::point(l.to_light);
            Vec4::new(d.x, d.y, d.z, 0.0)
        };
        let color = |l: blackbox_gfx::DirectionalLight| Vec4::new(l.color.x, l.color.y, l.color.z, 0.0);
        Self {
            light0_dir: dir(a),
            light1_dir: dir(b),
            light2_dir: dir(c),
            light0_color: color(a),
            light1_color: color(b),
            light2_color: color(c),
            ambient: Vec4::new(rig.ambient.x, rig.ambient.y, rig.ambient.z, 0.0),
        }
    }
}

impl Default for RigUniform {
    /// No lights and no ambient: a glossy draw with no rig set yet is black, not undefined.
    fn default() -> Self {
        Self {
            light0_dir: Vec4::ZERO,
            light1_dir: Vec4::ZERO,
            light2_dir: Vec4::ZERO,
            light0_color: Vec4::ZERO,
            light1_color: Vec4::ZERO,
            light2_color: Vec4::ZERO,
            ambient: Vec4::ZERO,
        }
    }
}

/// One glossy material's shading constants, mirroring native's `MaterialUniform` field for field.
#[derive(Debug, Clone, Copy, PartialEq, ShaderType)]
pub struct GlossyUniform {
    pub diffuse_min: Vec4,
    pub diffuse_range: Vec4,
    pub specular_min: Vec4,
    pub specular_range: Vec4,
    /// x = specular power, y = envmap min, z = envmap range, w = envmap power.
    pub params: Vec4,
}

impl GlossyUniform {
    pub fn of(m: &GlossyMaterial) -> Self {
        let [sr, sg, sb] = m.specular_min;
        let [dr, dg, db] = m.specular_range;
        Self {
            diffuse_min: Vec4::from(m.diffuse_min),
            diffuse_range: Vec4::from(m.diffuse_range),
            specular_min: Vec4::new(sr, sg, sb, 0.0),
            specular_range: Vec4::new(dr, dg, db, 0.0),
            params: Vec4::new(m.specular_power, m.envmap_min, m.envmap_range, m.envmap_power),
        }
    }
}

impl Default for GlossyUniform {
    fn default() -> Self {
        Self::of(&GlossyMaterial::default())
    }
}

#[cfg(test)]
mod tests {
    use blackbox_gfx::DirectionalLight;
    use glam::Vec3;

    use super::*;

    #[test]
    fn the_sun_s_direction_is_in_bevy_axes() {
        // game (0, 0, 1) is straight up; Bevy's up is +y.
        let rig = LightingRig { lights: [DirectionalLight::new(Vec3::Z, Vec3::ONE); 3], ambient: Vec3::ZERO };
        let u = RigUniform::of(&rig);
        assert!((u.light0_dir.y - 1.0).abs() < 1e-6);
        assert!(u.light0_dir.x.abs() < 1e-6 && u.light0_dir.z.abs() < 1e-6);
    }

    #[test]
    fn lights_are_packed_in_order_with_their_own_colour() {
        let rig = LightingRig {
            lights: [
                DirectionalLight::new(Vec3::X, Vec3::new(1.0, 0.5, 0.25)),
                DirectionalLight::new(Vec3::Y, Vec3::splat(0.5)),
                DirectionalLight::new(Vec3::Z, Vec3::splat(0.1)),
            ],
            ambient: Vec3::splat(0.2),
        };
        let u = RigUniform::of(&rig);
        assert_eq!(u.light0_color, Vec4::new(1.0, 0.5, 0.25, 0.0));
        assert_eq!(u.light1_color, Vec4::splat(0.5).with_w(0.0));
        assert_eq!(u.ambient, Vec4::new(0.2, 0.2, 0.2, 0.0));
    }

    #[test]
    fn material_scalars_go_to_params() {
        let m = GlossyMaterial {
            specular_power: 3.0,
            envmap_min: 3.5,
            envmap_range: -3.3,
            envmap_power: 0.15,
            specular_min: [0.7; 3],
            ..GlossyMaterial::default()
        };
        let u = GlossyUniform::of(&m);
        assert_eq!(u.params, Vec4::new(3.0, 3.5, -3.3, 0.15));
        assert_eq!(u.specular_min, Vec4::new(0.7, 0.7, 0.7, 0.0));
    }

    #[test]
    fn the_default_rig_has_no_light_or_ambient() {
        assert_eq!(RigUniform::default().ambient, Vec4::ZERO);
        assert_eq!(RigUniform::default().light0_color, Vec4::ZERO);
    }
}
