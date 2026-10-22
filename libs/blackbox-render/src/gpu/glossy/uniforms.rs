//! The uniform blocks of `glossy.wgsl`, packed from the public types.

use crate::{GlossyMaterial, LightingRig};

/// The `Rig` block: per light a direction and a colour, then the ambient colour.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub(super) struct RigUniform {
    pub to_light: [[f32; 4]; 3],
    pub color: [[f32; 4]; 3],
    pub ambient: [f32; 4],
}

impl RigUniform {
    pub(super) fn new(rig: &LightingRig) -> Self {
        Self {
            to_light: rig.lights.map(|l| l.to_light.extend(0.0).to_array()),
            color: rig.lights.map(|l| l.color.extend(0.0).to_array()),
            ambient: rig.ambient.extend(rig.material_specular_scale).to_array(),
        }
    }
}

/// The `Material` block.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub(super) struct MaterialUniform {
    pub diffuse_min: [f32; 4],
    pub diffuse_range: [f32; 4],
    pub specular_min: [f32; 4],
    pub specular_range: [f32; 4],
    /// x = specular power, y = envmap min, z = envmap range, w = envmap power.
    pub params: [f32; 4],
    /// roughness, coat, reflection intensity, SurfaceKind tag (zero disables).
    pub surface: [f32; 4],
}

impl MaterialUniform {
    /// Legacy materials retain their exponent control; opted-in surfaces have independent intensity.
    pub(super) fn new(m: &GlossyMaterial, brightness: f32) -> Self {
        let [r, g, b] = m.specular_min;
        let [dr, dg, db] = m.specular_range;
        let environment_power = match m.model == blackbox_gfx::GlossyModel::Custom && m.surface.is_some() {
            true => m.envmap_power,
            false => m.envmap_power * brightness,
        };
        Self {
            diffuse_min: m.diffuse_min,
            diffuse_range: m.diffuse_range,
            specular_min: [
                r,
                g,
                b,
                match m.model {
                    blackbox_gfx::GlossyModel::Custom => 0.0,
                    blackbox_gfx::GlossyModel::VertexLit => 1.0,
                    blackbox_gfx::GlossyModel::Shadowed => 2.0,
                },
            ],
            specular_range: [dr, dg, db, 0.0],
            params: [m.specular_power, m.envmap_min, m.envmap_range, environment_power],
            surface: match m.surface.filter(|_| m.model == blackbox_gfx::GlossyModel::Custom) {
                Some(surface) => [
                    surface.roughness.clamp(0.08, 1.0),
                    surface.coat.clamp(0.0, 1.0),
                    brightness.max(0.0),
                    surface.kind as u8 as f32,
                ],
                None => [0.0; 4],
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use glam::Vec3;

    use super::*;
    use crate::{DirectionalLight, GlossyMaterial, LightingRig};

    #[test]
    fn blocks_match_the_wgsl_layout() {
        // Seven vec4 and six vec4: every member is 16-byte aligned, so no padding is needed.
        assert_eq!(std::mem::size_of::<RigUniform>(), 112);
        assert_eq!(std::mem::size_of::<MaterialUniform>(), 96);
    }

    #[test]
    fn rig_is_packed_in_light_order() {
        let rig = LightingRig {
            lights: [
                DirectionalLight::new(Vec3::X, Vec3::new(1.0, 0.5, 0.25)),
                DirectionalLight::new(Vec3::Y, Vec3::splat(0.5)),
                DirectionalLight::new(Vec3::Z, Vec3::splat(0.1)),
            ],
            ambient: Vec3::splat(0.2),
            material_specular_scale: 0.75,
        };
        let u = RigUniform::new(&rig);
        assert_eq!(u.to_light, [[1.0, 0.0, 0.0, 0.0], [0.0, 1.0, 0.0, 0.0], [0.0, 0.0, 1.0, 0.0]]);
        assert_eq!(u.color[0], [1.0, 0.5, 0.25, 0.0]);
        assert_eq!(u.ambient, [0.2, 0.2, 0.2, 0.75]);
    }

    #[test]
    fn material_scalars_go_to_params() {
        let m = GlossyMaterial {
            specular_power: 3.0,
            envmap_min: 3.5,
            envmap_range: -3.3,
            envmap_power: 0.15,
            specular_min: [0.7; 3],
            ..Default::default()
        };
        let u = MaterialUniform::new(&m, 1.0);
        assert_eq!(u.params, [3.0, 3.5, -3.3, 0.15]);
        assert_eq!(u.specular_min, [0.7, 0.7, 0.7, 0.0]);
    }

    #[test]
    fn original_model_tags_share_material_layout_and_keep_specular_channels() {
        for (model, tag) in [
            (blackbox_gfx::GlossyModel::Custom, 0.0),
            (blackbox_gfx::GlossyModel::VertexLit, 1.0),
            (blackbox_gfx::GlossyModel::Shadowed, 2.0),
        ] {
            let u = MaterialUniform::new(
                &GlossyMaterial { model, specular_min: [0.2, 0.3, 0.4], ..Default::default() },
                1.0,
            );
            assert_eq!(u.specular_min, [0.2, 0.3, 0.4, tag]);
        }
    }

    #[test]
    fn brightness_scales_only_the_environment_term() {
        let m = GlossyMaterial { envmap_power: 0.2, specular_power: 3.0, ..Default::default() };
        let dim = MaterialUniform::new(&m, 0.5);
        let bright = MaterialUniform::new(&m, 2.0);
        assert!((dim.params[3] - 0.1).abs() < 1e-6);
        assert!((bright.params[3] - 0.4).abs() < 1e-6);
        assert_eq!((dim.params[0], dim.specular_min), (bright.params[0], bright.specular_min));
        assert_eq!(MaterialUniform::new(&m, 0.0).params[3], 0.0, "zero turns the environment off");
    }
}

#[cfg(test)]
mod surface_tests {
    use super::*;
    use blackbox_gfx::{GlossyModel, GlossySurface};
    #[test]
    fn surface_intensity_is_separate_and_original_families_ignore_the_opt_in() {
        let mut m = GlossyMaterial {
            surface: Some(GlossySurface { roughness: 0.78, coat: 0.03, ..Default::default() }),
            envmap_power: 0.7,
            ..Default::default()
        };
        let u = MaterialUniform::new(&m, 2.0);
        assert_eq!(u.params[3], 0.7);
        assert_eq!(u.surface, [0.78, 0.03, 2.0, 1.0]);
        for model in [GlossyModel::VertexLit, GlossyModel::Shadowed] {
            m.model = model;
            let u = MaterialUniform::new(&m, 2.0);
            assert_eq!(u.params[3], 1.4);
            assert_eq!(u.surface, [0.0; 4]);
        }
    }
}
