//! Original ordinary gameplay shadow strength, independent of map resolution.
pub fn strength(cloud: f32, sun_height: f32) -> f32 {
    if !cloud.is_finite() || !sun_height.is_finite() {
        return 0.1;
    }
    let altitude = ((sun_height - 0.14) / (0.677 - 0.14)).clamp(0.0, 1.0);
    0.1 + 0.8 * (1.0 - cloud.clamp(0.0, 1.0)) * altitude
}

#[cfg(test)]
mod tests {
    use super::strength;
    #[test]
    fn cloud_and_low_sun_reduce_shadow_contrast_without_changing_the_map() {
        for (cloud, height, expected) in
            [(0.0, 1.0, 0.9), (0.0, 0.14, 0.1), (1.0, 1.0, 0.1), (0.5, 0.677, 0.5), (0.0, 0.4085, 0.5)]
        {
            assert!((strength(cloud, height) - expected).abs() < 1e-6);
        }
        assert_eq!(strength(f32::NAN, 0.7), 0.1);
    }
}
