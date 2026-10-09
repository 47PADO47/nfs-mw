//! Bounded spark queries through the existing nearest-hit API.
use blackbox_collision::{BARRIER_TWO_SIDED, CollisionWorld, GROUP_EXCLUSION, Hit, HitKind, RayOptions};
use glam::Vec3;

pub(super) fn world_hit(world: &CollisionWorld, from: Vec3, to: Vec3) -> Option<Hit> {
    let options = RayOptions { exclude: u32::from(GROUP_EXCLUSION), ..RayOptions::default() };
    nearest(from, to, |start, end| world.ray_cast(start.to_array(), end.to_array(), &options))
}

fn nearest(from: Vec3, to: Vec3, mut cast: impl FnMut(Vec3, Vec3) -> Option<Hit>) -> Option<Hit> {
    let delta = to - from;
    let length = delta.length();
    if !length.is_finite() || length < 1e-7 {
        return None;
    }
    let mut progress = 0.0;
    for _ in 0..4 {
        let mut hit = cast(from + delta * progress, to)?;
        let fraction = (Vec3::from(hit.point) - from).dot(delta) / delta.length_squared();
        if !fraction.is_finite() || fraction < progress || fraction > 1.0 {
            return None;
        }
        if hit.kind != HitKind::Barrier || hit.front_facing || hit.surface_flags & BARRIER_TWO_SIDED != 0 {
            hit.t = fraction;
            return Some(hit);
        }
        progress = fraction + 0.001 / length;
        if progress >= 1.0 {
            return None;
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hit(x: f32) -> Hit {
        Hit {
            t: 0.5,
            point: [x, 0.0, 0.0],
            normal: [-1.0, 0.0, 0.0],
            kind: HitKind::Barrier,
            front_facing: false,
            surface_hash: 7,
            surface_index: 0,
            surface_flags: 0,
            section: 1,
            instance: 0,
        }
    }

    #[test]
    fn skips_one_sided_backs_and_retains_the_original_step_fraction() {
        let mut calls = 0;
        let result = nearest(Vec3::ZERO, Vec3::X, |start, _| {
            calls += 1;
            if calls == 1 {
                return Some(hit(0.2));
            }
            assert!(start.x > 0.2);
            Some(Hit { surface_flags: BARRIER_TWO_SIDED, ..hit(0.7) })
        })
        .unwrap();
        assert_eq!(calls, 2);
        assert!((result.t - 0.7).abs() < 1e-6);
    }

    #[test]
    fn excludes_extended_short_ray_hits_and_bounds_rejected_queries() {
        assert!(nearest(Vec3::ZERO, Vec3::X * 0.002, |_, _| Some(hit(0.004))).is_none());
        assert!(nearest(Vec3::ZERO, Vec3::ZERO, |_, _| panic!("stationary particles need no query")).is_none());
        let mut calls = 0;
        assert!(
            nearest(Vec3::ZERO, Vec3::X, |start, _| {
                calls += 1;
                Some(hit(start.x + 0.1))
            })
            .is_none()
        );
        assert_eq!(calls, 4);
    }
}
