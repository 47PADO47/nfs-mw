//! Authored car specular modulation. Spec: `car-lighting.md`.

use super::{Field, Preset, Presets, dusk_factor};

pub const CAR_SPECULAR_SCALE: u32 = 0xB656_E7C6;

/// The original cloud and high/low-sun blends, without requiring fog fields.
pub fn car_specular_scale(presets: &Presets, cloud: f32, sun_z: f32) -> Option<f32> {
    if presets.presets.len() < 4 || !sun_z.is_finite() {
        return None;
    }
    let scalar = |preset: &Preset| match preset.get(CAR_SPECULAR_SCALE)? {
        Field::Scalar(value) if value.is_finite() && value >= 0.0 => Some(value),
        _ => None,
    };
    let cloud = if cloud.is_finite() { cloud.clamp(0.0, 1.0) } else { 0.0 };
    let clear_high = scalar(&presets.presets[0])?;
    let clear_low = scalar(&presets.presets[1])?;
    let overcast_high = scalar(&presets.presets[2])?;
    let overcast_low = scalar(&presets.presets[3])?;
    let high = clear_high + (overcast_high - clear_high) * cloud;
    let low = clear_low + (overcast_low - clear_low) * cloud;
    Some(high + (low - high) * dusk_factor(sun_z))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn car_specular_scale_uses_cloud_and_dusk_without_fog_dependencies() {
        let data = Presets {
            presets: [0.5, 1.0, 2.0, 4.0]
                .map(|value| Preset { fields: vec![(CAR_SPECULAR_SCALE, Field::Scalar(value))] })
                .to_vec(),
        };
        assert_eq!(car_specular_scale(&data, 0.0, 0.5), Some(0.5));
        assert_eq!(car_specular_scale(&data, 1.0, 0.25), Some(4.0));
        assert_eq!(car_specular_scale(&data, 0.5, 0.375), Some(1.875));
        assert_eq!(car_specular_scale(&data, f32::NAN, 0.5), Some(0.5));
        assert_eq!(car_specular_scale(&data, 0.0, f32::NAN), None);
        let invalid = Presets { presets: vec![Preset::default(); 4] };
        assert_eq!(car_specular_scale(&invalid, 0.0, 0.5), None);
    }
}
