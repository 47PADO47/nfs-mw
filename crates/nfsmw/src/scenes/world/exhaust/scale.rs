//! How big the burning nitrous flame is drawn. The group's own emitters (`fxcar_nos`: a fire of 0.35 to 0.45 m and a glow that grows
//! to 1.5 m and is thrown out at 16 m/s) make a plume that fills the view behind the car. The nitrous of the 2012 game is a small
//! glow in each pipe, 0.3 m wide growing to 0.7 m at the peak of its alpha (`docs/formats/chameleon-nitrous.md`), so the host
//! draws the stock flame smaller. This is the rewrite's own choice, not a reading of the 2005 game; it changes the blue lanes
//! only, so the blow-off and the backfire keep the group's numbers.
//!
//! A bike's pop and shift flames are drawn smaller too ([`compact_bike`]): the lanes of a bike are the group's orange lanes at
//! [`BIKE_FLAME_SCALE`](super::trigger::BIKE_FLAME_SCALE) of the size, reach and pipe volume.

use blackbox_particles::EmitterSpec;

/// The factor on the width of the nitrous sprites.
pub(super) const NITROUS_SIZE: f32 = 0.3;
/// The factor on the speed the nitrous sprites leave the pipe at. The glow's drag (quadratic) keeps its reach under a metre.
pub(super) const NITROUS_REACH: f32 = 0.08;

/// `spec` for the burning nitrous: narrower sprites that stay nearer the pipe.
pub(super) fn compact_nitrous(spec: &EmitterSpec) -> EmitterSpec {
    let mut spec = spec.clone();
    spec.size = spec.size.map(|width| width * NITROUS_SIZE);
    spec.speed *= NITROUS_REACH;
    spec.velocity_start *= NITROUS_REACH;
    spec.velocity_delta *= NITROUS_REACH;
    spec
}

/// `spec` for the pop and shift flame of a bike: the whole flame at `factor` of its size, so the sprites, the speed they
/// leave the pipe at, the accelerations that bend their path and the volume they start from all shrink together and the flame
/// keeps its shape. The life, the rate and the colours are the group's.
pub(super) fn compact_bike(spec: &EmitterSpec, factor: f32) -> EmitterSpec {
    let mut spec = spec.clone();
    spec.size = spec.size.map(|width| width * factor);
    spec.speed *= factor;
    spec.speed_variance *= factor;
    spec.velocity_start *= factor;
    spec.velocity_delta *= factor;
    spec.accel_start *= factor;
    spec.accel_delta *= factor;
    spec.volume_center *= factor;
    spec.volume_extent *= factor;
    spec
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::Vec3;

    #[test]
    fn a_bike_flame_is_the_whole_emitter_at_a_fraction_of_its_size() {
        let spec = EmitterSpec {
            size: [0.1, 0.4, 1.0, 1.5],
            speed: 16.0,
            speed_variance: 2.0,
            velocity_start: Vec3::new(1.0, 2.0, 3.0),
            accel_delta: Vec3::splat(0.5),
            volume_extent: Vec3::new(0.0, 0.2, 0.0),
            rate: 50.0,
            life: 0.5,
            ..EmitterSpec::default()
        };
        let small = compact_bike(&spec, 0.4);
        assert!(small.size.iter().zip([0.04, 0.16, 0.4, 0.6]).all(|(a, b)| (a - b).abs() < 1e-6));
        assert!((small.speed - 6.4).abs() < 1e-5 && (small.speed_variance - 0.8).abs() < 1e-5);
        assert!((small.velocity_start - Vec3::new(0.4, 0.8, 1.2)).length() < 1e-6);
        assert!((small.accel_delta - Vec3::splat(0.2)).length() < 1e-6);
        assert!((small.volume_extent - Vec3::new(0.0, 0.08, 0.0)).length() < 1e-6);
        assert_eq!((small.rate, small.life), (50.0, 0.5), "the rate and the life are the group's");
        assert_eq!(spec.size[3], 1.5, "the source is untouched");
    }

    #[test]
    fn the_nitrous_sprites_are_narrower_and_slower_and_everything_else_is_kept() {
        let spec = EmitterSpec {
            size: [0.0, 0.4, 1.0, 1.5],
            speed: 16.0,
            velocity_start: Vec3::new(1.0, 2.0, 3.0),
            velocity_delta: Vec3::splat(0.5),
            rate: 50.0,
            life: 0.5,
            ..EmitterSpec::default()
        };
        let small = compact_nitrous(&spec);
        assert!(small.size.iter().zip([0.0, 0.12, 0.3, 0.45]).all(|(a, b)| (a - b).abs() < 1e-6));
        assert!((small.speed - 1.28).abs() < 1e-5);
        assert!((small.velocity_start - Vec3::new(0.08, 0.16, 0.24)).length() < 1e-6);
        assert_eq!((small.rate, small.life), (50.0, 0.5));
        assert_eq!(spec.size[3], 1.5, "the source is untouched");
    }
}
