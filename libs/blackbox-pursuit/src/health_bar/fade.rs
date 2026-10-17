//! The distance fade: a bar fades in between 2 and 5 m from the camera and out between 70 and 110 m. Spec section 9.

use super::params::BarParams;

/// The distance alpha of a bar `distance` metres from the camera, 0..=1. The curve is linear on each side.
pub fn distance_alpha(distance: f32, params: &BarParams) -> f32 {
    let near = (distance - params.near_fade_min) / (params.near_fade_max - params.near_fade_min);
    let far = (params.far_fade_max - distance) / (params.far_fade_max - params.far_fade_min);
    near.clamp(0.0, 1.0) * far.clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::health_bar::approx;

    #[test]
    fn near_and_far_ranges_fade_as_the_spec_gives_them() {
        let params = BarParams::default();
        assert!(approx(distance_alpha(2.0, &params), 0.0));
        assert!(approx(distance_alpha(3.5, &params), 0.5));
        assert!(approx(distance_alpha(5.0, &params), 1.0));
        assert!(approx(distance_alpha(70.0, &params), 1.0));
        assert!(approx(distance_alpha(90.0, &params), 0.5));
        assert!(approx(distance_alpha(110.0, &params), 0.0));
        assert!(approx(distance_alpha(150.0, &params), 0.0));
    }
}
