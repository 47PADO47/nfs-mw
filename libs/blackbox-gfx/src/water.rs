//! Shared surface-water inputs and world-anchored coverage for shading and contact effects.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct SurfaceWater {
    pub dampness: f32,
    pub pooling: f32,
    /// Simulation seconds; independent of camera movement and time-of-day scrubbing.
    pub time: f32,
    pub rain: f32,
}

impl SurfaceWater {
    pub fn sanitized(self) -> Self {
        let unit = |v: f32| match v.is_finite() {
            true => v.clamp(0.0, 1.0),
            false => 0.0,
        };
        Self {
            dampness: unit(self.dampness),
            pooling: unit(self.pooling),
            rain: unit(self.rain),
            time: match self.time.is_finite() {
                true => self.time.max(0.0),
                false => 0.0,
            },
        }
    }

    /// Approximate coverage, not terrain drainage. Coordinates are metres in the surface's horizontal plane.
    /// The caller must restrict this to suitable materials and apply slope/exposure separately.
    pub fn puddle(self, xy: [f32; 2], coverage: f32, scale: f32) -> f32 {
        if self.pooling <= 0.0 || coverage <= 0.0 || !xy.into_iter().all(f32::is_finite) {
            return 0.0;
        }
        let p = xy.map(|v| v / scale.max(0.25));
        let field = noise(p) * 0.7 + noise(p.map(|v| v * 3.7 + 19.0)) * 0.3;
        let threshold = 0.86 - 0.48 * self.pooling * coverage.clamp(0.0, 1.0);
        smooth(threshold - 0.025, threshold + 0.025, field) * self.pooling
    }
}

fn smooth(a: f32, b: f32, x: f32) -> f32 {
    let t = ((x - a) / (b - a)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}
fn hash(x: i32, y: i32) -> f32 {
    let mut h = (x as u32).wrapping_mul(1597334677) ^ (y as u32).wrapping_mul(3812015801);
    h = (h ^ (h >> 16)).wrapping_mul(2246822519);
    ((h ^ (h >> 13)) & 65535) as f32 / 65535.0
}
fn noise(p: [f32; 2]) -> f32 {
    let [x, y] = p.map(|v| v.floor() as i32);
    let [u, v] = p.map(|v| {
        let f = v - v.floor();
        f * f * (3.0 - 2.0 * f)
    });
    let mix = |a, b, t| a + (b - a) * t;
    mix(mix(hash(x, y), hash(x + 1, y), u), mix(hash(x, y + 1), hash(x + 1, y + 1), u), v)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn road_reflection_controls_reject_nonfinite_and_bound_trace_cost() {
        let options = crate::RenderExperiments {
            road_reflection_range: f32::INFINITY,
            road_reflection_fade: f32::NAN,
            ray_road_blend: f32::INFINITY,
            ray_car_range: f32::NAN,
            ray_car_blend: f32::NEG_INFINITY,
            ray_road_screen_reuse: f32::NAN,
            ..Default::default()
        }
        .sanitized();
        assert_eq!(options.road_reflection_range, 320.0);
        assert_eq!(options.road_reflection_fade, 0.65);
        assert_eq!(options.ray_road_blend, 1.0);
        assert_eq!(options.ray_car_range, 320.0);
        assert_eq!(options.ray_car_blend, 1.0);
        assert_eq!(options.ray_road_screen_reuse, 1.0);
        let options = crate::RenderExperiments { road_reflection_range: 10000.0, road_reflection_fade: 0.0, ..options }
            .sanitized();
        assert_eq!(options.road_reflection_range, 800.0);
        assert_eq!(options.road_reflection_fade, 0.1);
    }

    #[test]
    fn puddles_are_stable_continuous_and_grow_with_accumulation() {
        let wet = SurfaceWater { dampness: 1.0, pooling: 1.0, ..Default::default() };
        let damp = SurfaceWater { pooling: 0.3, ..wet };
        let mut occupied = 0;
        for x in -100..100 {
            let p = [x as f32 * 0.7, 17.3];
            let full = wet.puddle(p, 0.7, 7.0);
            assert!((0.0..=1.0).contains(&full));
            assert!(full >= damp.puddle(p, 0.7, 7.0));
            assert!((full - wet.puddle([p[0] + 0.001, p[1]], 0.7, 7.0)).abs() < 0.01);
            occupied += usize::from(full > 0.1);
            assert_eq!(full, SurfaceWater { time: 1000.0, ..wet }.puddle(p, 0.7, 7.0));
        }
        assert!(occupied > 10 && occupied < 180, "patches must have both wet and exposed regions");
        assert_eq!(SurfaceWater::default().puddle([0.0; 2], 1.0, 7.0), 0.0);
    }
}
