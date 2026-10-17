//! The game's distance fog: atmospheric scattering with a Rayleigh term (the sky colour) and a Mie term (the fog
//! haze colour), as the fog effect's constants name them (`Fog_Br_Plus_Bm`, `Fog_One_Over_BrBm`, `Fog_Const_1..3`,
//! `Fog_DistanceScale`). Inputs come from the lighting object (`docs/specs/time-of-day.md`).
//!
//! For a view distance `d` and a cosine `c` between the view and the sun:
//! `E = exp(−(Br + Bm)·d)`, and the in-scattered light is `(Br·3/(16π)·(1 + c²) + Bm·HG(c)) · (1 − E) / (Br + Bm)`,
//! where `HG(c) = (1 − g)² / (4π · (1 + g² − 2g·c)^{3/2})`. The pixel colour is `scene·E + in-scattered`.

use std::f32::consts::PI;

/// The game's depth scale constants (the range and the base of the depth term; see the spec).
pub const DEPTH_RANGE: f32 = 0.0099;
pub const DEPTH_BASE: f32 = 0.0001;

/// One frame's scattering coefficients.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Scatter {
    /// Rayleigh coefficients per channel (the sky colour, scaled by the sky distance).
    pub br: [f32; 3],
    /// Mie coefficients per channel (the haze colour, scaled by the fog distance).
    pub bm: [f32; 3],
    /// The Mie anisotropy `g`.
    pub g: f32,
}

/// The coefficients from the lighting object: the sky colour (`+0x70`), the haze colour (`+0x80`), the two
/// distance fields (`+0x90` for the sky and `+0x94` for the fog, each scaled by 0.01) and `g` (`+0x9C`).
pub fn scatter(sky: [f32; 3], haze: [f32; 3], sky_field: f32, fog_field: f32, g: f32) -> Scatter {
    let sky_dist = sky_field * 0.01 * DEPTH_RANGE + DEPTH_BASE;
    let fog_dist = match fog_field > 0.0 {
        true => fog_field * 0.01 * DEPTH_RANGE + DEPTH_BASE,
        false => 0.0,
    };
    Scatter { br: sky.map(|c| c * sky_dist), bm: haze.map(|c| c * fog_dist), g }
}

impl Scatter {
    /// The transmission `E` per channel at view distance `d`.
    pub fn extinction(&self, d: f32) -> [f32; 3] {
        std::array::from_fn(|i| (-(self.br[i] + self.bm[i]) * d).exp())
    }

    /// The in-scattered light per channel at distance `d`, for the cosine `c` between the view and the sun.
    pub fn inscatter(&self, d: f32, c: f32) -> [f32; 3] {
        let g = self.g;
        let hg = (1.0 - g) * (1.0 - g) / (4.0 * PI * (1.0 + g * g - 2.0 * g * c).max(1e-6).powf(1.5));
        let rayleigh = 3.0 / (16.0 * PI) * (1.0 + c * c);
        let mie = hg;
        let e = self.extinction(d);
        std::array::from_fn(|i| {
            let sum = self.br[i] + self.bm[i];
            if sum <= 0.0 {
                return 0.0;
            }
            (self.br[i] * rayleigh + self.bm[i] * mie) * (1.0 - e[i]) / sum
        })
    }

    /// The pixel colour for a scene colour at distance `d`.
    pub fn apply(&self, scene: [f32; 3], d: f32, c: f32) -> [f32; 3] {
        let e = self.extinction(d);
        let s = self.inscatter(d, c);
        std::array::from_fn(|i| scene[i] * e[i] + s[i])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Scatter {
        scatter([0.19, 0.3, 0.6], [0.5, 0.5, 0.5], 3.0, 2.27, -0.47)
    }

    #[test]
    fn at_zero_distance_the_scene_is_unchanged() {
        let s = sample();
        let scene = [0.2, 0.4, 0.6];
        let out = s.apply(scene, 0.0, 0.3);
        for i in 0..3 {
            assert!((out[i] - scene[i]).abs() < 1e-6);
        }
    }

    #[test]
    fn far_away_the_scene_fades_to_the_scattered_light() {
        let s = sample();
        let scene = [1.0, 1.0, 1.0];
        let far = s.apply(scene, 1.0e6, 0.3);
        let limit = s.inscatter(1.0e6, 0.3);
        for i in 0..3 {
            assert!((far[i] - limit[i]).abs() < 1e-5);
        }
    }

    #[test]
    fn transmission_falls_with_distance() {
        let s = sample();
        let near = s.extinction(100.0);
        let far = s.extinction(1000.0);
        for i in 0..3 {
            assert!(far[i] < near[i]);
            assert!(near[i] < 1.0);
        }
    }

    #[test]
    fn the_depth_constants_match_the_game_scale() {
        // The sky term at field 3.0: 0.03 · 0.0099 + 0.0001 = 0.000397.
        let s = scatter([1.0, 1.0, 1.0], [0.0; 3], 3.0, 0.0, 0.0);
        assert!((s.br[0] - 0.000397).abs() < 1e-7, "{}", s.br[0]);
    }

    #[test]
    fn nonpositive_haze_scale_disables_haze_instead_of_adding_the_depth_base() {
        for field in [-3.0, 0.0] {
            let s = scatter([1.0; 3], [1.0; 3], 3.0, field, 0.0);
            assert_eq!(s.bm, [0.0; 3]);
            assert!(s.br.iter().all(|value| *value > 0.0));
        }
    }
}
