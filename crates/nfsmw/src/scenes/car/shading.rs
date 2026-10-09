//! From the game's light materials to the renderer's glossy materials
//! (docs/specs/car-assembly.md §8).

use blackbox_render::GlossyMaterial;
use nfsmw_data::car::LightMaterial;

/// A material's `min` (edge-on) value and `range` up to its `max` (facing), per channel:
/// `min = min_scale × min colour`, `range = max_scale × max colour − min`.
fn ramp<const N: usize>(min_scale: f32, min: [f32; N], max_scale: f32, max: [f32; N]) -> ([f32; N], [f32; N]) {
    let min = min.map(|c| c * min_scale);
    let range = std::array::from_fn(|i| max[i] * max_scale - min[i]);
    (min, range)
}

/// The renderer material for a light material.
pub fn glossy_material(m: &LightMaterial) -> GlossyMaterial {
    let [dr, dg, db] = m.diffuse_min;
    let [dmr, dmg, dmb] = m.diffuse_max;
    let (diffuse_min, diffuse_range) = ramp(
        1.0,
        [dr * m.diffuse_min_scale, dg * m.diffuse_min_scale, db * m.diffuse_min_scale, m.diffuse_min_alpha],
        1.0,
        [dmr * m.diffuse_max_scale, dmg * m.diffuse_max_scale, dmb * m.diffuse_max_scale, m.diffuse_max_alpha],
    );
    let (specular_min, specular_range) =
        ramp(m.specular_min_scale, m.specular_min, m.specular_max_scale, m.specular_max);
    // The reflection strength is one number: the first channel of the colour.
    let (env_min, env_range) = ramp(m.envmap_min_scale, [m.envmap_min[0]], m.envmap_max_scale, [m.envmap_max[0]]);
    GlossyMaterial {
        diffuse_min,
        diffuse_range,
        specular_min,
        specular_range,
        specular_power: m.specular_power,
        envmap_min: env_min[0],
        envmap_range: env_range[0],
        envmap_power: m.envmap_power,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn silver() -> LightMaterial {
        // METPAINTSILVER (docs/formats/cardata.md): diffuse 0.75 -> 1.0, specular power 3
        // (0.7 -> 0.3), envmap power 0.15 (3.5 -> 0.2), all colours white.
        LightMaterial {
            name: "METPAINTSILVER".into(),
            name_hash: 0,
            version: 3,
            diffuse_min_scale: 0.75,
            diffuse_min: [1.0; 3],
            diffuse_max_scale: 1.0,
            diffuse_max: [1.0; 3],
            diffuse_min_alpha: 1.0,
            diffuse_max_alpha: 1.0,
            specular_power: 3.0,
            specular_min_scale: 0.7,
            specular_min: [1.0; 3],
            specular_max_scale: 0.3,
            specular_max: [1.0; 3],
            envmap_power: 0.15,
            envmap_min_scale: 3.5,
            envmap_min: [1.0; 3],
            envmap_max_scale: 0.2,
            envmap_max: [1.0; 3],
            metallic_scale: 1.0,
            specular_hot_spot: 1.0,
        }
    }

    fn close(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-6
    }

    #[test]
    fn ramps_run_from_the_edge_on_value_to_the_facing_value() {
        let m = glossy_material(&silver());
        assert!(close(m.diffuse_min[0], 0.75) && close(m.diffuse_range[0], 0.25));
        assert!(close(m.specular_min[1], 0.7) && close(m.specular_range[1], -0.4));
        assert!(close(m.envmap_min, 3.5) && close(m.envmap_range, -3.3));
        assert!(close(m.specular_power, 3.0) && close(m.envmap_power, 0.15));
        // Facing the viewer (f = 1) gives back the max values.
        assert!(close(m.diffuse_min[0] + m.diffuse_range[0], 1.0));
        assert!(close(m.envmap_min + m.envmap_range, 0.2));
    }

    #[test]
    fn colours_scale_their_channels_and_alpha_is_not_scaled() {
        let mut light = silver();
        light.diffuse_min = [1.0, 0.5, 0.0];
        light.diffuse_min_scale = 0.5;
        light.diffuse_min_alpha = 0.4;
        light.diffuse_max_alpha = 0.4;
        let m = glossy_material(&light);
        assert_eq!(&m.diffuse_min[..3], &[0.5, 0.25, 0.0]);
        assert!(close(m.diffuse_min[3], 0.4) && close(m.diffuse_range[3], 0.0));
    }
}
