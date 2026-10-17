//! Authored atmosphere inputs. Spec: `world-atmosphere.md`.

use super::{Field, Lighting, Preset, Presets, blend, dusk_factor, lighting};

const SKY_SCALE: u32 = 0x9095_E8B8;
const HAZE_SCALE: u32 = 0x02C1_F6CA;
const SUN_FALLOFF: u32 = 0x2EF8_A8BF;
const DISTANCE_SCALE: u32 = 0x7260_6030;
const IN_SCATTER: u32 = 0x0075_6239;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Atmosphere {
    pub lighting: Lighting,
    pub ambient_alpha: f32,
    pub specular_power: f32,
    pub sky_scale: f32,
    pub haze_scale: f32,
    pub g: f32,
    pub distance_scale: f32,
    pub in_scatter_scale: f32,
}

/// The same two-stage blend as the original lighting, including scalar fields.
/// Invalid/missing required attributes are unavailable, not location-specific constants.
pub fn atmosphere(presets: &Presets, cloud: f32, sun_z: f32) -> Option<Atmosphere> {
    if presets.presets.len() < 4 || !sun_z.is_finite() {
        return None;
    }
    let high = blend(&presets.presets[0], &presets.presets[2], cloud);
    let low = blend(&presets.presets[1], &presets.presets[3], cloud);
    let dusk = dusk_factor(sun_z);
    let scalar = |key| mix_scalar(&high, &low, key, dusk);
    let out = Atmosphere {
        lighting: lighting(presets, cloud, sun_z)?,
        // The PC update overrides the blended ambient alpha after its final mix.
        ambient_alpha: 4.0,
        specular_power: 11.0,
        sky_scale: scalar(SKY_SCALE)?,
        haze_scale: scalar(HAZE_SCALE)?,
        g: scalar(SUN_FALLOFF)?,
        distance_scale: scalar(DISTANCE_SCALE)?,
        in_scatter_scale: scalar(IN_SCATTER)?,
    };
    let values = [out.ambient_alpha, out.sky_scale, out.haze_scale, out.g, out.distance_scale, out.in_scatter_scale];
    if values.iter().any(|value| !value.is_finite()) || out.g.abs() >= 1.0 {
        return None;
    }
    if [out.lighting.diffuse, out.lighting.ambient, out.lighting.specular, out.lighting.fog_haze, out.lighting.fog_sky]
        .iter()
        .flatten()
        .any(|value| !value.is_finite())
    {
        return None;
    }
    if out.ambient_alpha < 0.0 || out.sky_scale < 0.0 || out.distance_scale < 0.0 || out.in_scatter_scale < 0.0 {
        return None;
    }
    Some(out)
}

fn mix_scalar(high: &Preset, low: &Preset, key: u32, dusk: f32) -> Option<f32> {
    let scalar = |preset: &Preset| match preset.get(key)? {
        Field::Scalar(value) => Some(value),
        Field::Vector4(_) => None,
    };
    let hi = scalar(high)?;
    Some(hi + (scalar(low)? - hi) * dusk)
}

#[cfg(test)]
mod tests {
    use super::super::AMBIENT_COLOUR;
    use super::*;

    fn presets() -> Presets {
        let presets = (0..4)
            .map(|index| {
                let value = index as f32;
                let mut fields = [
                    super::super::DIFFUSE_COLOUR,
                    AMBIENT_COLOUR,
                    super::super::SPECULAR_COLOUR,
                    super::super::FOG_HAZE_COLOUR,
                    super::super::FOG_SKY_COLOUR,
                ]
                .map(|key| (key, Field::Vector4([value, value, value, 1.0 + value])))
                .to_vec();
                fields
                    .extend([SKY_SCALE, HAZE_SCALE, DISTANCE_SCALE, IN_SCATTER].map(|key| (key, Field::Scalar(value))));
                fields.push((SUN_FALLOFF, Field::Scalar(-0.47)));
                Preset { fields }
            })
            .collect();
        Presets { presets }
    }

    #[test]
    fn scalar_inputs_share_cloud_and_dusk_blending_and_runtime_forces_ambient_alpha() {
        let data = presets();
        let clear_high = atmosphere(&data, 0.0, 0.5).unwrap();
        assert_eq!(clear_high.sky_scale, 0.0);
        assert_eq!(clear_high.ambient_alpha, 4.0);
        let cloudy_low = atmosphere(&data, 1.0, 0.25).unwrap();
        assert_eq!(cloudy_low.sky_scale, 3.0);
        assert_eq!(cloudy_low.ambient_alpha, 4.0);
        let mixed = atmosphere(&data, 0.5, 0.375).unwrap();
        assert_eq!(mixed.lighting.ambient, [1.5; 3]);
        assert_eq!(mixed.lighting.specular, [1.5; 3]);
        assert_eq!(mixed.specular_power, 11.0);
        assert_eq!(mixed.ambient_alpha, 4.0);
        assert_eq!([mixed.sky_scale, mixed.haze_scale, mixed.distance_scale, mixed.in_scatter_scale], [1.5; 4]);
        assert_eq!(mixed.g, -0.47);
    }

    #[test]
    fn malformed_required_inputs_are_unavailable() {
        assert!(atmosphere(&Presets { presets: vec![] }, 0.0, 0.5).is_none());
        let mut data = presets();
        assert!(atmosphere(&data, 0.0, f32::NAN).is_none());
        for preset in &mut data.presets {
            preset.fields.retain(|(key, _)| *key != DISTANCE_SCALE);
        }
        assert!(atmosphere(&data, 0.0, 0.5).is_none());
        let mut data = presets();
        for preset in &mut data.presets {
            preset.fields.push((u32::MAX, Field::Scalar(f32::NAN)));
        }
        assert!(atmosphere(&data, 0.0, 0.5).is_some());
    }

    #[test]
    #[ignore = "needs the base-game install selected by NFSMW_GAME_DIR"]
    fn installed_atmosphere_inputs_are_authored_and_finite_through_both_blends() {
        let dir = game_install::GameDir::open(std::env::var("NFSMW_GAME_DIR").unwrap()).unwrap();
        let bytes = dir.read("GLOBAL/ATTRIBUTES.BIN").unwrap();
        let db = blackbox_attrib::Database::open(&bytes).unwrap();
        let data = super::super::load(&db).unwrap();
        for cloud in [0.0, 0.25, 0.5, 1.0] {
            for z in [0.0, 0.25, 0.375, 0.5, 0.9] {
                let value = atmosphere(&data, cloud, z).expect("complete authored atmosphere fields");
                assert!(value.distance_scale > 0.0);
                assert!(value.ambient_alpha > 0.0);
                assert!((-1.0..1.0).contains(&value.g));
            }
        }
    }
}
